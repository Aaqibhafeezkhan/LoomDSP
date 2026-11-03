use super::*;
use crate::testing::RATE;

/// Straight stages of 10, 20, and 30 ms at a sustain of 0.5.
const ADSR: Settings = Settings {
    attack: 0.01,
    decay: 0.02,
    sustain: 0.5,
    release: 0.03,
    ad: false,
    power: 1.0,
};

/// Frames in `seconds`.
fn frames(seconds: f32) -> usize {
    (seconds * RATE) as usize
}

/// Each stage reaches its level at its time, within one frame.
#[test]
fn stages_take_their_set_times() {
    let mut env = Envelope::new(RATE);
    for _ in 0..frames(ADSR.attack) {
        env.tick(true, ADSR);
    }
    assert!((env.level() - 1.0).abs() < 1e-3);
    for _ in 0..frames(ADSR.decay) {
        env.tick(true, ADSR);
    }
    assert!((env.level() - ADSR.sustain).abs() < 1e-3);
    for _ in 0..frames(0.5) {
        assert_eq!(env.tick(true, ADSR), ADSR.sustain);
    }
    for _ in 0..frames(ADSR.release) {
        env.tick(false, ADSR);
    }
    assert!(env.level().abs() < 1e-3);
    env.tick(false, ADSR);
    assert_eq!(env.level(), 0.0);
}

/// An AD envelope runs to the end, even after the gate falls.
#[test]
fn ad_ignores_the_falling_gate() {
    let settings = Settings { ad: true, ..ADSR };
    let mut env = Envelope::new(RATE);
    env.tick(true, settings);
    let mut peak: f32 = 0.0;
    for _ in 0..frames(settings.attack + settings.decay) + 1 {
        peak = peak.max(env.tick(false, settings));
    }
    assert!((peak - 1.0).abs() < 1e-3);
    assert_eq!(env.tick(false, settings), 0.0);
}

/// A retrigger starts from the current level, so the output does not jump.
#[test]
fn retrigger_does_not_jump() {
    let mut env = Envelope::new(RATE);
    for _ in 0..frames(0.1) {
        env.tick(true, ADSR);
    }
    for _ in 0..frames(ADSR.release / 2.0) {
        env.tick(false, ADSR);
    }
    let before = env.level();
    let after = env.tick(true, ADSR);
    assert!(after > before && after - before < 0.02, "{before} to {after}");
}

/// The level never leaves 0 to 1, at any bend and with fast gates.
#[test]
fn level_stays_in_range() {
    for curve in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        let settings = Settings {
            power: power(curve),
            ..ADSR
        };
        let mut env = Envelope::new(RATE);
        for i in 0..20_000 {
            let level = env.tick(i % 777 < 300, settings);
            assert!((0.0..=1.0).contains(&level), "curve {curve}, frame {i}: {level}");
        }
    }
}

/// The bend meets both ends of a stage at any power, and a power above 1
/// starts fast with a slow tail.
#[test]
fn fall_shape() {
    for k in [0.125, 1.0, 8.0] {
        assert_eq!(fall(0.0, k), 1.0);
        assert_eq!(fall(1.0, k), 0.0);
    }
    assert!(fall(0.5, power(1.0)) < 0.5);
    assert!(fall(0.5, power(-1.0)) > 0.5);
    assert_eq!(power(0.0), 1.0);
}

/// `reset` returns to idle, so a held gate counts as a new rising edge.
#[test]
fn reset_returns_to_idle() {
    let mut env = Envelope::new(RATE);
    for _ in 0..100 {
        env.tick(true, ADSR);
    }
    env.reset();
    assert_eq!(env.level(), 0.0);
    assert!(env.tick(true, ADSR) > 0.0);
}

/// A reset envelope equals a new one, whatever it was doing, and in AD too.
#[test]
fn reset_equals_new() {
    let ad = Settings { ad: true, ..ADSR };
    for settings in [ADSR, ad] {
        let mut env = Envelope::new(RATE);
        // Long enough to be in the decay, which remembers its mode.
        for _ in 0..frames(settings.attack) + 2 {
            env.tick(true, settings);
        }
        env.reset();
        assert_eq!(env, Envelope::new(RATE));
    }
}
