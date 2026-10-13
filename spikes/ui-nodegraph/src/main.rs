//! P0 UI spike (ADR-0009): a LoomDSP patch canvas on Iced + iced_nodegraph.
//!
//! What it shows:
//! - Modules as nodes, typed and coloured ports, parameter sliders.
//! - Patching on the canvas edits the live engine: every connect, disconnect,
//!   or delete becomes a `Controller` edit and a schedule swap while audio
//!   keeps playing.
//! - The compiler's report (feedback loops, inserted 1-sample delays) shown
//!   live, with inserted delays drawn as dashed cables.
//! - Frame rate, worst frame time, and the canvas's own CPU time, for the
//!   ADR-0009 measurements.
//!
//! Run:
//!   cargo run --release -- ../../examples/patches/fm-feedback.ron
//!   cargo run --release -- --generate 300          # stress test: 300 modules
//!   cargo run --release -- --generate 300 --no-audio
//!
//! This is throwaway code to answer questions, not the start of the real UI.

use iced::widget::{button, column, container, pick_list, row, slider, text};
use iced::{Color, Element, Length, Padding, Point, Subscription, Theme, Vector};
use iced_nodegraph::prelude::*;
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

/// Output pins are numbered from here, inputs from 0.
const OUT: u32 = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct LoomIds;

impl Ids for LoomIds {
    type NodeId = u32;
    type PinId = u32;
    type EdgeId = ();
    type AnchorId = usize;
    type Payload = SignalType;
}

type Pin = PinRef<LoomIds>;

#[derive(Debug, Clone)]
enum Message {
    Moved { delta: Vector, ids: Vec<u32> },
    Connected { from: Pin, to: Pin },
    Disconnected { from: Pin, to: Pin },
    Delete(Vec<u32>),
    Param { module: u32, index: usize, value: f32 },
    Frame(Instant),
    Info(Duration),
    Pick(String),
    Add,
    HoldNote,
    Save,
}

#[derive(Clone)]
struct Args {
    patch: Option<PathBuf>,
    generate: Option<usize>,
    audio: bool,
}

fn parse_args() -> Args {
    let mut args = Args {
        patch: None,
        generate: None,
        audio: true,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--generate" => args.generate = it.next().and_then(|n| n.parse().ok()),
            "--no-audio" => args.audio = false,
            path => args.patch = Some(PathBuf::from(path)),
        }
    }
    if args.patch.is_none() && args.generate.is_none() {
        args.patch = Some(PathBuf::from("../../examples/patches/fm-feedback.ron"));
    }
    args
}

fn main() -> iced::Result {
    let args = parse_args();
    iced::application(move || App::new(args.clone()), App::update, App::view)
        .title("LoomDSP UI spike")
        .theme(App::theme)
        .subscription(App::subscription)
        .window_size((1400.0, 900.0))
        .run()
}

