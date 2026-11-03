//! Helpers for testing algorithms against their theory.

use std::f64::consts::{PI, TAU};

/// The sample rate the tests use.
pub const RATE: f32 = 48_000.0;

/// The gain of `system` at `hz`: runs a sine of `amplitude` until it settles,
/// then measures the output at that frequency.
pub fn tone_gain(hz: f64, amplitude: f64, mut system: impl FnMut(f32) -> f32) -> f64 {
    let step = TAU * hz / f64::from(RATE);
    let (settle, length) = (96_000, 48_000);
    let (mut a, mut b) = (0.0, 0.0);
    for i in 0..settle + length {
        let phase = step * i as f64;
        let y = f64::from(system((amplitude * phase.sin()) as f32));
        if i >= settle {
            a += y * phase.sin();
            b += y * phase.cos();
        }
    }
    2.0 * f64::hypot(a, b) / f64::from(length) / amplitude
}

/// `hz` as an analogue frequency relative to `cutoff`, under the bilinear
/// transform.
pub fn warped(hz: f64, cutoff: f64) -> f64 {
    let warp = |f: f64| (PI * f / f64::from(RATE)).tan();
    warp(hz) / warp(cutoff)
}

/// The share of a periodic wave's power that is not at its harmonics, so it
/// is aliasing. `wave` is called for one second, at a whole number of hertz.
pub fn alias_share(hz: u32, mut wave: impl FnMut(f32) -> f32) -> f64 {
    let frames = RATE as usize;
    let samples: Vec<f64> = (0..frames)
        .map(|i| {
            let phase = (u64::from(hz) * i as u64 % frames as u64) as f64 / frames as f64;
            f64::from(wave(phase as f32))
        })
        .collect();
    let total: f64 = samples.iter().map(|y| y * y).sum::<f64>() / frames as f64;
    let mut harmonics = samples.iter().sum::<f64>().powi(2) / (frames * frames) as f64;
    let mut n = 1;
    while n * hz < RATE as u32 / 2 {
        let step = TAU * f64::from(n * hz) / f64::from(RATE);
        let (mut a, mut b) = (0.0, 0.0);
        for (i, y) in samples.iter().enumerate() {
            a += y * (step * i as f64).sin();
            b += y * (step * i as f64).cos();
        }
        harmonics += 2.0 * (a * a + b * b) / (frames * frames) as f64;
        n += 1;
    }
    (total - harmonics).max(0.0) / total
}
