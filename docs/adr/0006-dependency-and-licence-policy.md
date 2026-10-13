# ADR-0006: Dependency and licence policy

- **Status:** Accepted
- **Date:** 2026-10-13
- **Related requirements:** NFR-OSS-01, 02, 06, 07, FR-SDK-06, FR-NATIVE-09

## Context

LoomDSP is Apache-2.0. Exported plugins include the LoomDSP runtime and all its dependencies. A copyleft dependency would force its licence onto every plugin a user exports.

LoomDSP is also built with heavy AI assistance. Some open-source communities restrict or reject AI-assisted contributions. If we depend on them, we can use their code but may not be able to send fixes back.

## Decision

### 1. Licence rules (hard)

Allowed without review:
`MIT`, `MIT-0`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Zlib`, `BSL-1.0`, `CC0-1.0`, `Unicode-3.0`, `Unicode-DFS-2016`.

Needs a written review in the issue tracker before use:
`MPL-2.0` (file-level copyleft), and any licence not listed here.

Not allowed:
`GPL-*`, `LGPL-*`, `AGPL-*`, `SSPL`, `EUPL`, `CC-BY-SA-*`, non-commercial licences, and crates with no licence.

CI enforces this with `cargo-deny`. Example `deny.toml` section:

```toml
[licenses]
allow = [
  "MIT", "MIT-0", "Apache-2.0", "Apache-2.0 WITH LLVM-exception",
  "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib", "BSL-1.0",
  "CC0-1.0", "Unicode-3.0", "Unicode-DFS-2016",
]
confidence-threshold = 0.9
```

### 2. Community rules (soft)

When two dependencies are otherwise comparable, prefer the one whose project welcomes AI-assisted contributions. Reason: we want to contribute fixes upstream.

A project with a restrictive AI policy can still be used if it is the only reasonable option, as long as its licence passes. We wrap it behind our own trait so it stays replaceable.

### 3. Build our own, then publish

If no acceptable crate exists, we build the primitive ourselves as a separate crate in the workspace, with its own docs and tests. When stable, we publish it to crates.io under Apache-2.0 OR MIT so others can use it.

### 4. Dependency review checklist

Each new non-trivial dependency gets a short note in its pull request:

- Licence (and licences of its own dependencies).
- Maintenance: last release, open issues, number of maintainers.
- Contribution and AI policy.
- Real-time suitability, if used on the audio path (no allocation, no locks).
- WASM compatibility, if used in the engine core.

## Current decisions

| Need | Choice | Status | Notes |
|---|---|---|---|
| CLAP wrapper | `clack` | Use | MIT/Apache-2.0. Low-level, plugin and host crates. |
| VST3 bindings | `vst3` crate | Use | MIT/Apache-2.0. Steinberg's SDK is MIT since 3.8.0. |
| VST3 bindings | `vst3-sys` | Not allowed | GPLv3. |
| Plugin framework | nih-plug, nice-plug | Not used | Contribution policies conflict with our AI-assisted workflow. nice-plug follows the RustAudio community AI policy. |
| VST3 adapter | Our own crate on `vst3` | Build | Candidate for publishing (FR-SDK-06). |
| UI | Iced | Use | MIT. See ADR-0009. |
| Node editor | `iced_nodegraph` | Use, pending spike | MIT. See OQ-12. |
| Audio I/O | `cpal` | Use, with caveat | Apache-2.0. RustAudio AI policy limits upstream contributions. Behind our own host trait. See OQ-11. |
| Patch format | `serde`, `ron` | Use | MIT/Apache-2.0. |

## Consequences

- Exported plugins carry only permissive licences. Users can ship them under any licence.
- More work for us in places (VST3 adapter, plugin GUI embedding). Each such piece becomes a published crate that others can help maintain.
- The repository needs its own AI contribution policy that is welcoming and sets quality rules (NFR-OSS-07).
