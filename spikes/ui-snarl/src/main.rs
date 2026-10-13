//! P0 UI comparison spike (ADR-0009): the LoomDSP patch canvas on egui +
//! egui-snarl, to compare with `spikes/ui-nodegraph` (Iced + iced_nodegraph).
//!
//! It mirrors the first spike as closely as the libraries allow: the same
//! patch loader and 300-module generator, live engine edits, typed and
//! coloured ports, parameter sliders, and the same stats line. Both spikes
//! render through wgpu (Metal on macOS) with vsync.
//!
//! The main difference under test: egui tessellates shapes into triangles on
//! the CPU, while iced_nodegraph shades signed distance fields per pixel on
//! the GPU.
//!
//! Run:
//!   cargo run --release -- ../../examples/patches/fm-feedback.ron
//!   cargo run --release -- --generate 300 --no-audio
//!   cargo run --release -- --generate 300 --no-audio --plain   # nodes without sliders
//!
//! Throwaway code to answer a question, not the start of the real UI.

use eframe::egui::{self, Color32, Pos2};
use egui_snarl::ui::{NodeLayout, PinInfo, PinPlacement, SnarlStyle, SnarlViewer, SnarlWidget};
use egui_snarl::{InPin, InPinId, NodeId, OutPin, Snarl};
use loom_core::{
    Controller, EngineConfig, EngineParts, InputRef, MidiEvent, MidiSender, ModuleId, OutputRef, SignalType,
    create_engine,
};
use loom_host_desktop::{AudioOptions, AudioOutput, CallbackStats, MidiConnection, RunningAudio};
use loom_patch::Patch;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct Args {
    patch: Option<PathBuf>,
    generate: Option<usize>,
    audio: bool,
    plain: bool,
}

fn parse_args() -> Args {
    let mut args = Args {
        patch: None,
        generate: None,
        audio: true,
        plain: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--generate" => args.generate = it.next().and_then(|n| n.parse().ok()),
            "--no-audio" => args.audio = false,
            "--plain" => args.plain = true,
            path => args.patch = Some(PathBuf::from(path)),
        }
    }
    if args.patch.is_none() && args.generate.is_none() {
        args.patch = Some(PathBuf::from("../../examples/patches/fm-feedback.ron"));
    }
    args
}

fn main() -> eframe::Result<()> {
    let args = parse_args();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 900.0]),
        // wgpu (Metal on macOS), like the Iced spike. Its default present
        // mode waits for vsync.
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "LoomDSP UI spike (egui-snarl)",
        options,
        Box::new(move |_cx| Ok(Box::new(App::new(args)))),
    )
}

struct App {
    controller: Controller,
    snarl: Snarl<ModuleId>,
    style: SnarlStyle,
    name: String,
    plain: bool,
    _audio: Option<RunningAudio>,
    _midi: Option<MidiConnection>,
    audio_stats: Option<Arc<CallbackStats>>,
    audio_note: String,
    midi_sender: Option<MidiSender>,
    holding: bool,
    status: String,
    frames: VecDeque<Instant>,
    frames_seen: u64,
    worst_frame: Duration,
    ui_time: Duration,
}

impl App {
    fn new(args: Args) -> Self {
        let output = if args.audio {
            AudioOutput::open(&AudioOptions::default()).map_err(|e| e.to_string())
        } else {
            Err("audio disabled".to_string())
        };
        let sample_rate = output.as_ref().map(|o| o.sample_rate() as f32).unwrap_or(48_000.0);
        let EngineParts {
            mut controller,
            engine,
            midi: sender,
        } = create_engine(loom_modules::registry(), EngineConfig::with_sample_rate(sample_rate));

        let mut positions = BTreeMap::new();
        let name = if let Some(n) = args.generate {
            generate(&mut controller, n);
            format!("Generated ({n} modules)")
        } else {
            let path = args.patch.clone().expect("patch path");
            let patch = Patch::load(&path).expect("patch loads");
            patch.load_into(&mut controller).expect("patch is valid");
            for m in &patch.modules {
                if let Some((x, y)) = m.position {
                    positions.insert(m.id, Pos2::new(x, y));
                }
            }
            patch.name.clone()
        };
        auto_layout(&controller, &mut positions);

        // Mirror the engine graph into the snarl, which keeps its own copy.
        let mut snarl = Snarl::new();
        let mut node_of: BTreeMap<ModuleId, NodeId> = BTreeMap::new();
        for (id, _) in controller.graph().nodes() {
            let pos = positions.get(&id.0).copied().unwrap_or(Pos2::ZERO);
            node_of.insert(id, snarl.insert_node(pos, id));
        }
        for (from, to) in controller.graph().cables() {
            snarl.connect(
                egui_snarl::OutPinId {
                    node: node_of[&from.module],
                    output: from.port,
                },
                InPinId {
                    node: node_of[&to.module],
                    input: to.port,
                },
            );
        }

        let mut audio_note = String::new();
        let (audio, midi, midi_sender) = match output {
            Ok(output) => {
                let (midi, sender) = match loom_host_desktop::midi_input_names() {
                    Ok(names) if !names.is_empty() => match loom_host_desktop::connect_midi(None, sender) {
                        Ok(connection) => (connection, None),
                        Err(e) => {
                            audio_note = format!("MIDI: {e}");
                            (None, None)
                        }
                    },
                    _ => (None, Some(sender)),
                };
                match output.start(engine) {
                    Ok(audio) => (Some(audio), midi, sender),
                    Err(e) => {
                        audio_note = format!("audio: {e}");
                        (None, None, None)
                    }
                }
            }
            Err(e) => {
                audio_note = format!("no audio ({e})");
                (None, None, None)
            }
        };

        let mut app = Self {
            controller,
            snarl,
            style: style(),
            name,
            plain: args.plain,
            audio_stats: audio.as_ref().map(|a| a.stats().clone()),
            _audio: audio,
            _midi: midi,
            audio_note,
            midi_sender,
            holding: false,
            status: String::new(),
            frames: VecDeque::new(),
            frames_seen: 0,
            worst_frame: Duration::ZERO,
            ui_time: Duration::ZERO,
        };
        app.status = report(&app.controller);
        app
    }

