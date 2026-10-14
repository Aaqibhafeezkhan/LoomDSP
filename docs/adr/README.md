# Architecture Decision Records

An ADR records one important technical decision: the context, the choice, and its consequences. ADRs are short and never deleted. If a decision changes, write a new ADR that supersedes the old one.

Why we use them: contributors (human and AI) need to know *why* the code looks the way it does, not just *what* it does. Without a record, old debates restart and good reasons get lost.

## Status values

- **Proposed**: under discussion.
- **Accepted**: in effect.
- **Deferred**: valid, but only applies to work we have postponed.
- **Superseded by ADR-NNNN**: replaced.
- **Rejected**: considered and declined (kept for the record).

## Index

| ADR | Title | Status |
|---|---|---|
| [0000](0000-template.md) | Template | n/a |
| [0001](0001-lock-free-control-audio-boundary.md) | Lock-free boundary between control and audio threads | Accepted |
| [0002](0002-graph-execution-and-feedback.md) | Graph execution model and sample-accurate feedback | Accepted |
| [0003](0003-single-worklet-web-runtime.md) | One WASM instance per patch in a single AudioWorklet | Deferred (web target) |
| [0004](0004-export-as-runtime-plus-patch-data.md) | Export plugins as a prebuilt runtime plus patch data | Proposed |
| [0005](0005-rust-end-to-end.md) | Rust end to end | Accepted |
| [0006](0006-dependency-and-licence-policy.md) | Dependency and licence policy | Accepted |
| [0007](0007-polymorphic-signal-model.md) | Polymorphic signal model | Proposed |
| [0008](0008-patch-format-ron.md) | Patch format in RON | Accepted |
| [0009](0009-ui-stack-iced.md) | UI stack: Iced (canvas decided in ADR-0010) | Accepted in part |
| [0010](0010-own-patch-canvas.md) | Our own patch canvas, built as an Iced widget | Proposed |
| [0011](0011-everything-is-a-module.md) | Everything is a module | Proposed |
| [0012](0012-arrangement-from-phase-driven-primitives.md) | Arrangement from phase-driven primitives | Proposed |
