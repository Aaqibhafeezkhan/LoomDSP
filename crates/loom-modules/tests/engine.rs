//! Engine behaviour tests using the example patches.

mod common;

use common::*;
use loom_core::ModuleId;

#[test]
fn sine_patch_plays_a_note_and_is_silent_without_one() {
    let mut parts = load("sine.ron");
    let (silent, _) = render(&mut parts, 4_800, 128, &[]);
    assert_eq!(rms(&silent), 0.0);

    let mut parts = load("sine.ron");
    let (left, right) = render(&mut parts, 48_000, 128, &one_note());
    assert!(rms(&left[1_000..20_000]) > 0.05, "note should be audible");
    assert_eq!(left, right, "mono into one side plays on both sides");
    assert!(rms(&left[40_000..]) < 1e-3, "note should have released");
}

#[test]
fn renders_are_bit_identical_across_runs() {
    // NFR-SND-02
    for patch in ["sine.ron", "subtractive.ron", "fm-feedback.ron"] {
        let a = render(&mut load(patch), 48_000, 256, &one_note());
        let b = render(&mut load(patch), 48_000, 256, &one_note());
        assert_eq!(a, b, "{patch}");
    }
}

#[test]
fn output_does_not_depend_on_host_buffer_size() {
    // FR-GRAPH-04 and the P1 exit criterion: feedback sounds the same at
    // buffer sizes 32 and 1024.
    for patch in ["subtractive.ron", "fm-feedback.ron", "fm-feedback-delay1.ron"] {
        let small = render(&mut load(patch), 48_000, 32, &one_note());
        let large = render(&mut load(patch), 48_000, 1024, &one_note());
        let odd = render(&mut load(patch), 48_000, 77, &one_note());
        assert_eq!(small, large, "{patch}: 32 vs 1024");
        assert_eq!(small, odd, "{patch}: 32 vs 77");
        assert!(rms(&small.0) > 0.01, "{patch} should make sound");
    }
}

#[test]
fn compiler_reports_feedback_loops() {
    let parts = load("subtractive.ron");
    assert!(parts.controller.last_report().loops.is_empty());

    let parts = load("fm-feedback.ron");
    let report = parts.controller.last_report();
    assert_eq!(report.loops.len(), 1);
    let feedback = &report.loops[0];
    assert_eq!(
        feedback.modules,
        vec![ModuleId(2), ModuleId(3), ModuleId(4), ModuleId(5)]
    );
    assert_eq!(feedback.inserted_delays.len(), 1);
    let (from, to) = feedback.inserted_delays[0];
    assert_eq!((from.module, to.module), (ModuleId(5), ModuleId(2)));

    // With an explicit unit delay, the compiler adds nothing.
    let parts = load("fm-feedback-delay1.ron");
    let report = parts.controller.last_report();
    assert_eq!(report.loops.len(), 1);
    assert!(report.loops[0].inserted_delays.is_empty());
}

#[test]
fn explicit_and_automatic_delays_sound_the_same() {
    // The compiler breaks the loop at the same cable the user did, so the
    // two patches must render identically.
    let automatic = render(&mut load("fm-feedback.ron"), 48_000, 128, &one_note());
    let explicit = render(&mut load("fm-feedback-delay1.ron"), 48_000, 128, &one_note());
    assert_eq!(automatic, explicit);
}

#[test]
fn self_loop_gets_a_delay() {
    let mut parts = load("sine.ron");
    let c = &mut parts.controller;
    let add = c.add_module("loom.math.add").unwrap();
    c.connect_named(add, "out", add, "a").unwrap();
    let report = c.commit().unwrap().clone();
    assert_eq!(report.loops.len(), 1);
    assert_eq!(report.loops[0].modules, vec![add]);
    assert_eq!(report.loops[0].inserted_delays.len(), 1);
    // An integrator with offset 1 counts up by one per sample.
    c.set_param_named(add, "offset", 1.0).unwrap();
    render(&mut parts, 256, 128, &[]);
}

#[test]
fn edits_during_playback_do_not_change_the_sound() {
    // Adding and removing modules that are not in the signal path must not
    // disturb anything that is: no reset state, no glitch (FR-GRAPH-02).
    let reference = render(&mut load("fm-feedback.ron"), 48_000, 128, &one_note());

    let mut parts = load("fm-feedback.ron");
    let mut extra = Vec::new();
    let edited = render_with(&mut parts, 48_000, 128, &one_note(), |position, parts| {
        let c = &mut parts.controller;
        match (position / 128) % 4 {
            0 => {
                let noise = c.add_module("loom.osc.noise").unwrap();
                let gain = c.add_module("loom.math.mul").unwrap();
                c.connect_named(noise, "out", gain, "a").unwrap();
                // Feed it into the loop's adder with zero gain: the topology
                // changes, the sound must not.
                c.set_param_named(gain, "gain", 0.0).unwrap();
                extra.push((noise, gain));
            }
            2 => {
                if let Some((noise, gain)) = extra.pop() {
                    c.remove_module(noise).unwrap();
                    c.remove_module(gain).unwrap();
                }
            }
            _ => {}
        }
        c.commit().unwrap();
    });
    assert_eq!(reference, edited);
    let stats = parts.controller.stats();
    assert_eq!(stats.leaked.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert!(stats.schedule_swaps.load(std::sync::atomic::Ordering::Relaxed) > 100);
}

#[test]
fn unit_delay_state_survives_recompiles() {
    // Recompiling must carry delay state across, or loops would click.
    let reference = render(&mut load("fm-feedback-delay1.ron"), 9_600, 64, &one_note());
    let mut parts = load("fm-feedback-delay1.ron");
    let edited = render_with(&mut parts, 9_600, 64, &one_note(), |_, parts| {
        let c = &mut parts.controller;
        let id = c.add_module("loom.math.constant").unwrap();
        c.commit().unwrap();
        c.remove_module(id).unwrap();
        c.commit().unwrap();
    });
    assert_eq!(reference, edited);
}

#[test]
fn removed_modules_are_returned_for_freeing() {
    let mut parts = load("sine.ron");
    for _ in 0..50 {
        let id = parts.controller.add_module("loom.osc.noise").unwrap();
        parts.controller.commit().unwrap();
        render(&mut parts, 128, 128, &[]);
        parts.controller.remove_module(id).unwrap();
        parts.controller.commit().unwrap();
        render(&mut parts, 128, 128, &[]);
    }
    parts.controller.collect_garbage();
    let stats = parts.controller.stats();
    assert_eq!(stats.leaked.load(std::sync::atomic::Ordering::Relaxed), 0);
}