    fn record_frame(&mut self) {
        let now = Instant::now();
        self.frames_seen += 1;
        if let Some(&last) = self.frames.back()
            && self.frames_seen > 10
        {
            self.worst_frame = self.worst_frame.max(now - last);
        }
        self.frames.push_back(now);
        while self.frames.front().is_some_and(|&t| now - t > Duration::from_secs(1)) {
            self.frames.pop_front();
        }
    }

    fn stats_line(&self, graph_cpu: Option<f32>) -> String {
        let gaps: Vec<Duration> = self
            .frames
            .iter()
            .zip(self.frames.iter().skip(1))
            .map(|(a, b)| *b - *a)
            .collect();
        let avg = gaps.iter().sum::<Duration>().as_secs_f64() * 1000.0 / gaps.len().max(1) as f64;
        let max_recent = gaps.iter().max().copied().unwrap_or_default().as_secs_f64() * 1000.0;
        let audio = self.audio_stats.as_ref().map_or(self.audio_note.clone(), |s| {
            let s = s.snapshot();
            format!(
                "audio load {:.1}% (max {:.1}%), overruns {}",
                s.last_load * 100.0,
                s.max_load * 100.0,
                s.overruns
            )
        });
        let graph = self.controller.graph();
        format!(
            "{} modules, {} cables{} | {} fps | frame avg {avg:.1} ms, max {max_recent:.1} ms (1 s), worst {:.1} ms | ui build {:.2} ms | egui CPU {} | {audio}",
            graph.len(),
            graph.cables().count(),
            if self.plain { " (plain)" } else { "" },
            self.frames.len(),
            self.worst_frame.as_secs_f64() * 1000.0,
            self.ui_time.as_secs_f64() * 1000.0,
            graph_cpu.map_or("n/a".to_string(), |s| format!("{:.2} ms", s * 1000.0)),
        )
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let started = Instant::now();
        self.record_frame();
        self.controller.collect_garbage();
        // Redraw every frame, so the FPS counter measures real render cost
        // (same as the Iced spike).
        ui.ctx().request_repaint();

        let stats = self.stats_line(frame.info().cpu_usage);
        egui::Panel::bottom("stats").show(ui, |ui| {
            ui.label(stats);
            ui.label(&self.status);
        });
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(&self.name);
                if self.midi_sender.is_some() {
                    let label = if self.holding { "Release note" } else { "Hold note" };
                    if ui.button(label).clicked()
                        && let Some(sender) = &mut self.midi_sender
                    {
                        self.holding = !self.holding;
                        sender.send(if self.holding {
                            MidiEvent::note_on(0, 0, 57, 100)
                        } else {
                            MidiEvent::note_off(0, 0, 57)
                        });
                    }
                }
            });
        });

        let inserted: BTreeSet<InputRef> = self
            .controller
            .last_report()
            .inserted_delays()
            .map(|(_, to)| *to)
            .collect();
        let mut viewer = Viewer {
            controller: &mut self.controller,
            plain: self.plain,
            inserted,
            changed: false,
        };
        egui::CentralPanel::default().show(ui, |ui| {
            SnarlWidget::new()
                .id(egui::Id::new("loom-snarl"))
                .style(self.style)
                .show(&mut self.snarl, &mut viewer, ui);
        });
        if viewer.changed {
            self.status = match self.controller.commit() {
                Ok(_) => report(&self.controller),
                Err(e) => format!("error: {e}"),
            };
        }
        self.ui_time = started.elapsed();
    }
}

