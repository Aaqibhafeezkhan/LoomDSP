use super::*;
use crate::testing::{RATE, tone_gain, warped};

/// Both outputs match the analogue response under the bilinear transform,
/// and the cutoff is exactly the -3 dB point.
#[test]
fn response_matches_the_theory() {
    for cutoff in [100.0, 1_000.0, 10_000.0] {
        let coefficients = Coefficients::new(cutoff, RATE);
        for ratio in [0.1, 0.5, 1.0, 2.0] {
            let hz = f64::from(cutoff) * ratio;
            let w = warped(hz, f64::from(cutoff));
            let mut low = OnePole::new();
            let measured = tone_gain(hz, 1.0, |x| low.tick(x, coefficients).low);
            let expected = 1.0 / (1.0 + w * w).sqrt();
            assert!(
                (measured - expected).abs() < 2e-3,
                "low at {hz}: {measured} against {expected}"
            );
            let mut high = OnePole::new();
            let measured = tone_gain(hz, 1.0, |x| high.tick(x, coefficients).high);
            let expected = w / (1.0 + w * w).sqrt();
            assert!(
                (measured - expected).abs() < 2e-3,
                "high at {hz}: {measured} against {expected}"
            );
        }
    }
}

/// The two outputs always sum to the input.
#[test]
fn outputs_sum_to_the_input() {
    let coefficients = Coefficients::new(700.0, RATE);
    let mut filter = OnePole::new();
    for i in 0..1_000 {
        let input = (i as f32 * 0.3).sin();
        let o = filter.tick(input, coefficients);
        assert!((o.low + o.high - input).abs() < 1e-5);
    }
}

/// `reset` clears the memory.
#[test]
fn reset_clears_the_state() {
    let mut filter = OnePole::new();
    filter.tick(1.0, Coefficients::new(700.0, RATE));
    assert_ne!(filter, OnePole::new());
    filter.reset();
    assert_eq!(filter, OnePole::new());
}
