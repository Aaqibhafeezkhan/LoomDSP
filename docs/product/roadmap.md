# Roadmap

The roadmap is split into phases. Each phase ends with something usable and has clear exit criteria. No dates yet. We should estimate after Phase 0 shows how fast the core comes together.

## Overview

| Phase | Name | Main outcome |
|---|---|---|
| P0 | Engine foundations | Headless engine that plays a patch file with live MIDI |
| P1 | Desktop patcher and live basics | Build and play a synth from primitives, with MIDI controllers |
| P2 | Encapsulation, panels, performance | Sub-patches, front panels, polyphony, performance mode, set lists |
| P3 | Native export | CLAP, VST3, and standalone export |
| P4 | Ecosystem | Module SDK, published crates, advanced modules |
| P5 | Optional web target | Browser app and WAM2 export, if performance allows |

Why this order:

1. The engine and its real-time rules are the hardest part to change later, so they come first.
2. Live performance is the main purpose of the app. MIDI controller support starts in P1, not after export.
3. Export depends on a stable engine, patch format, and panel system. Building it earlier would mean rebuilding it.
4. The web target is optional. The engine core stays WASM-compatible (NFR-PORT-01) so the option stays open at low cost.

---

## P0. Engine foundations

**Goal:** prove the core architecture with no UI.

Scope:
- Cargo workspace layout, for example `loom-core` (engine), `loom-modules`, `loom-patch` (RON format), `loom-rt` (real-time queues, a candidate for its own crate), `loom-host-desktop` (audio and MIDI I/O), `loom-cli`.
- Module trait with block and per-sample entry points (FR-SDK-01).
- Runtime graph with atomic schedule swap (FR-GRAPH-01, 02, NFR-RT-05).
- Feedback regions run per sample (ADR-0002), with benchmarks (NFR-PERF-08).
- SPSC queues for control and telemetry (NFR-RT-04).
- Patch format v0 in RON, with version field (FR-FILE-01, 02, 04).
- About 10 modules: phase generator, sine, saw, SVF, ADSR, add, multiply, one-sample delay, MIDI note in, audio out.
- Offline renderer (FR-GRAPH-06).
- CLI: `loom play patch.ron`, `loom render patch.ron out.wav`.
- CI: tests, `assert_no_alloc`, WASM build check, `cargo-deny` licence check.
- UI spike (OQ-02): Iced with `iced_nodegraph`, 100 nodes, cables, pan and zoom, a live scope.
- Repository basics: `CONTRIBUTING.md`, AI contribution policy, agent context file, issue templates.

Exit criteria:
- [ ] A patch file plays through the default audio device with live MIDI input.
- [ ] Adding and removing modules from a test script while audio plays causes zero dropouts over 10 minutes.
- [ ] Feedback benchmark results recorded. ADR-0002 confirmed or revised.
- [ ] Offline renders are bit-identical across runs.
- [ ] UI spike results recorded. ADR-0009 accepted or revised.
- [ ] NFR *(proposal)* numbers reviewed against real benchmarks.

## P1. Desktop patcher and live basics

**Goal:** a musician can build a synth from primitives and play it with a MIDI controller.

Scope:
- Desktop app with canvas, module browser, cables, undo/redo (FR-CANVAS-01 to 06).
- Polymorphic signals and colours, inferred channel count (FR-SIG-01 to 07).
- Feedback with automatic single-sample delay (FR-GRAPH-03, 04).
- Built-in modulation on parameters (FR-MOD-01, 03, 04, 05).
- Phase 1 module library (FR-LIB-01 to 07, 11).
- Scope probe and cable activity (FR-VIS-01, 02).
- Audio and MIDI device setup, hot-plugging, audio input (FR-APP-01, 03, 04, FR-MIDI-08).
- MIDI learn, panic, CPU meter (FR-MIDI-02, FR-PERF-04, 05).