struct App {
    controller: Controller,
    positions: BTreeMap<u32, Point>,
    name: String,
    save_path: PathBuf,
    _audio: Option<RunningAudio>,
    _midi: Option<MidiConnection>,
    audio_stats: Option<Arc<CallbackStats>>,
    midi_sender: Option<MidiSender>,
    holding: bool,
    status: String,
    audio_note: String,
    kinds: Vec<String>,
    picked: Option<String>,
    frames: VecDeque<Instant>,
    worst_frame: Duration,
    graph_cpu: Duration,
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
        let (name, save_path) = if let Some(n) = args.generate {
            generate(&mut controller, n);
            (
                format!("Generated ({n} modules)"),
                PathBuf::from(format!("generated-{n}.ron")),
            )
        } else {
            let path = args.patch.clone().expect("patch path");
            let patch = Patch::load(&path).expect("patch loads");
            patch.load_into(&mut controller).expect("patch is valid");
            for m in &patch.modules {
                if let Some((x, y)) = m.position {
                    positions.insert(m.id, Point::new(x, y));
                }
            }
            (patch.name.clone(), path.with_extension("saved.ron"))
        };
        auto_layout(&controller, &mut positions);

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
            positions,
            name,
            save_path,
            audio_stats: audio.as_ref().map(|a| a.stats().clone()),
            _audio: audio,
            _midi: midi,
            midi_sender,
            holding: false,
            status: String::new(),
            audio_note,
            kinds: loom_modules::registry().iter().map(|d| d.id.to_string()).collect(),
            picked: None,
            frames: VecDeque::new(),
            worst_frame: Duration::ZERO,
            graph_cpu: Duration::ZERO,
        };
        app.report();
        app
    }

    fn theme(&self) -> Theme {
        Theme::TokyoNight
    }

    fn subscription(&self) -> Subscription<Message> {
        // Redraw every frame, so the FPS counter measures real render cost.
        iced::window::frames().map(Message::Frame)
    }

    fn report(&mut self) {
        let report = self.controller.last_report();
        let inserted = report.inserted_delays().count();
        self.status = match report.loops.len() {
            0 => "no feedback loops".to_string(),
            n => format!("{n} feedback loop(s), {inserted} inserted 1-sample delay(s) (dashed)"),
        };
    }

    fn commit(&mut self) {
        match self.controller.commit() {
            Ok(_) => self.report(),
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Moved { delta, ids } => {
                for id in ids {
                    if let Some(p) = self.positions.get_mut(&id) {
                        *p += delta;
                    }
                }
            }
            Message::Connected { from, to } => {
                if let Some((output, input)) = endpoints(from, to) {
                    match self.controller.connect(output, input) {
                        Ok(_) => self.commit(),
                        Err(e) => self.status = format!("error: {e}"),
                    }
                }
            }
            Message::Disconnected { from, to } => {
                if let Some((_, input)) = endpoints(from, to) {
                    let _ = self.controller.disconnect(input);
                    self.commit();
                }
            }
            Message::Delete(ids) => {
                for id in ids {
                    let _ = self.controller.remove_module(ModuleId(id));
                    self.positions.remove(&id);
                }
                self.commit();
            }
            Message::Param { module, index, value } => {
                let _ = self.controller.set_param(ModuleId(module), index, value);
            }
            Message::Frame(now) => {
                if let Some(&last) = self.frames.back() {
                    let gap = now - last;
                    if self.frames.len() > 30 {
                        // Skip the first frames, which include start-up work.
                        self.worst_frame = self.worst_frame.max(gap);
                    }
                }
                self.frames.push_back(now);
                while self.frames.front().is_some_and(|&t| now - t > Duration::from_secs(1)) {
                    self.frames.pop_front();
                }
                self.controller.collect_garbage();
            }
            Message::Info(cpu) => self.graph_cpu = cpu,
            Message::Pick(kind) => self.picked = Some(kind),
            Message::Add => {
                if let Some(kind) = self.picked.clone() {
                    match self.controller.add_module(&kind) {
                        Ok(id) => {
                            let max_x = self.positions.values().map(|p| p.x).fold(0.0, f32::max);
                            self.positions.insert(id.0, Point::new(max_x + 240.0, 40.0));
                            self.commit();
                        }
                        Err(e) => self.status = format!("error: {e}"),
                    }
                }
            }
            Message::HoldNote => {
                if let Some(sender) = &mut self.midi_sender {
                    self.holding = !self.holding;
                    sender.send(if self.holding {
                        MidiEvent::note_on(0, 0, 57, 100)
                    } else {
                        MidiEvent::note_off(0, 0, 57)
                    });
                }
            }
            Message::Save => {
                let mut patch = Patch::from_graph(&self.name, self.controller.graph());
                for m in &mut patch.modules {
                    m.position = self.positions.get(&m.id).map(|p| (p.x.round(), p.y.round()));
                }
                self.status = match patch.save(&self.save_path) {
                    Ok(()) => format!("saved {}", self.save_path.display()),
                    Err(e) => format!("save failed: {e}"),
                };
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let graph = self.controller.graph();
        let inserted: BTreeSet<InputRef> = self
            .controller
            .last_report()
            .inserted_delays()
            .map(|(_, to)| *to)
            .collect();

        let mut canvas = NodeGraph::<LoomIds, _, _, _>::new()
            .on_move(|delta, ids| Message::Moved { delta, ids })
            .on_connect(|from, to| Message::Connected { from, to })
            .on_disconnect(|from, to| Message::Disconnected { from, to })
            .on_delete(Message::Delete)
            .on_info(|info| Message::Info(info.timings.iter().map(|t| t.duration).sum()))
            // Patch anything to anything (ADR-0007): only direction matters.
            .can_connect(|from, to| from.direction() != to.direction())
            .dragging_edge_style(|theme, pin| EdgeStyle {
                stroke_color: ColorQuad::arc(colour(*pin.info()), colour(*pin.info())),
                ..default_edge_style(theme, EdgeStatus::Idle)
            })
            .minimap(iced_nodegraph::Minimap::default())
            .width(Length::Fill)
            .height(Length::Fill);

        for (id, node) in graph.nodes() {
            let pos = self.positions.get(&id.0).copied().unwrap_or(Point::ORIGIN);
            canvas = canvas.push_node(self.module_node(id, node, pos));
        }

        canvas = canvas.edges(graph.cables().map(|(from, to)| {
            let dashed = inserted.contains(&to);
            edge(
                (),
                PinRef::new(from.module.0, OUT + from.port as u32),
                PinRef::new(to.module.0, to.port as u32),
            )
            .style(move |theme, status, start, end| {
                let mut style = EdgeStyle {
                    stroke_color: ColorQuad::arc(colour(*start.info()), colour(*end.info())),
                    ..default_edge_style(theme, status)
                };
                if dashed {
                    style.pattern = Pattern::dashed(style.pattern.thickness, 10.0, 6.0);
                }
                style
            })
        }));

        let fps = self.frames.len();
        let audio = self.audio_stats.as_ref().map_or(self.audio_note.clone(), |s| {
            let s = s.snapshot();
            format!(
                "audio load {:.1}% (max {:.1}%), overruns {}",
                s.last_load * 100.0,
                s.max_load * 100.0,
                s.overruns
            )
        });
        let stats = text(format!(
            "{} modules, {} cables | {fps} fps | worst frame {:.1} ms | canvas CPU {:.2} ms | {audio}",
            graph.len(),
            graph.cables().count(),
            self.worst_frame.as_secs_f64() * 1000.0,
            self.graph_cpu.as_secs_f64() * 1000.0,
        ))
        .size(13);

        let mut toolbar = row![
            text(&self.name).size(16),
            pick_list(self.kinds.clone(), self.picked.clone(), Message::Pick).placeholder("module..."),
            button("Add").on_press(Message::Add),
            button("Save").on_press(Message::Save),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);
        if self.midi_sender.is_some() {
            toolbar = toolbar
                .push(button(if self.holding { "Release note" } else { "Hold note" }).on_press(Message::HoldNote));
        }

        column![
            container(toolbar).padding(8),
            canvas,
            container(column![stats, text(&self.status).size(13)].spacing(2)).padding(8),
        ]
        .into()
    }

    fn module_node<'a>(
        &'a self,
        id: ModuleId,
        node: &'a loom_core::Node,
        pos: Point,
    ) -> iced_nodegraph::Node<'a, LoomIds, Message, Theme, iced::Renderer> {
        let d = node.descriptor;
        let mut body = column![].spacing(3);
        for (i, port) in d.inputs.iter().enumerate() {
            body = body.push(pin!(Left, i as u32, text(port.name).size(11), Input, port.signal));
        }
        for (i, port) in d.outputs.iter().enumerate() {
            body = body.push(pin!(
                Right,
                OUT + i as u32,
                text(port.name).size(11),
                Output,
                port.signal
            ));
        }
        for (index, spec) in d.params.iter().enumerate() {
            let value = node.params[index];
            let module = id.0;
            body = body.push(
                column![
                    text(format!("{} {:.3} {}", spec.name, value, spec.unit)).size(10),
                    slider(spec.min..=spec.max, value, move |value| Message::Param {
                        module,
                        index,
                        value
                    })
                    .step((spec.max - spec.min) / 1000.0),
                ]
                .spacing(1),
            );
        }
        let pad = Padding {
            top: 4.0,
            bottom: 4.0,
            left: 8.0,
            right: 8.0,
        };
        let content = column![
            node_header(
                container(text(format!("{} {}", d.name, id)).size(13)).padding(pad),
                category_colour(d.category),
                5.0
            ),
            container(body).width(Length::Fill).padding(pad),
        ]
        .width(Length::Fill);
        iced_nodegraph::node(id.0, pos, container(content).width(170.0)).pin_style(|theme, pin, _other, status| {
            PinStyle {
                color: colour(*pin.info()).into(),
                // Shape as well as colour, for colour vision deficiency (NFR-UX-03).
                shape: match pin.info() {
                    SignalType::Gate | SignalType::Pitch => PinShape::Square,
                    _ => PinShape::Circle,
                },
                ..default_pin_style(theme, status)
            }
        })
    }
}

