# UI spike: egui + egui-snarl (comparison)

Comparison spike for [ADR-0009](../../docs/adr/0009-ui-stack-iced.md). It mirrors [spikes/ui-nodegraph](../ui-nodegraph/README.md) (Iced + iced_nodegraph) as closely as the libraries allow: the same patch loader and 300-module generator, live engine edits, typed and coloured ports, parameter sliders, and the same stats line. Both render through wgpu (Metal on macOS) with vsync.

The question: does a renderer that tessellates shapes into triangles (egui) avoid the per-pixel GPU cost we measured in iced_nodegraph's signed-distance-field renderer on a Retina display?

Throwaway code, standalone Cargo project (not in the main workspace).

## Run it

```sh
cd spikes/ui-snarl
cargo run --release -- ../../examples/patches/fm-feedback.ron
cargo run --release -- --generate 300 --no-audio
cargo run --release -- --generate 300 --no-audio --plain   # nodes without sliders
```

Controls: drag between pins to connect, right-click a pin to unplug, drag nodes to move, drag the background to pan, scroll or pinch to zoom, right-click a node to remove it.

Differences from the Iced spike:
- egui-snarl keeps its own copy of the graph, so the spike mirrors every edit into both the snarl and the engine `Controller`.
- egui-snarl starts zoomed to fit the whole graph, so all 300 modules are on screen at once. The Iced spike showed only part of the graph.
- No dashed wires: inputs with an inserted 1-sample delay are labelled "(delay)".
- The stats line shows "ui build" (time in the app's `ui()` function) and "egui CPU" (eframe's own CPU time for the last frame, including tessellation).

![FM feedback patch in the egui-snarl spike](screenshots/fm-feedback.png)

## Results

Measure on the same Mac, same display, same window size as the Iced spike:

| Run | Display | fps | frame avg | ui build | egui CPU |
|---|---|---|---|---|---|
| fm-feedback.ron (no audio) | Laptop (Retina) | | | | |
| `--generate 300 --no-audio` | Laptop (Retina) | | | | |
| `--generate 300 --no-audio --plain` | Laptop (Retina) | | | | |
| `--generate 300 --no-audio` | External | | | | |

Early sign from a CPU-only VM (software rendering, not comparable in fps): with all 300 modules on screen, `ui()` took about 11 ms and egui's CPU time about 20 ms per frame. egui lays out every node every frame (immediate mode), so its cost is on the CPU side, the opposite of iced_nodegraph.

## Conclusion (2026-10-13)

Rejected on look before the real-hardware measurement: egui's stock widgets and spacing are far from the polish we want (the Bitwig Grid is the reference), and egui-snarl keeps its own copy of the graph, which fits our `Controller` poorly. The patch canvas will be our own Iced widget ([ADR-0010](../../docs/adr/0010-own-patch-canvas.md)). Whether triangle-based drawing avoids the per-pixel cost will be measured on that canvas's first prototype instead.
