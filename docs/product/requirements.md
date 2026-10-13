# Functional Requirements

Priorities: **Must**, **Should**, **Could**, **Won't (now)**. Phases refer to the [roadmap](roadmap.md):

- P0 Engine foundations
- P1 Desktop patcher and live basics
- P2 Encapsulation, panels, performance
- P3 Native export (CLAP, VST3, standalone)
- P4 Ecosystem
- P5 Optional web target

Each requirement has a short rationale. If the rationale stops being true, the requirement should be reviewed.

---

## 1. Signal model (`SIG`)

See ADR-0007 for the full signal model.

| ID | Requirement | Priority | Phase | Rationale |
|---|---|---|---|---|
| FR-SIG-01 | All cables carry 32-bit float samples at the engine sample rate. There is no separate control rate. | Must | P0 | Any output can drive any input without conversion. This is the basis of free patching. |
| FR-SIG-02 | Every port has a signal type: Audio, Modulation, Pitch, Phase, Gate. Each type has a colour and a documented, normalised value range. | Must | P1 | Types help users read a patch and let modules pick sensible defaults. |
| FR-SIG-03 | Signals are polymorphic: any output connects to any input with no conversion module. Ranges are chosen so that cross-type connections give musically useful results. | Must | P1 | "Patch anything to anything", as in Eurorack and Bitwig Grid. |
| FR-SIG-04 | Phase signals wrap to the range 0.0 to 1.0. Oscillators and sequencers can be driven by an external phase input. | Must | P1 | Separating timing from wave shape enables phase distortion, sync, and step indexing with one concept. |
| FR-SIG-05 | Pitch signals use LoomDSP's own documented scale (proposal: 0.1 per octave, 0.0 at a reference note). Helpers convert to and from Hz and MIDI note numbers. | Must | P1 | 0.1 per octave keeps 20 octaves inside the normalised -1 to 1 range, so pitch can modulate anything and anything can modulate pitch. |
| FR-SIG-06 | Gate signals treat values above 0.5 as high. | Must | P1 | Simple, documented threshold. Any signal can act as a trigger. |
| FR-SIG-07 | Channel count (mono or stereo) is inferred per cable when the graph is built. Stereo flows only where a stereo source exists. Mono inputs receiving stereo are handled per module (sum or process both channels). | Must | P1 | Stereo matters mainly for audio. Computing two channels on every modulation cable wastes CPU. |
| FR-SIG-08 | Cables can carry a polyphonic bundle (one value per active voice). | Must | P2 | Polyphony is required for most instruments. |

## 2. DSP graph and engine (`GRAPH`)

| ID | Requirement | Priority | Phase | Rationale |
|---|---|---|---|---|
| FR-GRAPH-01 | The engine runs a directed graph of modules that can change at runtime (add, remove, connect, disconnect) while audio plays. | Must | P0 | Live patching is the main interaction. |
| FR-GRAPH-02 | Graph changes are applied at a buffer boundary, with no audible dropout. | Must | P0 | Edits during performance must be safe. |
| FR-GRAPH-03 | The graph may contain cycles. Each cycle must pass through at least one single-sample delay. The engine inserts one automatically if the user does not, and shows where. | Must | P1 | Feedback is essential for FM, physical models, and filters. See ADR-0002. |
| FR-GRAPH-04 | Feedback loops are sample-accurate (one-sample delay), not one-buffer. | Must | P1 | Buffer-size feedback changes the sound when the buffer size changes. |
| FR-GRAPH-05 | New or removed connections fade over a short time (for example 5 ms) to avoid clicks. | Should | P1 | Hard switches create clicks on stage. |
| FR-GRAPH-06 | The engine can render offline, faster than real time, to a file or buffer. | Must | P0 | Needed for tests, sound parity checks, and bouncing. |
| FR-GRAPH-07 | The engine supports sample rates from 44.1 kHz to 192 kHz and any host buffer size. | Must | P0 | Plugins and audio interfaces vary. |
| FR-GRAPH-08 | Modules can report their latency. The engine sums path latency and reports total latency to the host. | Should | P3 | Hosts need it for delay compensation. |
| FR-GRAPH-09 | Modules that are not connected to an output are not processed. | Should | P1 | Saves CPU in large patches. |

