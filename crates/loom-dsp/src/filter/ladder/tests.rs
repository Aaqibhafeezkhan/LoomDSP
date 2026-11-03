use super::*;
use crate::testing::{RATE, tone_gain, warped};

/// The coefficients for a cutoff and a feedback, with no input gain.
fn coefficients(cutoff_hz: f32, feedback: f32) -> Coefficients {
    Coefficients::new(Settings {
        cutoff_hz,
        feedback,
        input_gain: 1.0,
        sample_rate: RATE,
    })
}

const ALL: [Slope; 3] = [Slope::Db12, Slope::Db18, Slope::Db24];

/// At a small level the saturation does nothing, so each slope matches the
/// closed-loop transfer `1 / ((1 + s)^N + k)` under the bilinear transform.
#[test]
fn small_signal_response_matches_the_theory() {
    let cutoff = 1_000.0;
    for slope in ALL {
        let stages = slope.stages();
        for k in [0.0, 1.0] {
            for ratio in [0.2, 0.5, 1.0, 2.0, 4.0] {
                let hz = cutoff * ratio;
                let w = warped(hz, cutoff);
                let (mut real, mut imaginary) = (1.0f64, 0.0f64);
                for _ in 0..stages {
                    (real, imaginary) = (real - imaginary * w, imaginary + real * w);
                }
                let expected = 1.0 / f64::hypot(real + k, imaginary);
                let mut filter = Ladder::new(RATE);
                filter.reset(slope);
                let settings = coefficients(cutoff as f32, k as f32);
                let measured = tone_gain(hz, 1e-5, |x| filter.tick(x, settings));
                assert!(
                    (measured - expected).abs() < 0.01 * expected.max(0.01),
                    "{stages} stages, k {k}, {hz} Hz: {measured} against {expected}"
                );
            }
        }
    }
}

/// During a slope change the feedback gain never passes the limit of a
/// tap that is heard, never jumps, and ends at the new slope's limit.
#[test]
fn slope_change_keeps_the_gain_under_every_heard_limit() {
    use Slope::{Db12, Db18, Db24};
    for (from, to) in [
        (Db12, Db24),
        (Db24, Db12),
        (Db18, Db24),
        (Db24, Db18),
        (Db12, Db18),
        (Db18, Db12),
    ] {
        let mut ladder = Ladder::new(RATE);
        ladder.reset(from);
        ladder.change_slope(to);
        let mut last = ladder.limit;
        for _ in 0..2_000 {
            let k = ladder.step_slope();
            for (tap, &w) in ladder.weights.iter().enumerate() {
                assert!(w == 0.0 || k <= K_MAX[tap], "{from:?} to {to:?}: k {k} on tap {tap}");
            }
            assert!(
                (k - last).abs() <= ladder.limit_step * 1.001,
                "{from:?} to {to:?}: jump"
            );
            last = k;
        }
        assert_eq!(ladder.weights[to.tap()], 1.0, "{from:?} to {to:?}");
        assert_eq!(last, to.feedback_limit(), "{from:?} to {to:?}");
    }
}

/// At full resonance the saturation keeps a ringing loop bounded.
#[test]
fn full_resonance_stays_bounded() {
    for slope in ALL {
        let settings = coefficients(2_000.0, slope.feedback_limit());
        let mut ladder = Ladder::new(RATE);
        ladder.reset(slope);
        ladder.tick(1.0, settings);
        for _ in 0..200_000 {
            let output = ladder.tick(0.0, settings);
            assert!(output.is_finite() && output.abs() < 16.0, "{slope:?}: {output}");
        }
    }
}

/// `clear_loop` empties the stages and leaves the slope alone. `reset` goes
/// straight to a slope.
#[test]
fn clearing_and_resetting() {
    let mut ladder = Ladder::new(RATE);
    let settings = coefficients(1_000.0, 2.0);
    ladder.reset(Slope::Db18);
    ladder.tick(1.0, settings);
    ladder.clear_loop();
    assert_eq!(ladder.target(), Slope::Db18);
    assert_eq!(ladder.tick(0.0, settings), 0.0);
    ladder.change_slope(Slope::Db24);
    ladder.reset(Slope::Db12);
    assert_eq!(ladder.target(), Slope::Db12);
    assert_eq!(ladder.step_slope(), Slope::Db12.feedback_limit());
}

/// At full resonance the 18 and 24 dB slopes sing on their own, because their
/// limits sit above where the loop starts to oscillate. An impulse is still
/// ringing long after it. The 12 dB slope has no such point, so it dies away.
#[test]
fn full_resonance_sings_on_the_two_steep_slopes() {
    for (slope, sings) in [(Slope::Db12, false), (Slope::Db18, true), (Slope::Db24, true)] {
        let settings = coefficients(1_000.0, slope.feedback_limit());
        let mut ladder = Ladder::new(RATE);
        ladder.reset(slope);
        ladder.tick(1.0, settings);
        for _ in 0..96_000 {
            ladder.tick(0.0, settings);
        }
        let peak = (0..960).map(|_| ladder.tick(0.0, settings).abs()).fold(0.0, f32::max);
        assert_eq!(peak > 0.01, sings, "{slope:?}: peak {peak}");
    }
}
