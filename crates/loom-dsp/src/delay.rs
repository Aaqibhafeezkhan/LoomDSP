//! A delay line with a 4-point Catmull-Rom fractional read, and a Schroeder
//! all-pass built on it.
//!
//! Background: E. Catmull and R. Rom, "A Class of Local Interpolating Splines",
//! 1974. M. R. Schroeder and B. F. Logan, "'Colorless' Artificial
//! Reverberation", JAES 9, 1961.
//!
//! ```
//! use loom_dsp::delay::DelayLine;
//!
//! let mut line = DelayLine::new(0.01, 48_000.0);
//! line.tick(1.0, 2.0);
//! line.tick(0.0, 2.0);
//! // The first sample comes back two samples later.
//! assert_eq!(line.tick(0.0, 2.0), 1.0);
//! ```

/// A circular buffer. Memory is allocated only in [`DelayLine::new`]. To take
/// another line's state on the audio thread, use [`DelayLine::copy_from`]:
/// `clone` and `clone_from` allocate.
#[derive(Debug, Clone, PartialEq)]
pub struct DelayLine {
    samples: Box<[f32]>,
    write: usize,
    max_samples: usize,
}

impl DelayLine {
    /// Allocates a zeroed buffer for `max_seconds` at `sample_rate`, plus four
    /// guard samples for the 4-point cubic stencil.
    #[inline]
    pub fn new(max_seconds: f32, sample_rate: f32) -> Self {
        let max_samples = (f64::from(max_seconds) * f64::from(sample_rate)).ceil().max(2.0) as usize;
        // Four extra frames so `k + 2` at `k == max_samples` stays in bounds
        // without overlapping the current write slot.
        Self {
            samples: vec![0.0; max_samples + 4].into_boxed_slice(),
            write: 0,
            max_samples,
        }
    }

    /// Copies `other` into this buffer, with no allocation. False when the
    /// two have different lengths.
    #[inline]
    pub fn copy_from(&mut self, other: &Self) -> bool {
        if self.samples.len() != other.samples.len() {
            return false;
        }
        self.samples.copy_from_slice(&other.samples);
        self.write = other.write;
        self.max_samples = other.max_samples;
        true
    }

    /// Zeroes all buffered samples and resets the write index.
    #[inline]
    pub fn reset(&mut self) {
        self.samples.fill(0.0);
        self.write = 0;
    }

    /// Clamps `raw_samples` to `[min_samples, max_samples]`, mapping `NaN` to
    /// `min_samples`. This guard stays here for any caller, because a delay
    /// out of range would read outside the buffer.
    #[inline]
    fn clamp_delay(&self, raw_samples: f64, min_samples: f64) -> f64 {
        if raw_samples.is_nan() {
            min_samples
        } else {
            raw_samples.clamp(min_samples, self.max_samples as f64)
        }
    }

    /// Sample `age` frames behind `self.write`. `age == 0` is the current
    /// write slot; `age == 1` is the sample written on the previous frame.
    #[inline]
    fn at(&self, age: usize) -> f32 {
        let index = if self.write >= age {
            self.write - age
        } else {
            self.samples.len() + self.write - age
        };
        self.samples[index]
    }

    /// 4-point Catmull-Rom (cubic Hermite) read at `delay_samples` behind
    /// `self.write`. Caller guarantees `1.0 <= delay_samples <= max_samples`.
    #[inline]
    fn read_catmull_rom(&self, delay_samples: f64) -> f32 {
        let k = delay_samples.floor() as usize;
        let t = (delay_samples - k as f64) as f32;
        let y_m1 = self.at(k - 1);
        let y0 = self.at(k);
        let y1 = self.at(k + 1);
        let y2 = self.at(k + 2);
        let c0 = y0;
        let c1 = 0.5 * (y1 - y_m1);
        let c2 = y_m1 - 2.5 * y0 + 2.0 * y1 - 0.5 * y2;
        let c3 = 0.5 * (y2 - y_m1) + 1.5 * (y0 - y1);
        ((c3 * t + c2) * t + c1) * t + c0
    }

    /// Steps the write index forward by one sample, wrapping at the end of the
    /// buffer.
    #[inline]
    fn advance(&mut self) {
        self.write += 1;
        if self.write == self.samples.len() {
            self.write = 0;
        }
    }

    /// One sample: writes `input` at age 0, reads at `raw_samples` clamped to
    /// `[1.0, max_samples]`, and advances. `input` must be finite, or a bad
    /// sample comes out again one delay time later.
    #[inline]
    pub fn tick(&mut self, input: f32, raw_samples: f64) -> f32 {
        debug_assert!(input.is_finite(), "input {input}");
        let delay_samples = self.clamp_delay(raw_samples, 1.0);
        self.samples[self.write] = input;
        let out = self.read_catmull_rom(delay_samples);
        self.advance();
        out
    }
}

/// A Schroeder all-pass: a flat magnitude response, with the phase changed. It
/// is its own type because it writes something else into its memory than a
/// delay does, the input plus the feedback. Mixing the two on one line would be
/// silently wrong.
///
/// As with [`DelayLine`], use [`Allpass::copy_from`] on the audio thread.
#[derive(Debug, Clone, PartialEq)]
pub struct Allpass {
    line: DelayLine,
}

impl Allpass {
    /// Allocates memory for `max_seconds` at `sample_rate`.
    #[inline]
    pub fn new(max_seconds: f32, sample_rate: f32) -> Self {
        Self {
            line: DelayLine::new(max_seconds, sample_rate),
        }
    }

    /// Copies `other` into this one, with no allocation. False when the two
    /// have different lengths.
    #[inline]
    pub fn copy_from(&mut self, other: &Self) -> bool {
        self.line.copy_from(&other.line)
    }

    /// Clears the memory.
    #[inline]
    pub fn reset(&mut self) {
        self.line.reset();
    }

    /// One sample: reads at `raw_samples` clamped to `[2.0, max_samples]` (so
    /// `k - 1 >= 1` only reads past `w` values), flushes denormals on the
    /// feedback product, sanitizes non-finite state so a bad sample cannot
    /// circulate forever, writes `w`, and advances.
    ///
    /// The check on `w` is here because it sits inside the feedback path,
    /// where a caller cannot reach it.
    #[inline]
    pub fn tick(&mut self, input: f32, raw_samples: f64, gain: f32) -> f32 {
        let line = &mut self.line;
        let delay_samples = line.clamp_delay(raw_samples, 2.0);
        let delayed = line.read_catmull_rom(delay_samples);
        let mut feedback = gain * delayed;
        if feedback.abs() < 1e-20 {
            feedback = 0.0;
        }
        let mut w = input + feedback;
        if !w.is_finite() {
            w = 0.0;
        }
        let out = delayed - gain * w;
        line.samples[line.write] = w;
        line.advance();
        out
    }
}

#[cfg(test)]
mod tests;