## 3. Polyphony and voices (`VOICE`)

| ID | Requirement | Priority | Phase | Rationale |
|---|---|---|---|---|
| FR-VOICE-01 | A patch or sub-patch can be marked polyphonic with a voice count. | Must | P2 | Standard instrument need. |
| FR-VOICE-02 | Voice allocation supports at least oldest-steal, quietest-steal, and mono/legato modes. | Must | P2 | Different instruments need different behaviour. |
| FR-VOICE-03 | Modules exist to merge voices into one signal and to send one signal to all voices. | Must | P2 | Users need to mix voices and share global modulation. |
| FR-VOICE-04 | Per-voice values for MIDI note, velocity, pressure, and MPE dimensions are available as signals. | Should | P2 | Expressive control is a key reason to build custom instruments. |

## 4. Modulation (`MOD`)

| ID | Requirement | Priority | Phase | Rationale |
|---|---|---|---|---|
| FR-MOD-01 | Every module parameter has a base value plus zero or more modulation sources, each with its own depth. | Must | P1 | Removes the need for many attenuator and mixer modules. Keeps the canvas readable. |
| FR-MOD-02 | Modulation depth can itself be modulated (for example velocity scales envelope depth). | Should | P2 | Common expressive pattern. |
| FR-MOD-03 | Modulation is evaluated per sample and the final value is clamped to the parameter range. | Must | P1 | Prevents unstable filters and keeps behaviour predictable. |
| FR-MOD-04 | Parameter knobs show the live modulated value and the modulation range. | Must | P1 | Users must see what modulation does. |
| FR-MOD-05 | Parameter changes from the UI and MIDI are smoothed to avoid zipper noise. | Must | P1 | Audible artefacts are unacceptable on stage. |

## 5. Module library (`LIB`)

The first library should be small and composable. Phase 1 needs enough to build a full subtractive synth from primitives.

| ID | Requirement | Priority | Phase | Rationale |
|---|---|---|---|---|
| FR-LIB-01 | Phase: phase generator, phase scaler, phase wrap/fold, phase reset. | Must | P1 | Core of the phase-driven model. |
| FR-LIB-02 | Oscillators/shapers: sine, saw, pulse, triangle (band-limited), wavetable reader, noise. | Must | P1 | Basic sound sources. |
| FR-LIB-03 | Filters: state variable filter (LP/HP/BP/notch), one-pole, ladder-style lowpass. | Must | P1 | Basic subtractive synthesis. |
| FR-LIB-04 | Envelopes and LFOs: ADSR, AD, LFO (phase driven), sample and hold, slew. | Must | P1 | Basic modulation. |
| FR-LIB-05 | Math and logic: add, multiply, mix, crossfade, clamp, scale/offset, compare, AND/OR/NOT, flip-flop, counter. | Must | P1 | Building blocks for anything else. |
| FR-LIB-06 | Delay and time: one-sample delay, delay line with interpolation, allpass. | Must | P1 | Needed for feedback, effects, and physical models. |
| FR-LIB-07 | I/O: audio in/out, MIDI note/CC/pitch bend, transport and clock (tempo, position). | Must | P1 | Connects the patch to the outside world. |
| FR-LIB-08 | Sequencing: step sequencer, clock divider, Euclidean generator. | Should | P2 | Common in performance patches. |
| FR-LIB-09 | Effects: reverb, chorus, distortion/waveshaper, compressor. | Should | P2 | Users expect usable effects out of the box. |
| FR-LIB-10 | Spectral: FFT-based modules. | Could | P4 | Higher complexity and latency. |
| FR-LIB-11 | Every built-in module has a help page with a short description, port list, and example patch. | Must | P1 | Discoverability is essential for learners. |

## 6. Encapsulation (`ENCAP`)