fn colour(signal: SignalType) -> Color {
    let [r, g, b] = signal.colour();
    Color::from_rgb8(r, g, b)
}

fn category_colour(category: &str) -> Color {
    match category {
        "Phase" => Color::from_rgb8(0x5B, 0x3F, 0x8C),
        "Oscillator" => Color::from_rgb8(0x8C, 0x3F, 0x3F),
        "Filter" => Color::from_rgb8(0x3F, 0x6E, 0x8C),
        "Envelope" => Color::from_rgb8(0x3F, 0x8C, 0x5B),
        "MIDI" | "I/O" => Color::from_rgb8(0x8C, 0x6E, 0x3F),
        _ => Color::from_rgb8(0x4A, 0x4A, 0x55),
    }
}

/// Turns two canvas pins into (output, input), whichever end was dragged.
fn endpoints(a: Pin, b: Pin) -> Option<(OutputRef, InputRef)> {
    let (out, inp) = if a.pin_id >= OUT { (a, b) } else { (b, a) };
    if out.pin_id < OUT || inp.pin_id >= OUT {
        return None;
    }
    Some((
        OutputRef {
            module: ModuleId(out.node_id),
            port: (out.pin_id - OUT) as usize,
        },
        InputRef {
            module: ModuleId(inp.node_id),
            port: inp.pin_id as usize,
        },
    ))
}

/// Places modules without a saved position in columns by signal-flow depth.
///
/// Layering with cycle breaking: a module is placed once all of its sources
/// are placed. When only cycles remain, the lowest-ID unplaced module is
/// placed using the sources placed so far. Its column is one right of its
/// furthest-right placed source.
fn auto_layout(controller: &Controller, positions: &mut BTreeMap<u32, Point>) {
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
        positions.insert(id.0, Point::new(40.0 + column as f32 * 230.0, *y));
        let rows = node.descriptor.inputs.len() + node.descriptor.outputs.len() + 2 * node.descriptor.params.len();
        *y += 60.0 + rows as f32 * 20.0;
    }
}

/// Builds `n` modules in chains of five: note -> phase -> saw -> svf -> mul.
/// Only the first chain is audible.
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
