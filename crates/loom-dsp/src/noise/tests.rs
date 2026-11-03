use super::*;
use crate::testing::RATE;

const COLOURS: [Colour; 3] = [Colour::White, Colour::Pink, Colour::Brown];

/// The RMS of `frames` samples of a colour.
fn rms(colour: Colour, frames: usize) -> f32 {
    let mut noise = Noise::new(RATE);
    noise.restart(7);
    let sum: f32 = (0..frames).map(|_| noise.tick(colour).powi(2)).sum();
    (sum / frames as f32).sqrt()
}

/// One seed gives the same bits, and a restart repeats them (hard rule 3).
#[test]
fn a_seed_repeats_exactly() {
    for colour in COLOURS {
        let mut noise = Noise::new(RATE);
        noise.restart(99);
        let first: Vec<u32> = (0..1_000).map(|_| noise.tick(colour).to_bits()).collect();
        noise.restart(99);
        let second: Vec<u32> = (0..1_000).map(|_| noise.tick(colour).to_bits()).collect();
        assert_eq!(first, second, "{colour:?}");
    }
}

/// Two seeds give different numbers.
#[test]
fn seeds_differ() {
    let mut a = Noise::new(RATE);
    let mut b = Noise::new(RATE);
    a.restart(1);
    b.restart(2);
    assert_ne!(a.tick(Colour::White), b.tick(Colour::White));
}

/// White noise from -1 to 1 has an RMS of `1 / √3`, and the others match it.
#[test]
fn every_colour_has_the_same_rms() {
    let white = 1.0 / 3.0f32.sqrt();
    for colour in COLOURS {
        let measured = rms(colour, RATE as usize * 10);
        assert!((measured - white).abs() < 0.1, "{colour:?}: {measured}");
    }
}

/// A white stream is the same whichever colour is asked for.
#[test]
fn colours_share_the_white_stream() {
    let mut noise = Noise::new(RATE);
    noise.tick(Colour::Pink);
    noise.tick(Colour::Brown);
    let mut plain = Noise::new(RATE);
    plain.tick(Colour::White);
    plain.tick(Colour::White);
    assert_eq!(noise.state, plain.state);
}

/// The brown memory survives when asked to, so a brown signal does not jump,
/// and a full restart clears it.
#[test]
fn restarts_keep_or_clear_the_brown_memory() {
    let mut noise = Noise::new(RATE);
    for _ in 0..1_000 {
        noise.tick(Colour::Brown);
    }
    let brown = noise.brown;
    assert_ne!(brown, 0.0);
    noise.restart_keeping_brown(5);
    assert_eq!(noise.brown, brown);
    noise.restart(5);
    assert_eq!(noise.brown, 0.0);
}

/// A copy takes the brown memory and not the random stream.
#[test]
fn copy_brown_leaves_the_streams_alone() {
    let mut source = Noise::new(RATE);
    for _ in 0..100 {
        source.tick(Colour::Brown);
    }
    let mut copy = Noise::new(RATE);
    let streams = (copy.state, copy.aux);
    copy.copy_brown_from(&source);
    assert_eq!(copy.brown, source.brown);
    assert_eq!((copy.state, copy.aux), streams);
}

/// The value from one step stays from -1 to 1, and a stream never sticks at 0.
#[test]
fn white_stays_in_range() {
    let mut state = 1;
    for _ in 0..100_000 {
        let value = next_white(&mut state);
        assert!((-1.0..=1.0).contains(&value));
        assert_ne!(state, 0);
    }
}

/// No random stream repeats another at a shifted offset, within or across two
/// seeds. The two seeds are the ones `loom.osc.noise` gives its stereo channels.
#[test]
fn streams_from_two_seeds_do_not_overlap() {
    let (mut left, mut right) = (Noise::new(RATE), Noise::new(RATE));
    left.restart(42);
    right.restart(42 ^ 0x85EB_CA6B);
    let states = [left.state, left.aux, right.state, right.aux].map(|mut state| {
        let mut values: Vec<u32> = (0..4_096)
            .map(|_| {
                next_white(&mut state);
                state
            })
            .collect();
        values.sort_unstable();
        values
    });
    for (i, a) in states.iter().enumerate() {
        for (j, b) in states.iter().enumerate().skip(i + 1) {
            assert!(
                !a.iter().any(|x| b.binary_search(x).is_ok()),
                "streams {i} and {j} overlap"
            );
        }
    }
}

/// An in-place radix-2 FFT. `re.len()` must be a power of two.
fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let angle = -std::f64::consts::TAU / len as f64;
        for start in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (sin, cos) = (angle * k as f64).sin_cos();
                let (a, b) = (start + k, start + k + len / 2);
                let (tr, ti) = (re[b] * cos - im[b] * sin, re[b] * sin + im[b] * cos);
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
            }
        }
        len <<= 1;
    }
}

/// The slope of the power spectral density between 100 Hz and 10 kHz, in
/// dB per octave. It averages the power per bin over one octave around
/// each frequency, over many Hann-windowed segments.
fn slope_db_per_octave(samples: &[f32]) -> f64 {
    const SEGMENT: usize = 1 << 16;
    let mut power = vec![0.0; SEGMENT / 2];
    for segment in samples.as_chunks::<SEGMENT>().0 {
        let mut re: Vec<f64> = segment
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let hann = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / SEGMENT as f64).cos();
                f64::from(v) * hann
            })
            .collect();
        let mut im = vec![0.0; SEGMENT];
        fft(&mut re, &mut im);
        for (bin, p) in power.iter_mut().enumerate() {
            *p += re[bin] * re[bin] + im[bin] * im[bin];
        }
    }
    let bin_hz = f64::from(RATE) / SEGMENT as f64;
    let mean_around = |centre: f64| {
        let (low, high) = (
            (centre / 2f64.sqrt() / bin_hz) as usize,
            (centre * 2f64.sqrt() / bin_hz) as usize,
        );
        power[low..high].iter().sum::<f64>() / (high - low) as f64
    };
    let octaves = (10_000.0f64 / 100.0).log2();
    10.0 * (mean_around(10_000.0) / mean_around(100.0)).log10() / octaves
}

/// Pink falls 3 dB per octave and brown 6, each within 1 dB. White is flat.
#[test]
fn spectra_follow_the_colour() {
    for (colour, expected) in [(Colour::White, 0.0), (Colour::Pink, -3.0), (Colour::Brown, -6.0)] {
        let mut noise = Noise::new(RATE);
        noise.restart(7);
        let samples: Vec<f32> = (0..16 << 16).map(|_| noise.tick(colour)).collect();
        let slope = slope_db_per_octave(&samples);
        assert!((slope - expected).abs() < 1.0, "{colour:?}: {slope} dB per octave");
    }
}

/// The generator is Marsaglia's xorshift32 with shifts 13, 17, and 5. A seed's
/// numbers are part of what a patch sounds like, so they must not change. From
/// a state of 1: `1 ^ (1 << 13)` is 8193, shifting right by 17 changes nothing,
/// and `8193 ^ (8193 << 5)` is 270369.
#[test]
fn white_is_xorshift32() {
    let mut state = 1;
    next_white(&mut state);
    assert_eq!(state, 270_369);
}