| ID | Requirement | Priority | Phase | Rationale |
|---|---|---|---|---|
| FR-ENCAP-01 | Any selection of modules can be collapsed into a sub-patch. Connections crossing the boundary become ports. | Must | P2 | Main tool for managing complexity. |
| FR-ENCAP-02 | Sub-patches can nest to any depth. | Must | P2 | Real instruments have several levels (voice, oscillator, filter). |
| FR-ENCAP-03 | A sub-patch can expose internal parameters as its own parameters, with a new name and range. | Must | P2 | Lets a sub-patch behave like a built-in module. |
| FR-ENCAP-04 | Users can open, inspect, and edit any sub-patch. | Must | P2 | Learning and debugging. |
| FR-ENCAP-05 | A sub-patch can be saved to the user library and reused in other patches. | Must | P2 | Reuse is the point of encapsulation. |
| FR-ENCAP-06 | Library sub-patches can be linked (updates propagate) or copied (independent). The user chooses. | Should | P2 | Both workflows are common. |
| FR-ENCAP-07 | Encapsulation has no runtime cost. The engine flattens sub-patches before execution. | Must | P2 | Users should not trade clarity for CPU. |

## 7. Patching canvas (`CANVAS`)

| ID | Requirement | Priority | Phase | Rationale |
|---|---|---|---|---|
| FR-CANVAS-01 | A zoomable, pannable canvas where users add, move, and delete modules. | Must | P1 | Basic interaction. |
| FR-CANVAS-02 | Connect ports by dragging. Cables are coloured by signal type. | Must | P1 | Basic interaction. |
| FR-CANVAS-03 | Module browser with search and categories. Add modules by typing a name at the cursor. | Must | P1 | Fast patching for experienced users. |
| FR-CANVAS-04 | Undo and redo for every edit, including parameter changes. | Must | P1 | Users experiment. Mistakes must be cheap. |
| FR-CANVAS-05 | Copy, paste, and duplicate, including across patches. | Must | P1 | Basic editing. |
| FR-CANVAS-06 | Keyboard navigation and shortcuts for common actions. | Should | P1 | Speed, and part of accessibility. |
| FR-CANVAS-07 | Comments and colour groups on the canvas. | Should | P2 | Documentation inside the patch. |
| FR-CANVAS-08 | Insert a module onto an existing cable by dropping it there. | Could | P2 | Common workflow in Bitwig Grid. |

## 8. Visual feedback (`VIS`)

| ID | Requirement | Priority | Phase | Rationale |
|---|---|---|---|---|
| FR-VIS-01 | Live value display on ports and cables (for example activity glow or small meter). | Should | P1 | Shows where signal flows. |
| FR-VIS-02 | Probe any cable with an oscilloscope view. | Must | P1 | Core debugging tool. |
| FR-VIS-03 | Spectrum analyser and level meter modules. | Should | P2 | Common sound design need. |
| FR-VIS-04 | Visual data reaches the UI without blocking the audio thread and without allocation. | Must | P1 | See NFR-RT-01. |

## 9. Front panels (`PANEL`)

| ID | Requirement | Priority | Phase | Rationale |
|---|---|---|---|---|
| FR-PANEL-01 | Each patch and sub-patch can have a front panel, separate from the cable view. | Must | P2 | Hides complexity. Core of performance mode. |
| FR-PANEL-02 | Panel widgets: knob, horizontal/vertical slider, toggle, button, selector, XY pad, label, scope, meter. | Must | P2 | Covers common instrument controls. |
| FR-PANEL-03 | Widgets bind to exposed parameters. One parameter can have several widgets. | Must | P2 | Flexible layout. |
| FR-PANEL-04 | Widgets support value mappings: linear, logarithmic frequency, decibels, stepped/enumerated. | Must | P2 | Controls must feel right. A linear cutoff knob is unusable. |
| FR-PANEL-05 | Panel editor with grid snapping, alignment, grouping, and resizing. | Must | P2 | Panels must look tidy without pixel-pushing. |
| FR-PANEL-06 | Themes: colours, fonts, and widget styles. Light and dark defaults. | Should | P2 | Stage visibility and branding for exports. |
| FR-PANEL-07 | Vector rendering that scales cleanly on high-DPI screens. Custom images (SVG, PNG) as backgrounds or widget skins. | Should | P2 | Professional look. |
| FR-PANEL-08 | A sub-patch panel can be embedded in its parent panel. | Should | P2 | Panels compose the same way patches do. |

## 10. Live performance (`PERF`)

Live performance is the top priority after the core patcher works.

