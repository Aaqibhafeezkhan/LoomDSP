# LoomDSP

[![CI](https://github.com/brylie/LoomDSP/actions/workflows/ci.yml/badge.svg)](https://github.com/brylie/LoomDSP/actions/workflows/ci.yml)

An open-source modular audio environment written in Rust, for musical experimentation and live performance.

Build instruments and effects by connecting small DSP modules. Hide the wiring behind a front panel. Play it live with MIDI controllers. Later, export it as a plugin (CLAP, VST3) or a standalone app.

**Status: Phase 0 (engine foundations).** There is no graphical editor yet. The engine, the module library, the patch format, and a command-line player work today. See the [roadmap](docs/product/roadmap.md).

## Try it

You need [Rust](https://rustup.rs) 1.88 or newer.

```sh
# List the available modules
cargo run --release -p loom-cli -- modules

# Play a patch with your MIDI keyboard (first MIDI input found)
cargo run --release -p loom-cli -- play examples/patches/subtractive.ron

# No MIDI keyboard? Hold a note instead
cargo run --release -p loom-cli -- play examples/patches/fm-feedback.ron --note 57

# Render to a WAV file
cargo run --release -p loom-cli -- render examples/patches/subtractive.ron -o out.wav --note 48 --note 55 --note 60

# See where the engine found feedback loops
cargo run --release -p loom-cli -- check examples/patches/fm-feedback.ron

# Edit the patch structure 20 times per second while it plays, and count dropouts
cargo run --release -p loom-cli -- stress examples/patches/subtractive.ron --seconds 600
```

Linux needs the ALSA development package (`libasound2-dev` on Debian and Ubuntu).

## What works in Phase 0

- A runtime-editable DSP graph. Add, remove, and reconnect modules while audio plays, with no dropouts.
- Sample-accurate feedback loops. The engine finds cycles and runs them one sample at a time. If a loop has no explicit 1-sample delay, the engine inserts one and tells you where ([ADR-0002](docs/adr/0002-graph-execution-and-feedback.md)).
- A lock-free boundary between the control thread and the audio thread. The audio path never allocates, locks, or waits, and a test enforces this ([ADR-0001](docs/adr/0001-lock-free-control-audio-boundary.md)).
- Deterministic output. The same patch and input give bit-identical audio at any host buffer size.
- Patch files in RON ([ADR-0008](docs/adr/0008-patch-format-ron.md)), with polymorphic signals ([ADR-0007](docs/adr/0007-polymorphic-signal-model.md)).
- 12 modules: phase generator, sine, saw (band-limited), noise, state variable filter, ADSR, add, multiply, constant, 1-sample delay, MIDI note input, audio output.

## Repository layout

| Path | What it is |
|---|---|
| `crates/loom-core` | Engine core: module API, graph, compiler, real-time engine. No I/O, builds for WASM. |
| `crates/loom-modules` | Built-in modules, integration tests, and benchmarks. |
| `crates/loom-patch` | RON patch format: load, save, migrate. |
| `crates/loom-rt` | Small real-time helpers (denormal flushing, smoothing). Reusable outside LoomDSP. |
| `crates/loom-host-desktop` | Audio (cpal) and MIDI (midir) host for desktop. |
| `crates/loom-cli` | The `loom` command-line tool. |
| `examples/patches` | Example patches. |
| `docs` | Requirements, roadmap, ADRs, and benchmark results. |

## Documentation

- [Vision](docs/product/vision.md), [requirements](docs/product/requirements.md), and [roadmap](docs/product/roadmap.md)
- [Architecture decisions](docs/adr/README.md)
- [Phase 0 benchmark results](docs/benchmarks/p0-engine.md)

## Contributing

Contributions are welcome, including AI-assisted ones. Read [CONTRIBUTING.md](CONTRIBUTING.md) and the [AI contribution policy](AI_POLICY.md). Issues labelled `good first issue` are a good place to start.

## Licence

Apache-2.0. See [LICENSE](LICENSE). Patches you create are yours, under any licence you choose.