Exit criteria:
- [ ] Tutorial: build a subtractive synth from primitives in under 20 minutes (tested with 3 people who did not write the tutorial).
- [ ] Play that synth with a MIDI keyboard and map 4 knobs with MIDI learn.
- [ ] An FM feedback patch sounds the same at buffer sizes 32 and 1024.
- [ ] Unplugging and replugging the MIDI controller during playback works without a restart.
- [ ] UI holds 60 fps with 300 modules.

## P2. Encapsulation, panels, performance

**Goal:** complex instruments stay manageable and are ready for the stage.

Scope:
- Sub-patches, nesting, exposed parameters, user library (FR-ENCAP-*).
- Polyphony and voice allocation (FR-SIG-08, FR-VOICE-*).
- Front panel editor and widgets (FR-PANEL-*).
- Performance mode, presets, set lists, layers (FR-PERF-01, 02, 06, 07).
- Per-device controller mappings, MIDI clock, MIDI out, MPE, pickup modes (FR-MIDI-03, 04, 06, 07, 09).
- Effects and sequencing modules (FR-LIB-08, 09).

Exit criteria:
- [ ] An 8-voice polysynth built from primitives, with a 6-knob panel, runs within the CPU budget.
- [ ] A 30-minute test set with 4 patches, preset changes, and external MIDI clock runs with zero dropouts.
- [ ] One-hour live soak test with edits during playback: zero dropouts, no memory growth.
- [ ] First real live performance with LoomDSP.

## P3. Native export

**Goal:** patches leave LoomDSP as plugins and standalone apps.

Scope:
- Prebuilt runtime that loads embedded patch data (ADR-0004).
- CLAP export on top of `clack` (FR-NATIVE-01).
- Our own VST3 adapter on top of the MIT-licensed SDK bindings, as a separate crate (FR-NATIVE-02, FR-SDK-06).
- Plugin GUI embedding for Iced in a host window (OQ-13).
- Standalone app export (FR-NATIVE-08).
- LoomDSP host plugin (FR-NATIVE-06).
- Latency reporting, licence notices (FR-GRAPH-08, FR-NATIVE-09).
- macOS code signing and notarisation guidance.

Exit criteria:
- [ ] Exported CLAP passes `clap-validator`. Loads, automates, and restores state in Bitwig and REAPER.
- [ ] Exported VST3 passes the Steinberg validator. Loads in at least two DAWs.
- [ ] Sound parity test between the app and both plugin formats passes.

## P4. Ecosystem

**Goal:** others extend LoomDSP and reuse its parts.

Scope:
- Module SDK release, templates, contributor guide (FR-SDK-02 to 05).
- Publish stable crates to crates.io (FR-SDK-06).
- Patch format documentation (FR-FILE-03).
- Spectral modules, preset morphing, OSC (FR-LIB-10, FR-PERF-03, FR-MIDI-05).

## P5. Optional web target

**Goal:** run in the browser, only if it can match the desktop experience. The main driver is education: students open a link and start patching with no install (persona P5, Sanna).

Go/no-go check before starting:
- Does the chosen UI stack (Iced on wgpu/WebGPU) run the canvas at 60 fps in current browsers?
- Is the WASM engine fast enough for the reference patch inside an AudioWorklet?
- Does someone in the community want to own this work?

Scope if yes: browser app and open-from-URL (FR-APP-02, 05), WAM2 export (FR-WAM-*), web runtime rules (NFR-WEB-*), ADR-0003.

---

## Project operations

How we keep the work visible and open to contributors:

- **GitHub Project board** with one milestone per phase (P0 to P5).
- **Issues reference requirement IDs** (for example "Implements FR-MIDI-02"). This links code to the reason it exists.
- **Labels:**
  - Type: `bug`, `feature`, `docs`, `research`, `adr`.
  - Area: `area:engine`, `area:ui`, `area:modules`, `area:midi`, `area:export`, `area:crates`.
  - Contributor: `good first issue`, `help wanted`, `hacktoberfest`.
  - Priority: `must`, `should`, `could`, matching the requirement priority.
- **Good first issues** come mostly from the module library (FR-LIB). Each module is small, self-contained, and testable with the SDK harness.
- **Hacktoberfest:** opt the repository in with the `hacktoberfest` topic and label suitable issues each October.
