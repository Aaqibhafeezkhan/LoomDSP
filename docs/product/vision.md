# Vision

## Summary

LoomDSP is an open-source modular audio environment written in Rust. It is a desktop application for musical experimentation and live performance. Users build instruments and effects by connecting small DSP modules on a visual canvas. They hide the wiring behind a custom front panel and play the result on stage with MIDI controllers.

Later, the same patch can be exported as a plugin (CLAP, VST3), a standalone performance app, or a Web Audio Module. Export opens the door to an ecosystem of creators who share instruments.

## The problem

Sound designers who want deep modular control face a split today:

1. **Modular environments** (Bitwig Grid, Reaktor, Max/MSP, VCV Rack) give deep control. But most are closed source, tied to one host, or hard to turn into a clean, shareable instrument.
2. **Plugin frameworks** (JUCE, iPlug2, Faust, Rust plugin frameworks) produce distributable plugins. But they require writing code, and the design loop is slow compared to patching.
3. **Live performance** with modular software is fragile. Complex patches cause CPU spikes, and editing during a show risks dropouts.

## The vision

> Patch it, play it live, then share it. All from one open-source tool.

A musician should be able to start from an empty canvas, build a synth from primitives, give it a tidy panel with five knobs, map those knobs to a MIDI controller, and play it on stage without worry. When the instrument is ready, they can export it so others can play it too.

## Priorities

1. **A working instrument for live performance.** Patching, MIDI controllers, front panels, presets, and stable audio come first.
2. **Encapsulation and panels.** These make large patches manageable and playable.
3. **Export.** CLAP, VST3, standalone, and WAM2 export come after the live instrument works. They matter for the ecosystem, but they depend on a solid core.

## Product principles

Each principle includes its reason, so we can judge trade-offs later.

1. **Audio never glitches because of the UI.** Editing a patch during a performance must be safe. Reason: LoomDSP is a live instrument. One dropout on stage destroys trust.
2. **Patch anything to anything.** Signals share one numeric format, one rate, and normalised ranges, in the spirit of Eurorack and Bitwig Grid. Types guide the user with colour but never block a connection. Reason: the creative value of modular comes from unexpected connections.
3. **Encapsulation is the main tool for managing complexity.** Any selection can collapse into a sub-patch with its own ports and panel. Reason: large patches become unreadable fast.
4. **The panel is a product, not a debug view.** Front panels should look finished enough to perform with and to ship. Reason: a performer needs clear controls, and export is only useful if the result is pleasant to use.
5. **Rust end to end.** Engine, UI, file format, and export tooling are all Rust. Reason: one language and one type system across the whole stack. No bridge to JavaScript, no duplicated data model. See ADR-0005.
6. **Open standards, without being limited by them.** We use open formats (CLAP, VST3 now that its SDK is MIT, WAM2, WebGPU via wgpu, WASM) where they help. We do not shape the core architecture around the browser. Reason: open standards help the ecosystem, but the desktop live instrument is the main product.
7. **Patches are data, not code.** Loading a patch never runs arbitrary code. Reason: users will share patches. Safe sharing needs this guarantee.
8. **Permissive and AI-friendly.** Apache-2.0 code, only permissively licensed dependencies, and a preference for upstream communities that welcome AI-assisted contributions. Where no suitable crate exists, we build the primitive ourselves and publish it as a crate. See ADR-0006. Reason: LoomDSP is built with heavy AI assistance, and we want to give back to the projects we depend on.

## Goals (version 1.0)

- G1. Build instruments and effects from a core library of primitive modules, with sample-accurate feedback.
- G2. Play patches live with MIDI controllers, presets, and a locked performance mode, with no dropouts.
- G3. Collapse any part of a patch into a reusable sub-patch with its own front panel.
- G4. Export a patch as a CLAP plugin, a VST3 plugin, and a standalone performance app.
- G5. Provide a documented module SDK so others can add DSP modules in Rust.
- G6. Publish reusable primitives (for example the VST3 adapter) as standalone crates.

Stretch goal: run in the browser and export to WAM2, if the UI framework and WASM build perform well there. The strongest case is education: a teacher shares a link, and students start patching with no install.

## Non-goals (version 1.0)

- **A full DAW.** No arrangement timeline, no multitrack recording.
- **The browser as the primary interface.** Browser support is optional (see stretch goal).
- **A JavaScript or webview UI.** See ADR-0005.
- **A general text-based DSP language.** We may import Faust later.
- **Running third-party plugins inside a patch.** This adds a large surface for crashes and licensing issues.
- **Cloud accounts, a hosted marketplace, or collaboration servers.** Sharing works through files first.
- **Mobile apps.**
- **Video or visual synthesis.**

## Success measures

These are first proposals. We should revisit them after Phase 1.

| Measure | Target for 1.0 |
|---|---|
| Time for a new user to build a playable subtractive synth from primitives, following the tutorial | Under 20 minutes |
| Reference patch (about 100 modules, 8 voices) at 48 kHz, 128-sample buffer | Under 25% of one core on the reference machine |
| Live soak test: 1 hour of playing, preset changes, and patch edits | Zero dropouts, no memory growth |
| MIDI controller latency (input event to audible change) | One audio buffer plus driver latency, no extra |
| Exported CLAP plugin | Passes `clap-validator`. Loads in Bitwig and REAPER |
| Exported VST3 plugin | Passes the Steinberg validator. Loads in at least two DAWs |
| Sound parity | Offline render of a patch matches between the app and exported plugins within a defined tolerance |
| Community | At least 3 modules contributed by non-core contributors. At least one published crate used by another project |

## How we build it

- **AI-assisted development.** Planning and implementation lean heavily on AI tools. Docs, ADRs, and clear requirement IDs help both people and AI agents work from the same context.
- **Open backlog.** A public GitHub project with milestones per roadmap phase, and labels such as `good first issue`, `help wanted`, and `hacktoberfest` to welcome new contributors.
- **Small, usable phases.** Each phase ends with something people can try. This keeps momentum and reduces burnout risk.

## Prior art and inspiration

| Project | What we learn from it |
|---|---|
| Bitwig Grid | Polymorphic, colour-coded signals that all interoperate; phase-driven design; modulation built into every parameter |
| Eurorack | Patch anything to anything; voltage conventions such as 1V per octave |
| NI Reaktor | Deep encapsulation, polished front panels |
| Max/MSP and RNBO | Patch-then-export workflow |
| Plogue Bidule | Plain, reliable graph patching for performance |
| VCV Rack and Cardinal | Open module ecosystem |
| Faust | Whole-graph compilation for sample-accurate feedback |
| HexoDSP, FunDSP | Rust DSP building blocks and graph design |
| Elementary Audio | Graph diffing and reconciliation on the control thread |
