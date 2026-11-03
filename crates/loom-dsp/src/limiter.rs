//! A stereo-linked peak limiter. One gain for both channels, so the stereo
//! image does not move while it acts.
//!
//! Without lookahead the gain falls in the sample that goes over. With
//! lookahead the audio is delayed by `lookahead` samples, and the gain is the
//! average, over the last `lookahead + 1` samples, of the lowest gain needed
//! anywhere in the window ahead. Every value in that average is at or under
//! the gain the delayed sample needs, so the average is too, and it changes in
//! straight lines rather than steps.
//!
//! Background: D. Giannoulis, M. Massberg, and J. D. Reiss, "Digital Dynamic Range
//! Compressor Design: A Tutorial and Analysis", JAES 60, 2012. The sliding
//! minimum is the monotonic-queue method: D. Lemire, "Streaming
//! Maximum-Minimum Filter Using No More than Three Comparisons per Element",
//! Nordic Journal of Computing, 2006.
//!
//! ```
//! use loom_dsp::limiter::{Limiter, Mode, Settings};
//!
//! let mut limiter = Limiter::new(Settings {
//!     ceiling: 0.9,
//!     lookahead: 96,
//!     release_seconds: 0.1,
//!     sample_rate: 48_000.0,
//! });
//! limiter.set_mode(Mode::Direct);
//! assert_eq!(limiter.tick(2.0, 1.0).0, 0.9);
//! ```

use std::collections::VecDeque;

/// What sets a [`Limiter`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// The highest peak allowed out.
    pub ceiling: f32,
    /// How far ahead [`Mode::Lookahead`] looks, in samples.
    pub lookahead: usize,
    /// The time constant of recovery.
    pub release_seconds: f32,
    /// The sample rate in hertz.
    pub sample_rate: f32,
}

/// How a [`Limiter`] acts on a peak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The gain falls in the sample that goes over. No latency.
    Direct,
    /// The audio is delayed, and the gain falls before the peak arrives.
    Lookahead,
}

/// The running state of one stereo limiter. All memory is allocated in
/// [`Limiter::new`], so `tick`, `reset`, and `copy_from` do not allocate. It is
/// not `Clone`: a cloned `VecDeque` has room only for its current length, so the
/// copy's next `tick` would allocate.
#[derive(Debug)]
pub struct Limiter {
    /// The highest peak allowed out.
    ceiling: f32,
    /// Gain applied to the last sample, after release.
    gain: f32,
    /// Per-sample factor that moves the gain toward 1.
    release: f32,
    lookahead: usize,
    mode: Mode,
    /// Delayed audio, `lookahead` frames.
    delay: VecDeque<(f32, f32)>,
    /// Sliding minimum of the needed gain: (sample number, gain), with gains
    /// rising from front to back.
    minimum: VecDeque<(u64, f32)>,
    /// The last `lookahead + 1` window minimums, and their sum.
    window: VecDeque<f32>,
    window_sum: f64,
    /// Samples seen since the last reset.
    count: u64,
}

impl Limiter {
    /// Creates a limiter in [`Mode::Direct`].
    #[inline]
    pub fn new(settings: Settings) -> Self {
        let Settings {
            ceiling,
            lookahead,
            release_seconds,
            sample_rate,
        } = settings;
        let mut limiter = Self {
            ceiling,
            gain: 1.0,
            release: (-1.0 / (release_seconds * sample_rate)).exp(),
            lookahead,
            mode: Mode::Direct,
            delay: VecDeque::with_capacity(lookahead + 1),
            minimum: VecDeque::with_capacity(lookahead + 2),
            window: VecDeque::with_capacity(lookahead + 2),
            window_sum: 0.0,
            count: 0,
        };
        limiter.reset();
        limiter
    }

    /// Changes the mode and resets, because the delay line runs only in
    /// [`Mode::Lookahead`], so what it holds is out of date after a change.
    #[inline]
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.reset();
    }

    /// Takes the whole state of `other`, with no allocation. False, with nothing
    /// changed, when the two have different lookaheads, and so different room.
    #[inline]
    pub fn copy_from(&mut self, other: &Self) -> bool {
        if self.lookahead != other.lookahead {
            return false;
        }
        self.ceiling = other.ceiling;
        self.gain = other.gain;
        self.release = other.release;
        self.mode = other.mode;
        self.delay.clear();
        self.delay.extend(other.delay.iter().copied());
        self.minimum.clear();
        self.minimum.extend(other.minimum.iter().copied());
        self.window.clear();
        self.window.extend(other.window.iter().copied());
        self.window_sum = other.window_sum;
        self.count = other.count;
        true
    }

    /// The lookahead in samples, which is also the latency in
    /// [`Mode::Lookahead`].
    #[inline]
    pub fn lookahead(&self) -> usize {
        self.lookahead
    }

    /// Back to unity gain and silence in the delay line. Keeps capacity, so
    /// it never allocates.
    #[inline]
    pub fn reset(&mut self) {
        self.gain = 1.0;
        self.delay.clear();
        self.delay.extend(std::iter::repeat_n((0.0, 0.0), self.lookahead));
        self.minimum.clear();
        self.window.clear();
        self.window.extend(std::iter::repeat_n(1.0, self.lookahead + 1));
        self.window_sum = (self.lookahead + 1) as f64;
        self.count = 0;
    }

    /// Processes one stereo sample. Both inputs must be finite.
    #[inline]
    pub fn tick(&mut self, left: f32, right: f32) -> (f32, f32) {
        debug_assert!(left.is_finite() && right.is_finite(), "input {left}, {right}");
        let peak = left.abs().max(right.abs());
        let needed = if peak > self.ceiling { self.ceiling / peak } else { 1.0 };
        let (target, left, right) = match self.mode {
            Mode::Lookahead => self.look_ahead(needed, left, right),
            Mode::Direct => (needed, left, right),
        };
        let released = 1.0 - (1.0 - self.gain) * self.release;
        self.gain = target.min(released);
        // The clamp only trims rounding: `x * (ceiling / x)` can land one
        // step above the ceiling.
        (
            (left * self.gain).clamp(-self.ceiling, self.ceiling),
            (right * self.gain).clamp(-self.ceiling, self.ceiling),
        )
    }

    /// Pushes one sample in, and returns the gain for the sample coming out
    /// of the delay line, with that sample.
    #[inline]
    fn look_ahead(&mut self, needed: f32, left: f32, right: f32) -> (f32, f32, f32) {
        let n = self.count;
        self.count += 1;
        while self.minimum.back().is_some_and(|&(_, gain)| gain >= needed) {
            self.minimum.pop_back();
        }
        self.minimum.push_back((n, needed));
        let oldest = n.saturating_sub(self.lookahead as u64);
        while self.minimum.front().is_some_and(|&(i, _)| i < oldest) {
            self.minimum.pop_front();
        }
        let lowest = self.minimum.front().map_or(1.0, |&(_, gain)| gain);

        self.window.push_back(lowest);
        self.window_sum += f64::from(lowest);
        if let Some(old) = self.window.pop_front() {
            self.window_sum -= f64::from(old);
        }
        let average = (self.window_sum / (self.lookahead + 1) as f64) as f32;

        self.delay.push_back((left, right));
        let (left, right) = self.delay.pop_front().unwrap_or((0.0, 0.0));
        (average.min(1.0), left, right)
    }
}

#[cfg(test)]
mod tests;
