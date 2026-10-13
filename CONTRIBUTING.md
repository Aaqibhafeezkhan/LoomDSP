# Contributing to LoomDSP

Thank you for helping. This guide explains how to build, test, and send changes.

## Before you start

- Look for an issue first. If none exists, open one and describe what you want to change.
- Issues reference requirement IDs from [docs/product/requirements.md](docs/product/requirements.md), for example `FR-MIDI-02`. Mention the ID in your pull request too. This links code to the reason it exists.
- Big design changes need an [ADR](docs/adr/README.md) first.
- AI-assisted contributions are welcome. Read the [AI contribution policy](AI_POLICY.md).

Good starting points: issues labelled `good first issue` or `help wanted`. New modules are often a good first contribution: they are small, self-contained, and easy to test.

## Build and test

You need Rust 1.88 or newer. On Linux, install the ALSA development package (`libasound2-dev`).

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo bench -p loom-modules          # optional, takes a few minutes
```

CI also checks that the engine core builds for WebAssembly and that every dependency has an allowed licence (`cargo deny check`).

## Rules for audio code

The audio thread has a hard deadline. At 48 kHz with 128-frame buffers, each buffer must finish in 2.67 ms, every time. So code that runs inside `Module::process` or `Engine::process` must not:

- allocate or free memory (no `Vec::push` past capacity, no `Box::new`, no `String`),
- take a lock or wait on another thread,
- make system calls (no printing, logging, or file access).

Why: each of these can take an unpredictable amount of time, and one late buffer is an audible dropout. The test `crates/loom-modules/tests/no_alloc.rs` aborts if the audio path allocates.

Modules must also give the same output whether they process 1 frame or 128 frames at a time. Inside feedback loops the engine calls them one frame at a time (ADR-0002). The test `output_does_not_depend_on_host_buffer_size` checks this.

## Adding a module

1. Add a `static` `ModuleDescriptor` and a struct implementing `Module` in `crates/loom-modules/src/`.
2. Add it to `ALL` in `crates/loom-modules/src/lib.rs`.
3. Give it a stable, namespaced ID (`loom.<category>.<name>`) and version 1. Never rename a released ID, port, or parameter. Add a new version instead.
4. Write a unit test with `loom_core::testing::run`.
5. Run the full test suite.

## Dependencies

Only permissively licensed dependencies are allowed (MIT, Apache-2.0, BSD, ISC, Zlib, and similar). No GPL, LGPL, AGPL, or other copyleft licences. See [ADR-0006](docs/adr/0006-dependency-and-licence-policy.md). Add a short dependency review to the pull request for any new dependency.

## Pull requests

- Keep them focused: one change per pull request.
- Include tests.
- Explain what changed and why, in plain language.
- CI must pass.

## Writing style for docs

Short, direct sentences. Plain words. Explain the reason behind a rule, not only the rule. Many contributors read English as a second language.

## Licence

By contributing, you agree that your contribution is licensed under Apache-2.0.
