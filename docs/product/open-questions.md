# Open Questions and Risks

Each open item lists the options and a current leaning. Close an item by writing an ADR or updating the requirement it affects. Resolved items stay in the list so the reasoning is not lost.

## Resolved

| ID | Question | Decision | Where recorded |
|---|---|---|---|
| OQ-01 | Desktop first or browser first? | Desktop first. The browser is an optional later target (P5). | Vision, roadmap |
| OQ-03 | Stereo on every cable? | No. Channel count is inferred per cable. Stereo flows where audio needs it. | FR-SIG-07, ADR-0007 |
| OQ-04 | Pitch and value conventions | Our own documented conventions, following the Eurorack and Bitwig Grid idea of "patch anything to anything". | FR-SIG-03, 05, ADR-0007 |
| OQ-05 | Patch file format | RON, because it keeps Serde's full data model (enums, structs, tuples). | FR-FILE-01, ADR-0008 |
| OQ-06 | VST3 licensing | Steinberg released VST 3 SDK 3.8.0 under the MIT licence (October 2025). The `vst3` crate is MIT/Apache-2.0 and now ships generated bindings. The older `vst3-sys` crate is GPLv3 and is excluded. | FR-NATIVE-02, ADR-0006 |
| OQ-07 | Plugin wrapper framework | No nih-plug (or its fork nice-plug), because of their contribution policies. Use `clack` (MIT/Apache-2.0) for CLAP. Build our own VST3 adapter on the `vst3` crate and publish it as a crate. | FR-NATIVE-01, 02, FR-SDK-06 |
| OQ-08 | Exported plugin licence | The runtime is Apache-2.0, with its licence and NOTICE included in every export. The patch author chooses the licence for their patch. | FR-NATIVE-09 |
| OQ-09 | Block vs. per-sample processing | Hybrid: blocks for acyclic parts, per sample inside feedback loops. Validate with benchmarks in P0. | ADR-0002, NFR-PERF-08 |
| OQ-10 | Project name | "LoomDSP" appears to be free (informal search). Do a formal trademark check before a 1.0 release. | |

## Open

### OQ-02. UI framework and node editor

- **Leaning:** Iced, with `iced_nodegraph` for the patching canvas. See ADR-0009.
- **To confirm in the P0 spike:** frame time with 300 nodes and live scopes; how easy it is to draw custom port shapes and cable colours; whether we can build the panel editor with the same tools.
- **Fallback:** fork `iced_nodegraph`, or write our own canvas widget on Iced's wgpu renderer.

### OQ-11. Desktop audio and MIDI I/O

- `cpal` is the standard Rust crate for audio I/O. It is Apache-2.0 licensed, so it passes our licence rules. But it lives in the RustAudio organisation, whose community AI policy restricts AI-generated contributions. We could use it, but probably could not send it AI-assisted fixes.
- **Options:**
  1. Use `cpal` as a dependency behind our own host-adapter trait. Accept that we may not contribute upstream.
  2. Write our own audio backends (CoreAudio, WASAPI, ALSA/PipeWire/JACK).
  3. Use `cpal` now, and replace it per platform later if we hit limits.
- **Leaning:** option 3. Writing audio backends for three platforms is a large project, and not where LoomDSP adds value early on. The adapter trait (NFR-PORT-02) keeps it replaceable.
- Same review needed for MIDI I/O (`midir`, which is outside the RustAudio organisation; check its licence and contribution policy).

### OQ-12. Maturity of `iced_nodegraph`

- It fits our needs on paper: typed pins, nested graphs, GPU-rendered cables, a 500-node stress test, MIT licence. Its repository also includes AI agent context files, which suggests it is open to AI-assisted work.
- But it is young and small (about 22 GitHub stars, a small team). It targets a specific Iced version.
- **Options:** depend on it directly; depend on it and contribute upstream; fork it into a LoomDSP crate.
- **Leaning:** depend on it, contribute upstream, and wrap it behind our own canvas API so a fork stays possible.

### OQ-13. Embedding Iced in plugin windows

- Plugins draw inside a window owned by the DAW. Iced normally creates its own window through `winit`.
- `nice-plug-iced` shows this is possible, but it sits inside RustAudio and its AI policy.
- **Action:** research before P3. A small "embed Iced in a parent window" crate could be another published primitive (FR-SDK-06).

### OQ-14. Iced version churn

- Iced is pre-1.0 and has breaking changes between releases. `iced_nodegraph` must keep up.
- **Leaning:** pin versions. Upgrade once per Iced release, in a dedicated issue.

## Risks

| ID | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R-01 | UI work is much larger than DSP work, and the project stalls on the canvas and panel editor. | High | High | UI spike in P0. Reuse `iced_nodegraph`. Ship the headless engine and CLI early so progress stays visible. |
| R-02 | Per-sample feedback regions are too slow for large patches. | Medium | High | Benchmark in P0 (NFR-PERF-08). Limit per-sample mode to cycle regions only. |
| R-04 | A dependency's licence blocks permissive plugin export. | Medium | High | `cargo-deny` in CI from P0 with an allow list (ADR-0006). |
| R-05 | Scope creep toward a full DAW. | Medium | Medium | Non-goals in the vision document. Review new requests against them. |
| R-06 | Single maintainer, limited time. | High | High | Small phases with usable results. Public backlog with contributor labels. AI-assisted development. |
| R-07 | macOS signing and notarisation makes export hard for non-developers. | High | Medium | Document the process. Consider unsigned builds with clear instructions. |
| R-08 | Building our own primitives (VST3 adapter, GUI embedding) adds maintenance load. | High | Medium | Only build what has no acceptable alternative. Keep each crate small, documented, and tested so others can help maintain it. |
| R-09 | A key young dependency (`iced_nodegraph`) is abandoned. | Medium | Medium | Wrap behind our own API. Fork if needed. |
| R-10 | Strict dependency rules leave a gap with no acceptable crate. | Medium | Medium | Licence rules are strict (Must). AI-policy preference is soft (Should). We can use a permissive crate even if we cannot contribute to it. |

## Notes on the background research

The research document was a good starting point. Points that changed during review:

1. **"Acyclic graph" vs. feedback.** The research describes the graph as a DAG and also promises single-sample feedback. Feedback needs cycles. FR-GRAPH-03 and ADR-0002 resolve this.
2. **Web Audio block sizes.** The 128-sample AudioWorklet limit only matters for the optional web target. The desktop engine works with any host buffer size.
3. **Iced vs. Vizia.** We agree with the research's lean toward Iced, for a different reason: `iced_nodegraph` now gives a GPU-rendered node editor for Iced. `iced_audio` (cited in the research) is not needed. We will build our own knobs and sliders.
4. **Performance figures** in the research come from varied sources. We will replace them with our own benchmarks in P0.
