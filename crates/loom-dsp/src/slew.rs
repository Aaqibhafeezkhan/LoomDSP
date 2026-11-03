//! A slew limiter: limits how fast a signal can rise or fall, at a constant
//! rate. Built from first principles.
//!
//! ```
//! use loom_dsp::slew::Slew;
//!
//! let mut slew = Slew::new(48_000.0);
//! // The first sample passes straight through, then the output moves slowly.
//! assert_eq!(slew.tick(0.0, 1.0, 1.0), 0.0);
//! let moved = slew.tick(1.0, 1.0, 1.0);
//! assert!(moved > 0.0 && moved < 0.001);
//! ```

/// The running state of one slew limiter. The output is kept in `f64`, so a
/// very slow ramp does not stall when its step rounds away.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slew {
    sample_rate: f64,
    out: f64,
    /// True until the first frame, which sets the output with no ramp.
    fresh: bool,
}

impl Slew {
    /// Creates a slew limiter whose first sample passes straight through.
    #[inline]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate: f64::from(sample_rate),
            out: 0.0,
            fresh: true,
        }
    }

    /// Back to the state of a new limiter.
    #[inline]
    pub fn reset(&mut self) {
        self.out = 0.0;
        self.fresh = true;
    }

    /// The output.
    #[inline]
    pub fn value(&self) -> f32 {
        self.out as f32
    }

    /// Moves toward `target`. A change of 1.0 takes `rise` or `fall` seconds.
    /// `target` must be finite, and `rise` and `fall` must be above 0.
    #[inline]
    pub fn tick(&mut self, target: f32, rise: f32, fall: f32) -> f32 {
        debug_assert!(target.is_finite(), "target {target}");
        debug_assert!(rise > 0.0 && fall > 0.0, "rise {rise}, fall {fall}");
        let target = f64::from(target);
        if self.fresh {
            // A new slew must not sweep up from 0.
            self.out = target;
            self.fresh = false;
        } else if target > self.out {
            self.out = (self.out + 1.0 / (f64::from(rise) * self.sample_rate)).min(target);
        } else {
            self.out = (self.out - 1.0 / (f64::from(fall) * self.sample_rate)).max(target);
        }
        self.value()
    }
}

#[cfg(test)]
mod tests;
