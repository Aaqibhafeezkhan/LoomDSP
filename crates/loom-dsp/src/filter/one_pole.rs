//! First-order filter with trapezoidal integration.
//!
//! Source: V. Zavalishin, "The Art of VA Filter Design".
//!
//! ```
//! use loom_dsp::filter::one_pole::{Coefficients, OnePole};
//!
//! let coefficients = Coefficients::new(1_000.0, 48_000.0);
//! let mut filter = OnePole::new();
//! let outputs = filter.tick(0.5, coefficients);
//! assert_eq!(outputs.low + outputs.high, 0.5);
//! ```

use super::prewarp;

/// The filter's setting for one sample. Computing it costs a tangent, so a
/// caller with a fixed cutoff computes it once.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coefficients {
    g: f32,
}

impl Coefficients {
    /// Computes the setting for a cutoff, which must be above 0 and below half
    /// of `sample_rate`.
    #[inline]
    pub fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        Self {
            g: prewarp(cutoff_hz, sample_rate),
        }
    }
}

/// Both responses for one sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outputs {
    /// Low-pass.
    pub low: f32,
    /// High-pass: the input minus the low-pass.
    pub high: f32,
}

/// The running state of one filter.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OnePole {
    state: f32,
}

impl OnePole {
    /// Creates a filter with cleared state.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears the filter's memory.
    #[inline]
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Processes one sample. `input` must be finite: NaN or infinity would
    /// stay in the state until [`OnePole::reset`].
    #[inline]
    pub fn tick(&mut self, input: f32, coefficients: Coefficients) -> Outputs {
        debug_assert!(input.is_finite(), "input {input}");
        // Not `g / (1 + g)` stored ahead: this order of operations is part of
        // the output's exact bits.
        let Coefficients { g } = coefficients;
        let v = (input - self.state) * g / (1.0 + g);
        let low = v + self.state;
        self.state = low + v;
        Outputs { low, high: input - low }
    }
}

#[cfg(test)]
mod tests;
