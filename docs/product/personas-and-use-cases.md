# Personas and Use Cases

## Personas

### P1. Mira, the sound designer

- Uses Bitwig and Reaktor. Comfortable with modular concepts. Does not write Rust.
- Wants to build unusual instruments fast and keep them as presets.
- Pain today: her Grid patches only work inside Bitwig. She cannot share them with people who use other DAWs.
- Needs: fast patching, good visual feedback (scopes, meters), modulation everywhere, reliable save and load.

### P2. Tomas, the live performer

- Plays electronic sets with a laptop and MIDI controllers.
- Wants a small number of large, clear controls on stage. He does not want to see cables during a show.
- Pain today: complex setups are fragile. CPU spikes cause dropouts.
- Needs: performance mode with a locked panel, MIDI learn, fast preset switching without clicks, rock-solid audio.

### P3. Aino, the instrument builder

- A developer who sells or gives away plugins. Knows some DSP.
- Wants to prototype by patching, then ship a plugin without rewriting it in C++.
- Pain today: RNBO costs money and needs Max. Writing JUCE code is slow for experimentation.
- Needs: export to CLAP, VST3, and standalone (WAM2 later); control over plugin metadata and parameters; a clean, branded panel; a stable patch format for version control.

### P4. Leo, the module developer

- A Rust programmer interested in DSP.
- Wants to write a new filter or oscillator and have it appear in the module browser.
- Needs: a documented module SDK, a test harness, examples, and a clear contribution process.

### P5. Sanna, the educator

- Teaches sound synthesis at a university or in workshops.
- Wants students to open a link in a browser and start patching. No installs.
- Needs: browser version, example patches, visible signal flow, the ability to open a sub-patch and see how it works.
- Note: Sanna is the main reason for the optional web target (P5). Until then, she can use the desktop app with example patches.

## Key use cases

Each use case links to the requirements that enable it.

### UC1. Build an instrument from primitives (P1, P5)

1. Create a new patch.
2. Add a phase generator, an oscillator shaper, a filter, an envelope, and an output.
3. Connect them. Hear sound immediately while patching.
4. Add a feedback path from the filter output back to its input with a one-sample delay.
5. Save the patch.

Requirements: FR-GRAPH-*, FR-MOD-*, FR-CANVAS-*, FR-LIB-*, FR-FILE-01.

### UC2. Encapsulate and build a panel (P1, P3)

1. Select a group of modules and collapse them into a sub-patch called "Voice".
2. Expose four internal parameters as sub-patch inputs.
3. Open the panel editor. Place four knobs and a waveform display. Bind them to the exposed parameters.
4. Hide the cable view.

Requirements: FR-ENCAP-*, FR-PANEL-*.

### UC3. Perform live (P2). Highest priority use case

1. Open a saved set of instruments in performance mode.
2. The app recognises the connected MIDI controllers and loads their saved mappings.
3. Map any new knobs with MIDI learn.
4. Play notes. Sync tempo to external MIDI clock. Switch between presets and patches mid-set without clicks.
5. Tweak the patch structure during a break without any dropout.
6. If something goes wrong, hit "panic" to silence all notes instantly.

Requirements: FR-PERF-*, FR-MIDI-*, NFR-RT-*.

### UC4. Export to a desktop DAW (P3)

1. Choose "Export > CLAP" or "Export > VST3".
2. Set the plugin name, vendor, and category. Pick target platforms.
3. Install the plugin. Load it in a DAW. Automate its parameters.

Requirements: FR-NATIVE-*.

### UC5. Export a standalone performance app (P3)

1. Choose "Export > Standalone".
2. Get an app that opens straight into the instrument's front panel, with audio and MIDI settings.
3. Share it with a performer who does not use LoomDSP.

Requirements: FR-NATIVE-08.

### UC5b. Export to the browser (optional, P5)

1. Choose "Export > Web Audio Module".
2. Get a folder that can be hosted on a static web server.
3. Load it in a WAM2 host and play it.

Requirements: FR-WAM-*, NFR-WEB-*.

### UC6. Add a new module (P4)

1. Create a module crate from a template.
2. Implement the processing trait and describe ports and parameters.
3. Run the provided test harness.
4. Register the module so it appears in the library.

Requirements: FR-SDK-*.

### UC7. Learn from a shared patch (P5)

1. Open a patch from a URL in the browser (P5), or open a shared patch file in the desktop app.
2. Step into each sub-patch to see how it works.
3. Probe any cable with a scope.

Requirements: FR-FILE-*, FR-ENCAP-04, FR-VIS-*.
