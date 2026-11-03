use super::*;
use crate::testing::RATE;

/// After ten minutes the phase is still where exact arithmetic puts it, so the
/// pitch does not drift.
#[test]
fn the_rate_stays_accurate_over_a_long_run() {
    let step = Phasor::step_for(440.0, RATE);
    let frames: u64 = 48_000 * 600;
    let mut phasor = Phasor::new();
    for _ in 0..frames {
        phasor.advance(step);
    }
    // 440 cycles a second for a whole number of seconds ends where it began.
    let error = phasor.phase().min(1.0 - phasor.phase());
    assert!(error < 1e-6, "{error}");
}

/// The step is limited on both sides of Nyquist, and an overflow to infinity
/// gives the limit, not NaN.
#[test]
fn the_step_is_limited() {
    assert_eq!(Phasor::step_for(100_000.0, RATE), MAX_STEP);
    assert_eq!(Phasor::step_for(-100_000.0, RATE), -MAX_STEP);
    assert_eq!(Phasor::step_for(f32::INFINITY, RATE), MAX_STEP);
    assert_eq!(Phasor::step_for(f32::NEG_INFINITY, RATE), -MAX_STEP);
    assert_eq!(Phasor::step_for(480.0, RATE), 0.01);
}

/// NaN stops the phase, and the phase stays usable.
#[test]
fn nan_stops_the_phase() {
    let mut phasor = Phasor::new();
    phasor.advance(0.25);
    assert_eq!(Phasor::step_for(f32::NAN, RATE), 0.0);
    phasor.advance(Phasor::step_for(f32::NAN, RATE));
    assert_eq!(phasor.phase(), 0.25);
}

/// A negative step runs backward and wraps to the top of the range.
#[test]
fn a_negative_step_runs_backward() {
    let mut phasor = Phasor::new();
    phasor.advance(-0.25);
    assert_eq!(phasor.phase(), 0.75);
    phasor.advance(-0.25);
    assert_eq!(phasor.phase(), 0.5);
}

/// `tick` gives the phase before the step, starts at exactly 0, and wraps. The
/// values are exact, because 1/4 is exact in binary.
#[test]
fn tick_returns_the_phase_then_moves_on() {
    let mut phasor = Phasor::new();
    let values: Vec<f32> = (0..6).map(|_| phasor.tick(0.25)).collect();
    assert_eq!(values, [0.0, 0.25, 0.5, 0.75, 0.0, 0.25]);
}

/// The result depends on how many samples ran, not on how a caller groups them
/// into blocks.
#[test]
fn block_size_does_not_change_the_phase() {
    let step = Phasor::step_for(1_234.5, RATE);
    let mut whole = Phasor::new();
    let one_block: Vec<f32> = (0..1_000).map(|_| whole.tick(step)).collect();
    for size in [1, 77, 512] {
        let mut phasor = Phasor::new();
        let mut blocks = Vec::new();
        for chunk in one_block.chunks(size) {
            blocks.extend(chunk.iter().map(|_| phasor.tick(step)));
        }
        assert_eq!(blocks, one_block, "block size {size}");
    }
}

/// `reset` returns to phase 0, equal to a new phasor.
#[test]
fn reset_returns_to_zero() {
    let mut phasor = Phasor::new();
    phasor.advance(0.3);
    phasor.reset();
    assert_eq!(phasor, Phasor::new());
}

/// At the limit a wrap is still a jump of over half a cycle, which is how
/// anything watching the ramp tells it from a step. At exactly half a cycle the
/// phase would read 0, 0.5, 0, 0.5, and no jump would show.
#[test]
fn a_wrap_stays_visible_at_the_limit() {
    let step = Phasor::step_for(1e9, RATE);
    let mut phasor = Phasor::new();
    let phases: Vec<f32> = (0..200).map(|_| phasor.tick(step)).collect();
    assert!(phases.windows(2).any(|w| (w[1] - w[0]).abs() > 0.5));
}
