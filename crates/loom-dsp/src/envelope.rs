//! An ADSR and AD envelope, with a bend on every stage.
//!
//! Each stage runs for its set time, whatever level it starts from, so a
//! retrigger does not click and still takes the full attack time. Built from
//! first principles.
//!
//! ```
//! use loom_dsp::envelope::{Envelope, Settings, power};
//!
//! let settings = Settings {
//!     attack: 0.01,
//!     decay: 0.1,
//!     sustain: 0.5,
//!     release: 0.2,
//!     ad: false,
//!     power: power(0.0),
//! };
//! let mut envelope = Envelope::new(48_000.0);
//! assert!(envelope.tick(true, settings) > 0.0);
//! ```

/// The shaping power for `curve` from -1 to 1: `2^(3 × curve)`, from 1/8
/// to 8.
#[inline]
pub fn power(curve: f32) -> f32 {
    (3.0 * curve.clamp(-1.0, 1.0)).exp2()
}

/// How far a falling stage has gone: `(1 - x)^k`, from 1 at `x` = 0 to 0
/// at `x` = 1. A rising stage uses `1 - fall(x, k)`.
#[inline]
pub fn fall(x: f32, k: f32) -> f32 {
    let rest = (1.0 - x).max(0.0);
    // Skip the `powf` for the straight line, the default.
    if k == 1.0 { rest } else { rest.powf(k) }
}

/// Where the envelope is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    /// At 0, waiting for a rising gate.
    Idle,
    /// Rising from the level it started at to 1.
    Attack,
    /// Falling from 1 to the sustain level (ADSR) or to 0 (AD).
    Decay,
    /// Holding the sustain level while the gate is high (ADSR only).
    Sustain,
    /// Falling from the level it started at to 0 (ADSR only).
    Release,
}

/// The envelope's settings for one sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// Attack time in seconds.
    pub attack: f32,
    /// Decay time in seconds.
    pub decay: f32,
    /// Sustain level, 0 to 1.
    pub sustain: f32,
    /// Release time in seconds.
    pub release: f32,
    /// True for AD: no sustain or release, so it runs to the end once started.
    pub ad: bool,
    /// The bend, from [`power`].
    pub power: f32,
}

/// The running state of one envelope.
#[derive(Debug, Clone, PartialEq)]
pub struct Envelope {
    sample_rate: f64,
    stage: Stage,
    /// The output level.
    level: f32,
    /// The level when the current stage started.
    start: f32,
    /// Progress through the current stage, from 0 to 1. Kept in `f64`, so
    /// a 10 s stage at a high sample rate still ends on time.
    progress: f64,
    gate_was_high: bool,
    /// Whether the current decay falls to 0 (AD) rather than to the
    /// sustain level, so a mode change mid-decay can restart it.
    decay_ad: bool,
}

impl Envelope {
    /// Creates an idle envelope.
    #[inline]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate: f64::from(sample_rate),
            stage: Stage::Idle,
            level: 0.0,
            start: 0.0,
            progress: 0.0,
            gate_was_high: false,
            decay_ad: false,
        }
    }

    /// Back to idle at 0.
    #[inline]
    pub fn reset(&mut self) {
        self.stage = Stage::Idle;
        self.level = 0.0;
        self.start = 0.0;
        self.progress = 0.0;
        self.gate_was_high = false;
        self.decay_ad = false;
    }

    /// The output level.
    #[inline]
    pub fn level(&self) -> f32 {
        self.level
    }

    /// Starts `stage` from the current level.
    #[inline]
    fn enter(&mut self, stage: Stage) {
        self.stage = stage;
        self.start = self.level;
        self.progress = 0.0;
    }

    /// Moves the progress on by one frame of a stage that lasts `seconds`,
    /// and returns the new progress, at most 1.
    #[inline]
    fn advance(&mut self, seconds: f32) -> f32 {
        let frames = (f64::from(seconds) * self.sample_rate).max(1.0);
        self.progress = (self.progress + 1.0 / frames).min(1.0);
        self.progress as f32
    }

    /// Processes one sample and returns the level.
    #[inline]
    pub fn tick(&mut self, gate: bool, settings: Settings) -> f32 {
        let Settings {
            attack,
            decay,
            sustain,
            release,
            ad,
            power: k,
        } = settings;
        debug_assert!((0.0..=1.0).contains(&sustain), "sustain {sustain}");
        debug_assert!(
            attack >= 0.0 && decay >= 0.0 && release >= 0.0,
            "times {attack}, {decay}, {release}"
        );
        if gate && !self.gate_was_high {
            // Retrigger from the current level: no click. The attack
            // still takes the full attack time.
            self.enter(Stage::Attack);
        } else if !gate && self.gate_was_high && !ad && self.stage != Stage::Idle {
            self.enter(Stage::Release);
        }
        self.gate_was_high = gate;
        // AD has no sustain. A switch to AD while holding falls to 0.
        if ad && self.stage == Stage::Sustain {
            self.enter(Stage::Decay);
            self.decay_ad = true;
        }
        // A mode change mid-decay moves the decay's end. Restart the
        // stage from the current level, so the output does not jump.
        // With the gate already low, ADSR goes on to the release.
        if self.stage == Stage::Decay && self.decay_ad != ad {
            self.enter(if ad || gate { Stage::Decay } else { Stage::Release });
            self.decay_ad = ad;
        }

        match self.stage {
            Stage::Idle => self.level = 0.0,
            Stage::Attack => {
                let x = self.advance(attack);
                self.level = self.start + (1.0 - self.start) * (1.0 - fall(x, k));
                if x >= 1.0 {
                    self.level = 1.0;
                    self.enter(Stage::Decay);
                    self.decay_ad = ad;
                }
            }
            Stage::Decay => {
                let x = self.advance(decay);
                let end = if ad { 0.0 } else { sustain };
                self.level = end + (self.start - end) * fall(x, k);
                if x >= 1.0 {
                    self.level = end;
                    if ad {
                        self.stage = Stage::Idle;
                    } else if gate {
                        self.stage = Stage::Sustain;
                    } else {
                        // The gate fell while the mode was AD, so no
                        // falling edge will come: release from here.
                        self.enter(Stage::Release);
                    }
                }
            }
            Stage::Sustain => self.level = sustain,
            Stage::Release => {
                let x = self.advance(release);
                self.level = self.start * fall(x, k);
                if x >= 1.0 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }
        self.level
    }
}

#[cfg(test)]
mod tests;
