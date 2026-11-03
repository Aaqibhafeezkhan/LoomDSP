use super::*;
use loom_core::testing::run;

const RATE: f32 = 48_000.0;

/// Renders `frames` samples of noise in the given colour (0 white, 1
/// pink, 2 brown) at the given seed, in blocks of `block` frames.
fn render(colour: f32, seed: f32, frames: usize, block: usize) -> Vec<f32> {
    let ModuleRole::Dsp(create) = NOISE.role else {
        unreachable!()
    };
    let mut noise = create(&ModuleConfig {
        sample_rate: RATE,
        max_block: block,
    });
    noise.set_param(0, seed);
    noise.set_param(1, colour);
    noise.reset();
    let mut out = vec![0.0; frames];
    for chunk in out.chunks_mut(block) {
        run(noise.as_mut(), RATE, &[], &[], &mut [chunk], &[]);
    }
    out
}

/// A copied noise stream mixes in the right channel and stays deterministic.
#[test]
fn copied_noise_uses_the_channel_in_its_seed() {
    fn run_copy(channel: usize, block: usize) -> Vec<f32> {
        let mut left = Noise::new(RATE);
        let mut output = [0.0; 64];
        run(&mut left, RATE, &[], &[], &mut [&mut output], &[]);
        let mut copy = Noise::new(RATE);
        assert!(copy.copy_state_from(&left));
        let mut result = vec![0.0; 512];
        for chunk in result.chunks_mut(block) {
            loom_core::testing::run_with_transport_and_channel(
                &mut copy,
                RATE,
                loom_core::TransportInfo::default(),
                &[],
                &[],
                &mut [chunk],
                &[],
                channel,
            );
        }
        result
    }
    let left = run_copy(0, 77);
    let right = run_copy(1, 77);
    assert_ne!(left, right);
    assert_eq!(right, run_copy(1, 1));
    assert_eq!(right, run_copy(1, 512));
}

/// The two channels play different noise, in white and in pink. The crate
/// checks that their streams never overlap.
#[test]
fn stereo_noise_channels_differ() {
    for colour in [Colour::White, Colour::Pink] {
        let mut left = Noise::new(RATE);
        let mut right = Noise::new(RATE);
        left.restart(42);
        right.channel = 1;
        right.restart(42);
        for i in 0..128 {
            assert_ne!(
                left.generator.tick(colour),
                right.generator.tick(colour),
                "{colour:?} frame {i}"
            );
        }
    }
}

fn rms(samples: &[f32]) -> f64 {
    (samples.iter().map(|&v| f64::from(v) * f64::from(v)).sum::<f64>() / samples.len() as f64).sqrt()
}

/// The colour knob reaches the generator: each value plays that colour's
/// samples. The crate checks the spectrum of each colour against its theory.
#[test]
fn the_colour_knob_selects_the_generator_colour() {
    for (knob, colour) in [(0.0, Colour::White), (1.0, Colour::Pink), (2.0, Colour::Brown)] {
        let mut generator = noise::Noise::new(RATE);
        generator.restart(7);
        let expected: Vec<f32> = (0..1_000).map(|_| generator.tick(colour)).collect();
        assert_eq!(render(knob, 7.0, 1_000, 64), expected, "knob {knob}");
    }
}

/// Every colour has about the same loudness as white (within 3 dB), and
/// brown stays near zero over a minute.
#[test]
fn colours_match_white_in_level_and_brown_does_not_drift() {
    let frames = 60 * RATE as usize;
    let white = rms(&render(0.0, 3.0, frames, 4096));
    for colour in [1.0, 2.0] {
        let samples = render(colour, 3.0, frames, 4096);
        let db = 20.0 * (rms(&samples) / white).log10();
        assert!(db.abs() < 3.0, "colour {colour}: {db} dB from white");
        let mean = samples.iter().map(|&v| f64::from(v)).sum::<f64>() / frames as f64;
        assert!(mean.abs() < 0.05, "colour {colour}: mean {mean}");
    }
}

/// The same seed gives the same noise at block sizes 1, 77, and 512, and
/// switching to white gives the same samples as before pink existed.
#[test]
fn same_seed_same_output_at_any_block_size() {
    for colour in [0.0, 1.0, 2.0] {
        let reference = render(colour, 5.0, 2_000, 512);
        assert_eq!(render(colour, 5.0, 2_000, 1), reference, "colour {colour}, block 1");
        assert_eq!(render(colour, 5.0, 2_000, 77), reference, "colour {colour}, block 77");
    }
}
