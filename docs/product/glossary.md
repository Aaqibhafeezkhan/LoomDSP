# Glossary

| Term | Meaning |
|---|---|
| **AudioWorklet** | The browser API that runs custom audio code on a dedicated audio thread. It always processes blocks of 128 frames. |
| **Block / buffer** | A group of samples processed together. Hosts choose the size (often 32 to 1024). |
| **Cable** | A connection from an output port to an input port. |
| **CLAP** | CLever Audio Plug-in. An open plugin format with a permissive licence. |
| **Copyleft** | A licence (such as GPL or AGPL) that requires derived works to use the same licence. Not allowed in LoomDSP dependencies. |
| **Control thread** | Any non-audio thread, usually the UI thread. Allowed to allocate and block. |
| **COOP / COEP** | HTTP headers that make a page "cross-origin isolated". Required for SharedArrayBuffer in browsers. |
| **Encapsulation** | Collapsing part of a patch into a sub-patch with its own ports and panel. |
| **Engine** | The Rust code that runs a patch and produces audio. The same engine runs in every target. |
| **Front panel** | The user-facing control surface of a patch or sub-patch. Separate from the cable view. |
| **Gate** | A signal used as on/off. Values above 0.5 are high. |
| **Lock-free** | Code that lets threads communicate without one thread waiting for another to release a lock. |
| **Modulation** | Changing a parameter over time with another signal (LFO, envelope, MIDI). |
| **Module** | One DSP unit on the canvas, such as an oscillator or filter. Built-in or from the SDK. |
| **Panic (MIDI)** | An action that silences all voices and clears stuck notes. Not the same as a Rust `panic!`. |
| **Patch** | A complete graph of modules, cables, sub-patches, panels, and settings. Saved as one file. |
| **Performance mode** | A locked view that shows only the front panel, for live use. |
| **Phase signal** | A ramp from 0.0 to 1.0 that wraps. It drives oscillators, LFOs, and sequencers. |
| **Pitch signal** | A signal that represents musical pitch on a fixed scale (see OQ-04). |
| **Polymorphic signal** | A signal that can connect to any input, whatever its type. Types only change colour and default ranges. |
| **Polyphony** | Playing several notes at once. Each note uses its own voice. |
| **Port** | An input or output point on a module. |
| **Real-time safe** | Code that always finishes in a bounded time. No allocation, locks, or system calls. |
| **RON** | Rusty Object Notation. A text format that maps directly to Rust data types through Serde. Used for patch files. |
| **Schedule** | The ordered list of module calls the engine runs each block. Built on a control thread. |
| **SharedArrayBuffer (SAB)** | Browser memory that the main thread and the audio thread can both read and write. |
| **SIMD** | Single Instruction, Multiple Data. One CPU instruction processes several values at once. |
| **SPSC queue** | Single-producer, single-consumer queue. One thread writes, one thread reads, no locks. |
| **Strongly connected component (SCC)** | A part of a graph where every node can reach every other node. In audio terms: a feedback loop. |
| **Sub-patch** | A patch nested inside another patch. Also called a macro or abstraction in other tools. |
| **Voice** | One copy of a polyphonic sub-patch, playing one note. |
| **VST3** | Steinberg's plugin format. Widest DAW support. The SDK is MIT-licensed since version 3.8. |
| **WAM2** | Web Audio Modules 2.0. A plugin standard for browser-based audio hosts. |
| **WASM** | WebAssembly. A portable binary format that runs at near-native speed in browsers. |
