# Phase 0 Engine Benchmarks

- **Date:** 2026-10-13
- **Machine:** cloud VM, 2 vCPUs, Intel Xeon @ 2.80 GHz, Linux. Not the reference machine. Numbers on a recent laptop should be similar or better.
- **Build:** `cargo bench -p loom-modules` (release, Rust 1.97)
- **Settings:** 48 kHz, 128-frame blocks. The real-time budget per block is **2,667 µs**.
- **Requirements:** NFR-PERF-08 (feedback benchmark), ADR-0002 (validation)

## Question

ADR-0002 runs feedback loops one sample at a time and everything else a block at a time. How much does the per-sample mode cost?

## Method

The same chain of N modules runs two ways:

- **chain:** acyclic. Each module processes the whole block in one call.
- **loop:** the last module feeds back into the first. The engine runs the chain one sample at a time (128 calls per module per block).

Two module types: `add` (very cheap, so call overhead dominates) and `svf` (a realistic filter).

## Results

Time per 128-frame block (median):

| Module | N | chain | loop | loop / chain | Extra cost per module per sample |
|---|---:|---:|---:|---:|---:|
| add | 1 | 0.96 µs | 2.15 µs | 2.2x | 9.3 ns |
| add | 10 | 5.0 µs | 12.3 µs | 2.4x | 5.7 ns |
| add | 50 | 23.4 µs | 57.9 µs | 2.5x | 5.4 ns |
| svf | 1 | 5.4 µs | 11.6 µs | 2.1x | 48 ns* |
| svf | 10 | 45.4 µs | 67.4 µs | 1.5x | 17 ns |
| svf | 50 | 233 µs | 319 µs | 1.4x | 13 ns |

\* The 1-module SVF loop also includes a noise source and a mixer, so this row overstates the per-module cost.

Example patches (one note playing):

| Patch | Time per block | Share of real-time budget |
|---|---:|---:|
| `subtractive.ron` (9 modules, no loops) | 9.3 µs | 0.35% |
| `fm-feedback.ron` (8 modules, one 4-module loop) | 9.2 µs | 0.35% |

## What this means

1. **Per-sample mode costs about 5 to 15 ns per module per sample.** That is the price of one indirect function call plus setup. For cheap modules it more than doubles their cost. For real DSP modules it adds about 40%.
2. **It is affordable.** A 50-filter feedback loop uses 12% of one core's budget on this VM. Typical loops (FM feedback, Karplus-Strong, filter feedback) have 2 to 10 modules.
3. **ADR-0002 is confirmed.** The hybrid model stays. Possible later optimisations, if real patches need them:
   - Compile hot loops to straight-line code (ADR-0002 option 4).
   - Process small sub-blocks inside a loop when its total delay is longer than one sample.
4. **The SVF is the most expensive module so far** (about 36 ns per sample) because it computes `tan()` and `exp2()` every sample. Caching coefficients when cutoff is not modulated would make it several times faster. Tracked as a follow-up, not a P0 blocker.

## Stress test (offline)

`loom stress <patch> --offline` runs the engine as fast as possible while making structural edits between buffers, and times each buffer.

| Patch | Simulated time | Edits | Mean load | Leaks |
|---|---:|---:|---:|---:|
| `subtractive.ron` | 120 s | 6,429 | 1.0% | 0 |
| `fm-feedback.ron` | 60 s | 22,500 (one per buffer) | 0.9% | 0 |

Rare single-buffer spikes (up to about 100% of budget) appeared in buffers both with and without edits, and changed between runs. They come from OS scheduling on a busy 2-vCPU VM without a real-time thread, not from the engine. Buffers right after a schedule swap cost at most about 5 to 8% in repeated runs.

The real P0 exit criterion is the **live** test on a desktop machine:

```sh
cargo run --release -p loom-cli -- stress examples/patches/subtractive.ron --seconds 600
```

It plays through the real audio device with real-time scheduling and reports overruns.

### Live result (2026-10-13)

Run on the maintainer's Mac (default output device). The device ran at 44.1 kHz with 512-frame buffers (11.6 ms per buffer), worked out from 51,680 callbacks in 600 s.

| Patch | Duration | Edits | Schedule swaps | Max load | Overruns | Stream errors | Leaks | Result |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| `subtractive.ron` | 600 s | 11,026 | 11,027 | 4.1% | 0 | 0 | 0 | **PASS** |

The P0 exit criterion ("zero dropouts over 10 minutes of edits") is met.

Follow-up: repeat with `--buffer 128` (2.9 ms per buffer). Smaller buffers are what live performers use for low latency, and they leave less room for timing jitter.

## How to reproduce

```sh
cargo bench -p loom-modules
cargo run --release -p loom-cli -- stress examples/patches/fm-feedback.ron --offline --seconds 60 --rate 200
```