/// Bridges snarl callbacks to the engine `Controller`.
struct Viewer<'a> {
    controller: &'a mut Controller,
    plain: bool,
    inserted: BTreeSet<InputRef>,
    changed: bool,
}

impl Viewer<'_> {
    fn descriptor(&self, id: ModuleId) -> &'static loom_core::ModuleDescriptor {
        self.controller.graph().node(id).expect("module exists").descriptor
    }
}

fn pin(signal: SignalType) -> PinInfo {
    // Shape as well as colour, for colour vision deficiency (NFR-UX-03).
    let info = match signal {
        SignalType::Gate | SignalType::Pitch => PinInfo::square(),
        _ => PinInfo::circle(),
    };
    info.with_fill(colour(signal))
}

impl SnarlViewer<ModuleId> for Viewer<'_> {
    fn title(&mut self, node: &ModuleId) -> String {
        format!("{} {}", self.descriptor(*node).name, node)
    }

    fn header_frame(
        &mut self,
        default: egui::Frame,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        snarl: &Snarl<ModuleId>,
    ) -> egui::Frame {
        let id = snarl[node];
        default.fill(category_colour(self.descriptor(id).category))
    }

    fn inputs(&mut self, node: &ModuleId) -> usize {
        self.descriptor(*node).inputs.len()
    }

    fn outputs(&mut self, node: &ModuleId) -> usize {
        self.descriptor(*node).outputs.len()
    }

    fn show_input(
        &mut self,
        pin_ref: &InPin,
        ui: &mut egui::Ui,
        snarl: &mut Snarl<ModuleId>,
    ) -> impl egui_snarl::ui::SnarlPin + 'static {
        let id = snarl[pin_ref.id.node];
        let port = &self.descriptor(id).inputs[pin_ref.id.input];
        let delayed = self.inserted.contains(&InputRef {
            module: id,
            port: pin_ref.id.input,
        });
        // egui-snarl has no dashed wires; mark inserted delays in the label.
        ui.label(if delayed {
            format!("{} (delay)", port.name)
        } else {
            port.name.to_string()
        });
        pin(port.signal)
    }

    fn show_output(
        &mut self,
        pin_ref: &OutPin,
        ui: &mut egui::Ui,
        snarl: &mut Snarl<ModuleId>,
    ) -> impl egui_snarl::ui::SnarlPin + 'static {
        let id = snarl[pin_ref.id.node];
        let port = &self.descriptor(id).outputs[pin_ref.id.output];
        ui.label(port.name);
        pin(port.signal)
    }

    fn has_body(&mut self, node: &ModuleId) -> bool {
        !self.plain && !self.descriptor(*node).params.is_empty()
    }

    fn show_body(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut egui::Ui,
        snarl: &mut Snarl<ModuleId>,
    ) {
        let id = snarl[node];
        let descriptor = self.descriptor(id);
        let values = self.controller.graph().node(id).expect("module exists").params.clone();
        ui.vertical(|ui| {
            for (index, spec) in descriptor.params.iter().enumerate() {
                let mut value = values[index];
                let slider = egui::Slider::new(&mut value, spec.min..=spec.max)
                    .text(spec.name)
                    .suffix(spec.unit);
                if ui.add(slider).changed() {
                    let _ = self.controller.set_param(id, index, value);
                }
            }
        });
    }

    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<ModuleId>) {
        let output = OutputRef {
            module: snarl[from.id.node],
            port: from.id.output,
        };
        let input = InputRef {
            module: snarl[to.id.node],
            port: to.id.input,
        };
        if self.controller.connect(output, input).is_ok() {
            // Each input takes one cable: a new one replaces the old one.
            snarl.drop_inputs(to.id);
            snarl.connect(from.id, to.id);
            self.changed = true;
        }
    }

    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<ModuleId>) {
        let input = InputRef {
            module: snarl[to.id.node],
            port: to.id.input,
        };
        let _ = self.controller.disconnect(input);
        snarl.disconnect(from.id, to.id);
        self.changed = true;
    }

    fn drop_inputs(&mut self, pin_ref: &InPin, snarl: &mut Snarl<ModuleId>) {
        let input = InputRef {
            module: snarl[pin_ref.id.node],
            port: pin_ref.id.input,
        };
        let _ = self.controller.disconnect(input);
        snarl.drop_inputs(pin_ref.id);
        self.changed = true;
    }

    fn drop_outputs(&mut self, pin_ref: &OutPin, snarl: &mut Snarl<ModuleId>) {
        for remote in &pin_ref.remotes {
            let input = InputRef {
                module: snarl[remote.node],
                port: remote.input,
            };
            let _ = self.controller.disconnect(input);
        }
        snarl.drop_outputs(pin_ref.id);
        self.changed = true;
    }

    fn has_node_menu(&mut self, _node: &ModuleId) -> bool {
        true
    }

    fn show_node_menu(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut egui::Ui,
        snarl: &mut Snarl<ModuleId>,
    ) {
        if ui.button("Remove").clicked() {
            let id = snarl.remove_node(node);
            let _ = self.controller.remove_module(id);
            self.changed = true;
            ui.close();
        }
    }
}

