//! Engine benchmarks (NFR-PERF-08, ADR-0002 validation).
//!
//! Compares the same chain of modules run two ways:
//! - `chain`: acyclic, so it runs a whole block per module call.
//! - `loop`: the last module feeds back into the first, so the engine runs
//!   the chain one sample at a time.
//!
//! The difference is the cost of per-sample feedback regions.
//!
//! Run with: `cargo bench -p loom-modules`

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use loom_core::{EngineConfig, EngineParts, MidiEvent, create_engine};
use loom_patch::Patch;
use std::hint::black_box;

const BLOCK: usize = 128;

/// `n` add modules in a chain, fed by a constant, ending at the audio output.
/// With `feedback`, the last adder also feeds the first adder's `b` input.
fn chain(n: usize, feedback: bool, kind: &str) -> EngineParts {
    let mut parts = create_engine(loom_modules::registry(), EngineConfig::default());
    let c = &mut parts.controller;
    let source = c.add_module("loom.math.constant").unwrap();
    c.set_param_named(source, "value", 0.001).unwrap();
    let mut previous = (source, "out");
    let mut first = None;
    for _ in 0..n {
        let module = c.add_module(kind).unwrap();
        c.connect_named(previous.0, previous.1, module, "a").unwrap();
        first.get_or_insert(module);
        previous = (module, "out");
    }
    if feedback {
        c.connect_named(previous.0, previous.1, first.unwrap(), "b").unwrap();
    }
    let out = c.add_module("loom.io.audio-out").unwrap();
    c.connect_named(previous.0, previous.1, out, "left").unwrap();
    let report = c.commit().unwrap();
    assert_eq!(report.loops.len(), usize::from(feedback));
    parts
}

fn feedback_regions(c: &mut Criterion) {
    let mut left = vec![0.0f32; BLOCK];
    let mut right = vec![0.0f32; BLOCK];
    for kind in ["loom.math.add", "loom.filter.svf"] {
        let short = kind.rsplit('.').next().unwrap();
        let mut group = c.benchmark_group(format!("feedback/{short}"));
        for n in [1usize, 10, 50] {
            group.throughput(Throughput::Elements((n * BLOCK) as u64));
            for feedback in [false, true] {
                // The SVF's input is named "in", not "a"; use the add chain
                // shape with a mul in front for it.
                let mut parts = if kind == "loom.filter.svf" {
                    svf_chain(n, feedback)
                } else {
                    chain(n, feedback, kind)
                };
                let label = if feedback { "loop" } else { "chain" };
                group.bench_with_input(BenchmarkId::new(label, n), &n, |b, _| {
                    b.iter(|| {
                        parts.engine.process(&mut left, &mut right);
                        black_box(&left);
                    })
                });
            }
        }
        group.finish();
    }
}

/// Like `chain`, but with SVF filters (a heavier module).
fn svf_chain(n: usize, feedback: bool) -> EngineParts {
    let mut parts = create_engine(loom_modules::registry(), EngineConfig::default());
    let c = &mut parts.controller;
    let source = c.add_module("loom.osc.noise").unwrap();
    let mix = c.add_module("loom.math.add").unwrap();
    c.connect_named(source, "out", mix, "a").unwrap();
    let mut previous = mix;
    let mut last_port = "out";
    for _ in 0..n {
        let module = c.add_module("loom.filter.svf").unwrap();
        c.connect_named(previous, last_port, module, "in").unwrap();
        previous = module;
        last_port = "lowpass";
    }
    if feedback {
        c.connect_named(previous, last_port, mix, "b").unwrap();
    }
    let out = c.add_module("loom.io.audio-out").unwrap();
    c.connect_named(previous, last_port, out, "left").unwrap();
    c.commit().unwrap();
    parts
}

fn patches(c: &mut Criterion) {
    let mut left = vec![0.0f32; BLOCK];
    let mut right = vec![0.0f32; BLOCK];
    let mut group = c.benchmark_group("patch");
    group.throughput(Throughput::Elements(BLOCK as u64));
    for name in ["subtractive.ron", "fm-feedback.ron"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/patches")
            .join(name);
        let patch = Patch::load(path).unwrap();
        let mut parts = create_engine(loom_modules::registry(), EngineConfig::default());
        patch.load_into(&mut parts.controller).unwrap();
        parts.midi.send(MidiEvent::note_on(0, 0, 60, 100));
        group.bench_function(name, |b| {
            b.iter(|| {
                parts.engine.process(&mut left, &mut right);
                black_box(&left);
            })
        });
    }
    group.finish();
}

criterion_group!(benches, feedback_regions, patches);
criterion_main!(benches);
