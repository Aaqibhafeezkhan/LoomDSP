# Contributing to LoomDSP

Thank you for helping. This guide explains how to build, test, and send changes.

## Before you start

- Look for an issue first. If none exists, open one and describe what you want to change.
- Issues reference requirement IDs from [docs/product/requirements.md](docs/product/requirements.md), for example `FR-MIDI-02`. Mention the ID in your pull request too. This links code to the reason it exists.
- Big design changes need an [ADR](docs/adr/README.md) first. The ADR records the decision only. Details that will change (ports, parameters, examples) go in a [design specification](docs/design/README.md), which you update with the code. See [kinds of documents](docs/README.md#kinds-of-documents).
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

### Git hooks (recommended)

The repository uses [prek](https://prek.j178.dev) to run quick checks before each commit, so you hear about problems while the change is fresh. Tools come from [mise](https://mise.jdx.dev):

```sh
mise install    # Rust, prek, typos, cargo-deny (from mise.toml)
prek install    # installs the pre-commit and pre-push hooks
```

| When | Checks |
|---|---|
| Every commit (a few seconds) | Whitespace, line endings, TOML/YAML syntax, large files, private keys, `cargo fmt`, `cargo clippy`, `typos`, and no direct commits to `main` |
| Before push | `cargo test`, `cargo deny check` |

`cargo fmt` and the whitespace hooks fix files for you. When they do, the commit stops: review the changes, stage them, and commit again. Run everything by hand with `prek run --all-files`. The configuration is in `prek.toml`.

### What CI checks

Every pull request runs these jobs. All must pass before merging (the "CI success" check).

| Job | What it checks | Run it locally |
|---|---|---|
| Format | `rustfmt` style | `cargo fmt --all --check` |
| Clippy | Lint groups set in the workspace `Cargo.toml`: correctness, suspicious, complexity, perf, style, cargo | `cargo clippy --workspace --all-targets -- -D warnings` |
| Test | All tests on Linux, macOS (Apple Silicon), and Windows, plus benchmarks compile | `cargo test --workspace` |
| Coverage | Line coverage, shown in the job summary | `cargo llvm-cov --workspace` (install [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)) |
| Docs | API docs build without warnings | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` |
| MSRV | Still builds with Rust 1.88 | `cargo +1.88 check --workspace` |
| WebAssembly | Engine crates build for `wasm32-unknown-unknown` | `cargo build --target wasm32-unknown-unknown -p loom-core -p loom-modules -p loom-patch -p loom-rt` |
| Licences | Only permissive licences, no known vulnerabilities | `cargo deny check` (install [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)) |
| Spelling | Typos in code and docs | `typos` (install [typos-cli](https://github.com/crate-ci/typos)) |

Why warnings fail CI but not your local build: a warning while you are still editing is noise, but a warning on `main` is a problem someone else inherits.

Dependabot opens weekly pull requests for dependency updates.

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