fn style() -> SnarlStyle {
    SnarlStyle {
        node_layout: Some(NodeLayout::coil()),
        pin_placement: Some(PinPlacement::Edge),
        pin_size: Some(9.0),
        ..SnarlStyle::new()
    }
}

fn report(controller: &Controller) -> String {
    let report = controller.last_report();
    let inserted = report.inserted_delays().count();
    match report.loops.len() {
        0 => "no feedback loops".to_string(),
        n => format!("{n} feedback loop(s), {inserted} inserted 1-sample delay(s) (marked 'delay')"),
    }
}

fn colour(signal: SignalType) -> Color32 {
    let [r, g, b] = signal.colour();
    Color32::from_rgb(r, g, b)
}

fn category_colour(category: &str) -> Color32 {
    match category {
        "Phase" => Color32::from_rgb(0x5B, 0x3F, 0x8C),
        "Oscillator" => Color32::from_rgb(0x8C, 0x3F, 0x3F),
        "Filter" => Color32::from_rgb(0x3F, 0x6E, 0x8C),
        "Envelope" => Color32::from_rgb(0x3F, 0x8C, 0x5B),
        "MIDI" | "I/O" => Color32::from_rgb(0x8C, 0x6E, 0x3F),
        _ => Color32::from_rgb(0x4A, 0x4A, 0x55),
    }
}

/// Places modules without a saved position in columns by signal-flow depth
/// (same algorithm as the Iced spike).
fn auto_layout(controller: &Controller, positions: &mut BTreeMap<u32, Pos2>) {
    let graph = controller.graph();
    let mut sources: BTreeMap<u32, Vec<u32>> = graph.nodes().map(|(id, _)| (id.0, Vec::new())).collect();
    for (from, to) in graph.cables() {
        if from.module != to.module {
            sources.get_mut(&to.module.0).unwrap().push(from.module.0);
        }
    }
    let mut depth: BTreeMap<u32, usize> = BTreeMap::new();
    while depth.len() < sources.len() {
        let ready = sources
            .iter()
            .find(|(id, srcs)| !depth.contains_key(id) && srcs.iter().all(|s| depth.contains_key(s)))
            .or_else(|| sources.iter().find(|(id, _)| !depth.contains_key(id)))
            .map(|(id, _)| *id)
            .unwrap();
        let d = sources[&ready]
            .iter()
            .filter_map(|s| depth.get(s))
            .map(|d| d + 1)
            .max()
            .unwrap_or(0);
        depth.insert(ready, d);
    }
    let mut next_y: BTreeMap<usize, f32> = BTreeMap::new();
    for (id, node) in graph.nodes() {
        if positions.contains_key(&id.0) {
            continue;
        }
        let column = depth[&id.0];
        let y = next_y.entry(column).or_insert(40.0);
        positions.insert(id.0, Pos2::new(40.0 + column as f32 * 360.0, *y));
        let rows = node.descriptor.inputs.len() + node.descriptor.outputs.len() + 2 * node.descriptor.params.len();
        *y += 60.0 + rows as f32 * 20.0;
    }
}

/// Builds `n` modules in chains of five: note -> phase -> saw -> svf -> mul.
/// Only the first chain is audible. Same as the Iced spike.
fn generate(controller: &mut Controller, n: usize) {
    let c = controller;
    let out = c.add_module("loom.io.audio-out").unwrap();
    let note = c.add_module("loom.midi.note").unwrap();
    let mut made = 2;
    let mut chain = 0;
    while made + 4 <= n {
        let phase = c.add_module("loom.phase.generator").unwrap();
        let saw = c.add_module("loom.osc.saw").unwrap();
        let svf = c.add_module("loom.filter.svf").unwrap();
        let mul = c.add_module("loom.math.mul").unwrap();
        c.connect_named(note, "pitch", phase, "pitch").unwrap();
        c.connect_named(phase, "phase", saw, "phase").unwrap();
        c.connect_named(saw, "out", svf, "in").unwrap();
        c.connect_named(svf, "lowpass", mul, "a").unwrap();
        c.connect_named(note, "gate", mul, "b").unwrap();
        c.set_param_named(mul, "gain", if chain == 0 { 0.2 } else { 0.0 })
            .unwrap();
        if chain == 0 {
            c.connect_named(mul, "out", out, "left").unwrap();
        }
        made += 4;
        chain += 1;
    }
    c.commit().unwrap();
}
