use super::*;
use crate::testing::{RATE, tone_gain};
use std::f64::consts::TAU;

/// A whole-number delay returns the input exactly that many samples later.
#[test]
fn whole_number_delay_is_exact() {
    let mut line = DelayLine::new(0.01, RATE);
    let input: Vec<f32> = (0..200).map(|i| (i as f32 * 0.7).sin()).collect();
    for (i, &x) in input.iter().enumerate() {
        let y = line.tick(x, 13.0);
        let expected = if i >= 13 { input[i - 13] } else { 0.0 };
        assert_eq!(y, expected, "frame {i}");
    }
}

/// A fractional read of a slow sine is close to the true delayed value.
#[test]
fn fractional_read_matches_a_slow_sine() {
    let hz = 440.0;
    let delay = 37.4;
    let step = TAU * hz / f64::from(RATE);
    let mut line = DelayLine::new(0.01, RATE);
    for i in 0..2_000 {
        let y = line.tick((step * f64::from(i)).sin() as f32, delay);
        if i > 100 {
            let expected = (step * (f64::from(i) - delay)).sin();
            assert!((f64::from(y) - expected).abs() < 1e-4, "frame {i}");
        }
    }
}

/// A Schroeder all-pass passes every frequency at unity gain, at any feedback.
#[test]
fn allpass_has_unity_gain() {
    for gain in [-0.7, 0.3, 0.7] {
        for hz in [100.0, 1_000.0, 5_000.0, 15_000.0] {
            let mut line = Allpass::new(0.01, RATE);
            let measured = tone_gain(hz, 1.0, |x| line.tick(x, 37.0, gain));
            assert!((measured - 1.0).abs() < 2e-3, "gain {gain}, {hz} Hz: {measured}");
        }
    }
}

/// With no feedback, the all-pass is the same as a plain delay.
#[test]
fn allpass_with_no_gain_is_a_delay() {
    let mut plain = DelayLine::new(0.01, RATE);
    let mut allpass = Allpass::new(0.01, RATE);
    for i in 0..500 {
        let x = (i as f32 * 0.1).sin();
        assert_eq!(plain.tick(x, 20.5), allpass.tick(x, 20.5, 0.0));
    }
}

/// A delay out of range, or NaN, is clamped, so it never reads outside the
/// buffer.
#[test]
fn bad_delays_are_clamped() {
    let mut line = DelayLine::new(0.001, RATE);
    let mut allpass = Allpass::new(0.001, RATE);
    for raw in [f64::NAN, -5.0, 0.0, 1e12, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(line.tick(0.5, raw).is_finite(), "{raw}");
        assert!(allpass.tick(0.5, raw, 0.5).is_finite(), "{raw}");
    }
}

/// A bad input cannot circulate in the all-pass.
#[test]
fn a_bad_input_does_not_stay_in_the_allpass() {
    let mut line = Allpass::new(0.001, RATE);
    line.tick(f32::NAN, 10.0, 0.5);
    for _ in 0..100 {
        assert!(line.tick(0.0, 10.0, 0.5).is_finite());
    }
}

/// `reset` empties the line, and `copy_from` takes another line's state unless
/// the lengths differ.
#[test]
fn reset_and_copy() {
    let mut line = DelayLine::new(0.01, RATE);
    for i in 0..100 {
        line.tick(i as f32, 5.0);
    }
    let mut copy = DelayLine::new(0.01, RATE);
    assert!(copy.copy_from(&line));
    assert_eq!(copy, line);
    assert!(!DelayLine::new(0.02, RATE).copy_from(&line));
    line.reset();
    assert_eq!(line, DelayLine::new(0.01, RATE));
}

/// The shortest delay is one sample: the output is the input before this one.
#[test]
fn a_delay_of_one_sample_gives_the_previous_input() {
    let mut line = DelayLine::new(0.01, RATE);
    let mut previous = 0.0;
    for i in 1..50 {
        let x = i as f32;
        assert_eq!(line.tick(x, 1.0), previous, "frame {i}");
        previous = x;
    }
}