| ID | Requirement | Priority | Phase | Rationale |
|---|---|---|---|---|
| FR-PERF-01 | Performance mode: shows only the panel, locks editing, uses large controls. | Must | P2 | Prevents accidental edits on stage. |
| FR-PERF-02 | Presets (snapshots of parameter values) with click-free switching. | Must | P2 | Song changes during a set. |
| FR-PERF-03 | Preset morphing between two snapshots. | Could | P4 | Expressive performance tool. |
| FR-PERF-04 | CPU load meter and dropout counter visible at all times. | Must | P1 | Performers need warning before problems become audible. |
| FR-PERF-05 | A "panic" action silences all voices and resets stuck notes immediately. Mappable to a MIDI control. | Must | P1 | Stuck notes happen on stage. Recovery must be one press. |
| FR-PERF-06 | A set list: an ordered list of patches and presets. Switching between them is click-free and loads in the background. | Should | P2 | A live set uses several instruments. |
| FR-PERF-07 | Several patches can run at once (layers or splits across the keyboard). | Should | P2 | Common live setup, for example bass on the left hand and pad on the right. |

## 11. MIDI and control input (`MIDI`)

| ID | Requirement | Priority | Phase | Rationale |
|---|---|---|---|---|
| FR-MIDI-01 | Receive MIDI notes, CC, pitch bend, and aftertouch from hardware (standalone) or host (plugin). | Must | P1 | Basic instrument need. |
| FR-MIDI-02 | MIDI learn for any parameter: touch a control, move a knob, done. | Must | P1 | Fast controller mapping is central to live use. |
| FR-MIDI-03 | MPE support. | Should | P2 | Expressive controllers are common among the target audience. |
| FR-MIDI-04 | Send MIDI out from a patch (notes, CC, clock). | Should | P2 | Live rigs drive other gear from sequencers. |
| FR-MIDI-05 | OSC input. | Could | P4 | Common in live coding and installation work. |
| FR-MIDI-06 | Controller mappings are saved per device, separate from patches, and restored when the device connects. | Must | P2 | The same controller is used across many patches. Remapping every time is slow. |
| FR-MIDI-07 | Sync tempo to incoming MIDI clock, and send MIDI clock. | Should | P2 | Playing with drum machines and other performers. |
| FR-MIDI-08 | MIDI devices can connect and disconnect while the app runs, without a restart. | Must | P1 | Cables get unplugged on stage. |
| FR-MIDI-09 | Mapping modes for relative encoders, value pickup (soft takeover), and value scaling/inversion. | Should | P2 | Prevents parameter jumps when a knob position does not match the current value. |

## 12. Desktop application (`APP`)

| ID | Requirement | Priority | Phase | Rationale |
|---|---|---|---|---|
| FR-APP-01 | Desktop app for macOS, Windows, and Linux with audio device and MIDI device selection. | Must | P1 | The main product. |
| FR-APP-02 | Browser version of the app, with patching, example patches, and sub-patch inspection. | Could | P5 | Lets educators share a link so students can patch with no install (persona P5). Pursue only if the UI framework and WASM build perform well in browsers. |
| FR-APP-05 | Open a patch from a URL in the browser version. | Could | P5 | Teachers share example patches as links. |
| FR-APP-03 | Audio input processing (use LoomDSP as a live effect). | Must | P1 | Effects are half of the use cases. |
| FR-APP-04 | Audio device loss (unplugged interface) is handled without a crash. The app shows a clear message and can reconnect. | Must | P1 | Hardware problems happen on stage. |

## 13. Files and patch format (`FILE`)

See ADR-0008.

| ID | Requirement | Priority | Phase | Rationale |
|---|---|---|---|---|
| FR-FILE-01 | Patches save as RON (Rusty Object Notation), a human-readable text format. | Must | P0 | Keeps Rust enums and structs exact through Serde. Works with git. |
| FR-FILE-02 | The format has a version number. Older patches load in newer versions through migrations. | Must | P0 | Users must not lose work after upgrades. |
| FR-FILE-03 | The format is documented. | Should | P2 | Enables third-party tools. |
| FR-FILE-04 | Patches reference modules by stable ID and version, not by display name. | Must | P0 | Renaming a module must not break patches. |
| FR-FILE-05 | Loading a patch with an unknown module shows a placeholder and keeps the data. | Should | P1 | Graceful handling of missing modules from newer versions. |
| FR-FILE-06 | A patch can embed assets (wavetables, samples, panel images) or reference them. | Should | P2 | Exported plugins and shared patches must be self-contained. |
| FR-FILE-07 | A JSON export of the patch format, for tools outside the Rust world. | Could | P4 | Only if third-party tools ask for it. |

