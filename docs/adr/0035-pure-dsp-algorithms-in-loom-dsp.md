# ADR-0035: Pure DSP algorithms live in `loom-dsp`, and modules wrap them

- **Status:** Accepted
- **Date:** 2026-11-02
- **Related requirements:** FR-SDK-01, FR-SDK-02, FR-SDK-06, NFR-PORT-02
- **Related decisions:** ADR-0006 (dependency and licence policy), ADR-0017 (primitives and abstractions), ADR-0018 (clean room), ADR-0022 (state copy)
- **Specification:** [DSP algorithms](../design/dsp-algorithms.md)
- **Issue:** [#537](https://github.com/brylie/LoomDSP/issues/537)

## Context

Most DSP math sits inside `Module::process`, mixed with ports, parameter mapping, smoothing, and guards against bad cable values. So an algorithm can only be used, tested, and studied through the module API.

This has three costs:

- **Tests are indirect.** The harness compares a whole module with a reference render. That catches a change, but it does not show that a filter matches its theory.
- **Research is hard.** Comparing two methods for one filter means building two modules. Nothing gives an algorithm one place for its sources and its measurements.
- **Nobody else can use it.** Vision goal G6 and FR-SDK-06 want reusable crates that give back to the Rust audio ecosystem. To use our filter today, a project must take on `loom-core`, its descriptors, and its breaking changes. A separate application, such as a possible mastering tool, has a different way of working, but it could share the same algorithms.

The cost of moving grows with each module. M2 adds Dynamics, EQ, and Crossover (#541). About 15 existing algorithms are worth moving today, and some are close to pure already (`DelayBuffer`, `poly_blep`, `poly_blamp`, `DcBlocker`, `Limiter`).

## Options considered

1. **Keep algorithms in modules.** No work now. All three costs stay and grow, and extraction later is larger.
2. **Plain helper functions and structs inside `loom-modules`.** Tests get better, but the crate still depends on `loom-core`, so nobody else can use them, and nothing stops the two from mixing again.
3. **A new crate, `loom-dsp`, with no workspace dependencies.** The compiler enforces the separation. Costs one more crate, a wrapper for each module, and the risk of a slower hot path, which a benchmark checks.
4. **Extract only when a second project needs it.** This waits for evidence, but the work is largest then, and the testing and research gains wait with it.

## Decision

Option 3. Pure DSP algorithms live in `crates/loom-dsp`, and a module becomes a thin wrapper over them.

- **Dependencies point one way.** `loom-dsp` depends on nothing in the workspace. `loom-modules` and `loom-core` may depend on it, never the reverse. It has no third-party dependencies unless a later decision adds one. Dev dependencies are allowed, because they do not reach users.
- **An algorithm is a plain struct.** It holds state, has a `tick` that processes one sample, and has a `reset`. Coefficients are a separate small `Copy` type, so a caller decides how often to compute them. Audio-rate modulation computes them each sample; a fixed filter computes them once. `tick` is the contract. A block helper can be added later without breaking it. There is no shared trait, because the signatures differ and nobody needs one yet.
- **What goes in:** algorithms that make sense outside a modular patch, such as filters, oscillator shapes, phase generation, delay lines, envelopes, dynamics, and measurements. A member of a family goes in however short it is, such as the sine, because a family without it is not self-standing.
- **What stays in modules:** ports, parameter ranges and the mapping from a knob to a value, smoothing, panels, display values, and patching logic such as `Add`, `Logic`, `Clock`, and MIDI, along with `loom.phase.scale`, `wrap`, and `trigger`. Signal conventions stay in `loom-core`, including the pitch scale of ADR-0007, `WrapDetector`, `EdgeDetector`, and `trigger`, because they describe LoomDSP's cables and not an algorithm.
- **Band-limited shapes have two layers.** A band limit needs the phase step. A pure function takes the true step, for a caller who knows its rate, so a reset or sync jump does not read as a very high pitch. A struct works the step out from the last phase, for a phase that arrives from a cable, which is what patched phase needs.
- **Heap memory is allocated only in `new`.** An algorithm with heap memory offers an allocation-free `copy_from`, for state copy on the audio thread (ADR-0022), and does not rely on `Clone` for that.
- **The wrapper owns the guards.** An algorithm documents the range it accepts, such as a finite input and a cutoff below half the sample rate. Today's checks against NaN, infinity, and an unsafe cutoff stay in the module, because bad values come from cables. A caller reading clean files should not pay for them. Two kinds of check stay in the algorithm, because a caller cannot do them: one inside a feedback path, such as the all-pass, and one that keeps a read inside its memory, such as a delay time.
- **The wrapper owns smoothing.** `Smoothed` stays in `loom-rt`, so `loom-dsp` needs no dependency for it.
- **Samples are `f32`.** It is what cables carry (FR-SIG-01). An algorithm can use `f64` inside if it needs to. A generic sample type waits for a real user.
- **It uses `std`.** Hard rule 6 needs WebAssembly, which `std` supports, and `f32::tan` and similar functions need it. `no_std` waits for a real user.
- **The hard rules still apply.** A `tick` never allocates, locks, or does I/O. Results do not depend on block size. They are deterministic for the same build on the same target: Rust does not fix the precision of functions such as `tan` and `powf` across targets and versions, and flush-to-zero changes results too. Algorithms come from published theory or first principles (ADR-0018).
- **Denormals are the caller's job.** The crate cannot set CPU flags without `unsafe`. It documents which algorithms flush for themselves, and a caller turns on flush-to-zero where the CPU has it.
- **Each algorithm has direct tests against its theory** that do not use the module API. Its documentation cites its sources, or says it is built from first principles.
- **The constraints are enforced, not only stated.** Tests fail on a dependency that reaches users and on any allocation. Lints and `clippy.toml` ban `unsafe`, panics, locks, threads, I/O, clocks, and random hashing. Debug assertions check each documented input range, so module tests also check that wrappers guard. The specification lists them.
- **Moving an algorithm must not change the sound.** Reference renders stay identical, bit for bit. A change of sound is a separate change.
- **No publishing yet.** The crate is published to crates.io when its API is stable (FR-SDK-06). Before that, consider `#[non_exhaustive]` on public enums and output structs.

## Consequences

- **Algorithms can be tested and compared alone,** including frequency response, aliasing, and stability, and new methods can be tried beside old ones.
- **Each module with real DSP has two parts.** A small module is a little longer to read, and a new contributor learns one more place to look. `CONTRIBUTING.md` must say where each part goes.
- **The API becomes a promise once published.** Until then it can change like everything else during the early alpha.
- **New effect modules start this way,** beginning with #541. The existing candidates move in #537.
- **G6 and FR-SDK-06 gain DSP as a candidate,** and hard rule 6 and the WebAssembly check gain the crate.
- **A wider framework is not decided here.** The workspace already separates algorithms, real-time helpers (`loom-rt`), measurement (`loom-analysis`), the engine (`loom-core`), and the canvas (`loom-canvas`). Later splits get their own decisions.
