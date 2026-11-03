use super::*;
use crate::testing::{RATE, tone_gain};
use std::f64::consts::TAU;

/// The gain matches the filter's transfer function,
/// `a (1 - z^-1) / (1 - a z^-1)`.
#[test]
fn response_matches_the_theory() {
    let a = f64::from((-std::f32::consts::TAU * 5.0 / RATE).exp());
    for hz in [5.0, 20.0, 100.0, 1_000.0, 10_000.0] {
        let mut filter = DcBlocker::new(5.0, RATE);
        let measured = tone_gain(hz, 1.0, |x| filter.tick(x));
        let w = TAU * hz / f64::from(RATE);
        let top = a * f64::hypot(1.0 - w.cos(), w.sin());
        let bottom = f64::hypot(1.0 - a * w.cos(), a * w.sin());
        let expected = top / bottom;
        assert!(
            (measured - expected).abs() < 2e-3,
            "{hz} Hz: {measured} against {expected}"
        );
    }
}

/// A held offset falls all the way to 0, not just near it, so it can never
/// become a denormal.
#[test]
fn a_held_offset_reaches_exactly_zero() {
    let mut filter = DcBlocker::new(5.0, RATE);
    let mut output = 1.0;
    for _ in 0..RATE as usize * 4 {
        output = filter.tick(0.25);
    }
    assert_eq!(output, 0.0);
}

/// `reset` clears the memory and keeps the setting.
#[test]
fn reset_clears_the_state() {
    let mut filter = DcBlocker::new(5.0, RATE);
    filter.tick(1.0);
    filter.reset();
    assert_eq!(filter, DcBlocker::new(5.0, RATE));
}
