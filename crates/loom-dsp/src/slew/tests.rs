use super::*;
use crate::testing::RATE;

/// The first sample passes straight through.
#[test]
fn first_sample_passes_through() {
    let mut slew = Slew::new(RATE);
    assert_eq!(slew.tick(0.75, 1.0, 1.0), 0.75);
}

/// A change of 1.0 takes the set time, up and down at their own rates.
#[test]
fn rise_and_fall_take_their_times() {
    let (rise, fall) = (0.01, 0.02);
    let mut slew = Slew::new(RATE);
    slew.tick(0.0, rise, fall);
    for _ in 0..(rise * RATE) as usize - 1 {
        assert!(slew.tick(1.0, rise, fall) < 1.0);
    }
    assert!((slew.tick(1.0, rise, fall) - 1.0).abs() < 1e-3);
    slew.tick(1.0, rise, fall);
    assert_eq!(slew.value(), 1.0);
    for _ in 0..(fall * RATE) as usize + 1 {
        slew.tick(0.0, rise, fall);
    }
    assert_eq!(slew.value(), 0.0);
}

/// It stops at the target, with no overshoot.
#[test]
fn stops_at_the_target() {
    let mut slew = Slew::new(RATE);
    slew.tick(0.0, 0.001, 0.001);
    for _ in 0..1_000 {
        assert!(slew.tick(0.3, 0.001, 0.001) <= 0.3);
    }
    assert_eq!(slew.value(), 0.3);
}

/// A very slow ramp keeps moving, because the state is in `f64`: its step
/// would round away in `f32`.
#[test]
fn a_very_slow_ramp_does_not_stall() {
    let mut slew = Slew::new(RATE);
    slew.tick(1.0, 1_000.0, 1_000.0);
    for _ in 0..RATE as usize * 10 {
        slew.tick(2.0, 1_000.0, 1_000.0);
    }
    assert!((slew.value() - 1.01).abs() < 1e-4, "{}", slew.value());
}

/// `reset` makes the next sample pass through again.
#[test]
fn reset_passes_through_again() {
    let mut slew = Slew::new(RATE);
    slew.tick(0.5, 1.0, 1.0);
    slew.reset();
    assert_eq!(slew.tick(-0.5, 1.0, 1.0), -0.5);
}
