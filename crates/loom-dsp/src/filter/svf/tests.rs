use super::*;
use crate::testing::{RATE, tone_gain, warped};

const SAMPLE_RATE: f32 = RATE;

/// The analogue response at `hz`, after the bilinear transform's frequency
/// warping: the magnitude of the low, band, high, and notch outputs.
fn theory(hz: f32, cutoff: f32, damping: f32) -> [f64; 4] {
    let w = warped(f64::from(hz), f64::from(cutoff));
    let k = f64::from(damping);
    let denominator = ((1.0 - w * w).powi(2) + (k * w).powi(2)).sqrt();
    [1.0, w, w * w, (1.0 - w * w).abs()].map(|numerator| numerator / denominator)
}

/// Measures the amplitude of each output at `hz`.
fn measure(hz: f32, cutoff: f32, damping: f32) -> [f64; 4] {
    let coefficients = Coefficients::new(cutoff, damping, SAMPLE_RATE);
    let pick: [fn(Outputs) -> f32; 4] = [|o| o.low, |o| o.band, |o| o.high, |o| o.notch];
    pick.map(|output| {
        let mut filter = Svf::new();
        tone_gain(f64::from(hz), 1.0, |x| output(filter.tick(x, coefficients)))
    })
}

/// All four outputs match the analogue response under the bilinear transform,
/// below, at, and above the cutoff.
#[test]
fn response_matches_the_theory() {
    for (cutoff, damping) in [(1_000.0, 1.414), (1_000.0, 0.2), (8_000.0, 0.7), (100.0, 2.0)] {
        for ratio in [0.1, 0.5, 0.9, 1.0, 1.1, 2.0, 5.0] {
            let hz = cutoff * ratio;
            // Stay well below half the sample rate.
            if hz > SAMPLE_RATE * 0.45 {
                continue;
            }
            let measured = measure(hz, cutoff, damping);
            let expected = theory(hz, cutoff, damping);
            for (output, (m, e)) in measured.iter().zip(expected).enumerate() {
                assert!(
                    (m - e).abs() < 2e-3 * e.max(0.05),
                    "output {output} at {hz} Hz, cutoff {cutoff}, damping {damping}: measured {m}, expected {e}"
                );
            }
        }
    }
}

/// The low-pass, the scaled band-pass, and the high-pass sum to the input, on
/// any signal and with the cutoff moving.
#[test]
fn outputs_sum_to_the_input() {
    let mut filter = Svf::new();
    for i in 0..10_000 {
        let t = i as f32 / SAMPLE_RATE;
        let damping = 0.3;
        let cutoff = 2_000.0 + 1_500.0 * (t * 300.0).sin();
        let input = (t * 2_500.0).sin() + 0.5 * (t * 17_000.0).cos();
        let o = filter.tick(input, Coefficients::new(cutoff, damping, SAMPLE_RATE));
        let sum = o.low + damping * o.band + o.high;
        assert!((sum - input).abs() < 1e-3, "sample {i}: {sum} against {input}");
    }
}

/// With the most resonance the module allows and a cutoff near the top of the
/// range, an impulse stays finite and dies away.
#[test]
fn impulse_decays_at_high_resonance() {
    for cutoff in [5.0, 1_000.0, SAMPLE_RATE * 0.49] {
        let coefficients = Coefficients::new(cutoff, 0.02, SAMPLE_RATE);
        let mut filter = Svf::new();
        filter.tick(1.0, coefficients);
        // Low cutoffs ring longest: about 2 / (damping * w) seconds per e-fold.
        for _ in 0..4_000_000 {
            let o = filter.tick(0.0, coefficients);
            assert!(o.low.is_finite() && o.band.is_finite() && o.high.is_finite());
        }
        let o = filter.tick(0.0, coefficients);
        assert!(o.band.abs() < 1e-3, "cutoff {cutoff}: band still {}", o.band);
    }
}

/// An audio-rate sweep at high resonance stays bounded: the trapezoidal form
/// keeps its stability when the cutoff moves.
#[test]
fn audio_rate_sweep_stays_bounded() {
    let mut filter = Svf::new();
    let damping = 0.02;
    for i in 0..200_000 {
        let t = i as f32 / SAMPLE_RATE;
        let cutoff = 8_000.0 + 7_900.0 * (t * 5_000.0).sin();
        let o = filter.tick((t * 3_000.0).sin(), Coefficients::new(cutoff, damping, SAMPLE_RATE));
        assert!(o.band.abs() < 2.0 / damping, "sample {i}: band {}", o.band);
    }
}

/// `reset` clears the memory, so the next output is as from a new filter.
#[test]
fn reset_clears_the_state() {
    let coefficients = Coefficients::new(1_000.0, 0.5, SAMPLE_RATE);
    let mut filter = Svf::new();
    for _ in 0..100 {
        filter.tick(1.0, coefficients);
    }
    assert_ne!(filter, Svf::new());
    filter.reset();
    assert_eq!(filter, Svf::new());
    assert_eq!(filter.tick(0.0, coefficients).low, 0.0);
}

/// The same input gives the same bits whether it runs once or twice
/// (hard rule 3).
#[test]
fn is_deterministic() {
    let run = || {
        let coefficients = Coefficients::new(700.0, 0.4, SAMPLE_RATE);
        let mut filter = Svf::new();
        (0..1_000)
            .map(|i| filter.tick((i as f32 * 0.1).sin(), coefficients).low.to_bits())
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}
