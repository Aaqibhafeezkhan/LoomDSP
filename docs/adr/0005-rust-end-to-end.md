# ADR-0005: Rust end to end

- **Status:** Accepted
- **Date:** 2026-10-13
- **Related requirements:** FR-APP-01, FR-SDK-01, NFR-PORT-02, FR-FILE-01

## Context

LoomDSP has a DSP engine, a node-graph editor, a panel editor, a file format, and export tooling. The UI and engine share a large data model: modules, ports, parameters, cables, sub-patches, and panels.

Mature node-editor libraries exist for JavaScript (React Flow, Rete.js, LiteGraph). Using one would mean a webview UI talking to a Rust engine.

## Options considered

1. **Rust engine, JavaScript/TypeScript UI in a webview (for example Tauri).** Rich UI libraries. But the data model is duplicated in two languages and kept in sync by message passing. High-rate data (scopes, meters) must be serialised or shared through special memory. Plugin GUIs in webviews inside DAWs have known problems.
2. **Rust engine and Rust UI.** One type system. The editor works directly on the same patch structures that are saved to disk and compiled into the engine schedule. Fewer mature UI libraries.

## Decision

Use option 2. Engine, UI, file format, and tooling are all Rust. The UI runs on a GPU-rendered Rust framework (see ADR-0009).

## Consequences

- One data model, defined once with Serde. The editor, the file format (ADR-0008), and the engine builder share types. Refactors are checked by the compiler across the whole stack.
- AI tools and contributors work in one language and one build system.
- Telemetry (scopes, meters) moves through the real-time queues (ADR-0001) straight into the renderer, with no serialisation.
- We may need to build more UI ourselves (knobs, panel editor). This is accepted.
- A WAM2 export still needs a small amount of JavaScript glue, because the standard requires it. That glue is generated, not hand-written UI code.