## 14. Native export (`NATIVE`)

Export comes after the live instrument works.

| ID | Requirement | Priority | Phase | Rationale |
|---|---|---|---|---|
| FR-NATIVE-01 | Export a patch as a CLAP instrument or effect for macOS, Windows, and Linux. | Must | P3 | CLAP is an open, permissively licensed standard. |
| FR-NATIVE-02 | Export a patch as a VST3 plugin. | Must | P3 | VST3 has the widest DAW support. The SDK is MIT-licensed since 3.8. We build our own adapter (FR-SDK-06). |
| FR-NATIVE-03 | Exported plugins expose parameters, report latency, support state save/restore, and receive MIDI. | Must | P3 | Basic plugin contract. |
| FR-NATIVE-04 | Exported plugins show the patch front panel as their GUI. | Must | P3 | The panel is the product. |
| FR-NATIVE-05 | Export does not require a Rust toolchain on the user's machine. | Must | P3 | Most users are not developers. See ADR-0004. |
| FR-NATIVE-06 | A LoomDSP host plugin can load any patch inside a DAW. | Should | P3 | Lets users patch directly inside their DAW. |
| FR-NATIVE-07 | AU (Audio Unit) export. | Won't (now) | | Possible later via a CLAP-to-AU wrapper. |
| FR-NATIVE-08 | Export a patch as a standalone app that opens straight into its front panel. | Should | P3 | Lets performers use an instrument without learning LoomDSP. |
| FR-NATIVE-09 | Exported plugins include the Apache-2.0 licence and NOTICE for the runtime. The patch author chooses the licence for their patch. | Must | P3 | Meets Apache-2.0 attribution terms. Patch authors keep their rights. |

## 15. Web export (`WAM`), optional

Only pursued if the UI and WASM build perform well in browsers (see P5 and ADR-0003).

| ID | Requirement | Priority | Phase | Rationale |
|---|---|---|---|---|
| FR-WAM-01 | Export a patch as a WAM2 package: descriptor, JS glue, AudioWorklet processor, WASM engine, patch data, and panel GUI. | Could | P5 | Opens the browser plugin ecosystem. |
| FR-WAM-02 | The whole patch runs inside one AudioWorkletProcessor and one WASM instance. | Must (if web) | P5 | Separate Web Audio nodes add 128-sample delays and break feedback. |
| FR-WAM-03 | Exposed panel parameters become WAM2 parameters. MIDI and automation events are sample-accurate. State save/restore works. | Must (if web) | P5 | Basic WAM2 contract. |
| FR-WAM-04 | Works without cross-origin isolation, with reduced visual update quality. | Should (if web) | P5 | Many hosts cannot set COOP/COEP headers. |

## 16. Module SDK and published crates (`SDK`)

| ID | Requirement | Priority | Phase | Rationale |
|---|---|---|---|---|
| FR-SDK-01 | A public Rust trait for modules: describe ports and parameters, process a block, process one sample, reset, report latency. | Must | P0 | Built-in modules use the same API. This keeps it honest. |
| FR-SDK-02 | A test harness: render a module with given inputs and compare to reference output. | Must | P1 | Quality and regression safety. |
| FR-SDK-03 | Module template, examples, and contributor guide. | Must | P4 | Lowers the barrier for module developers. |
| FR-SDK-04 | Third-party modules are compiled into the main build. No runtime loading of native code in v1. | Must | P4 | Keeps patches safe and portable. |
| FR-SDK-05 | Import Faust DSP code as a module. | Could | P4 | Large existing library of DSP code. Check Faust's licence terms for generated code first. |
| FR-SDK-06 | Reusable primitives are separate crates with their own docs and semver, and are published to crates.io when stable. Candidates: real-time queues, VST3 adapter, CLAP adapter, plugin GUI embedding, node-graph widget extensions. | Should | P3 | Gives back to the Rust audio ecosystem and gives contributors smaller, self-contained projects. |
