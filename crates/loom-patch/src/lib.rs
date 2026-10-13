//! The LoomDSP patch file format (ADR-0008).
//!
//! Patches are RON (Rusty Object Notation) text files. Why RON: it keeps
//! Rust's data model (structs, enums, tuples) exactly, allows comments, and
//! diffs well in git.
//!
//! ```ron
//! Patch(
//!     version: 1,
//!     name: "Simple sine",
//!     modules: [
//!         (id: 1, kind: "loom.midi.note@1"),
//!         (id: 2, kind: "loom.phase.generator@1"),
//!         (id: 3, kind: "loom.osc.sine@1"),
//!         (id: 4, kind: "loom.io.audio-out@1"),
//!     ],
//!     cables: [
//!         (from: (1, "pitch"), to: (2, "pitch")),
//!         (from: (2, "phase"), to: (3, "phase")),
//!         (from: (3, "out"), to: (4, "left")),
//!     ],
//! )
//! ```
//!
//! Saved files are canonical: modules sorted by ID, cables sorted by input,
//! parameters sorted by name. This keeps git diffs small.

#![warn(missing_docs)]

use loom_core::{Controller, Graph, InputRef, LoomError, ModuleId, OutputRef};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The patch format version this build writes.
pub const CURRENT_VERSION: u32 = 1;

/// A patch file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Patch")]
pub struct Patch {
    /// Format version. See [`CURRENT_VERSION`].
    pub version: u32,
    /// Display name.
    #[serde(default)]
    pub name: String,
    /// Free text for the patch author.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Modules.
    #[serde(default)]
    pub modules: Vec<PatchModule>,
    /// Cables.
    #[serde(default)]
    pub cables: Vec<PatchCable>,
}

/// A module in a patch file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchModule {
    /// Stable module ID within the patch.
    pub id: u32,
    /// Module type as `"id@version"`.
    pub kind: String,
    /// Optional user label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Parameter values by name. Missing parameters use their defaults.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, f32>,
    /// Position on the canvas, for the editor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<(f32, f32)>,
}

/// A cable in a patch file: `(module id, port name)` to `(module id, port name)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchCable {
    /// Output side.
    pub from: (u32, String),
    /// Input side.
    pub to: (u32, String),
}

/// Patch loading and saving errors.
#[derive(Debug, thiserror::Error)]
pub enum PatchError {
    /// The file could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// File path.
        path: String,
        /// Underlying error.
        source: std::io::Error,
    },
    /// The text is not a valid patch.
    #[error("invalid patch: {0}")]
    Parse(#[from] ron::error::SpannedError),
    /// Serialisation failed.
    #[error("could not write patch: {0}")]
    Write(#[from] ron::Error),
    /// The file was written by a newer LoomDSP.
    #[error("patch format version {found} is newer than this build supports ({supported})")]
    TooNew {
        /// Version in the file.
        found: u32,
        /// Highest supported version.
        supported: u32,
    },
    /// The patch refers to something the engine rejects.
    #[error(transparent)]
    Engine(#[from] LoomError),
    /// A parameter name does not exist on a module.
    #[error("module {module}: unknown parameter `{param}`")]
    UnknownParam {
        /// Module ID in the patch.
        module: u32,
        /// Parameter name.
        param: String,
    },
}

impl Patch {
    /// An empty patch.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            version: CURRENT_VERSION,
            name: name.into(),
            description: String::new(),
            modules: Vec::new(),
            cables: Vec::new(),
        }
    }

    /// Parses RON text and migrates it to the current version.
    pub fn from_ron(text: &str) -> Result<Self, PatchError> {
        let patch: Patch = ron::from_str(text)?;
        patch.migrate()
    }

    /// Reads a patch file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, PatchError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| PatchError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_ron(&text)
    }

    /// Canonical RON text.
    pub fn to_ron(&self) -> Result<String, PatchError> {
        let mut canonical = self.clone();
        canonical.canonicalise();
        let config = ron::ser::PrettyConfig::new()
            .struct_names(true)
            .depth_limit(2)
            .separate_tuple_members(false)
            .indentor("    ");
        let mut text = ron::ser::to_string_pretty(&canonical, config)?;
        text.push('\n');
        Ok(text)
    }

    /// Writes a patch file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), PatchError> {
        let path = path.as_ref();
        std::fs::write(path, self.to_ron()?).map_err(|source| PatchError::Io {
            path: path.display().to_string(),
            source,
        })
    }

    /// Sorts modules, cables, and parameters into canonical order.
    pub fn canonicalise(&mut self) {
        self.modules.sort_by_key(|m| m.id);
        self.cables.sort_by(|a, b| (a.to.0, &a.to.1).cmp(&(b.to.0, &b.to.1)));
    }

    /// Upgrades an older patch to [`CURRENT_VERSION`].
    ///
    /// Each format change adds one step here, plus a test that loads a
    /// fixture saved in the old format (FR-FILE-02).
    pub fn migrate(mut self) -> Result<Self, PatchError> {
        if self.version > CURRENT_VERSION {
            return Err(PatchError::TooNew {
                found: self.version,
                supported: CURRENT_VERSION,
            });
        }
        // Version 0 was never released. Treat it as version 1.
        if self.version == 0 {
            self.version = 1;
        }
        Ok(self)
    }

    /// Adds every module and cable to a controller and commits.
    ///
    /// The controller should be empty. Module IDs from the file are kept.
    pub fn load_into(&self, controller: &mut Controller) -> Result<(), PatchError> {
        for module in &self.modules {
            let id = ModuleId(module.id);
            controller.add_module_with_id(id, &module.kind)?;
            for (name, &value) in &module.params {
                controller
                    .set_param_named(id, name, value)
                    .map_err(|error| match error {
                        LoomError::NoSuchParam { .. } => PatchError::UnknownParam {
                            module: module.id,
                            param: name.clone(),
                        },
                        other => PatchError::Engine(other),
                    })?;
            }
        }
        for cable in &self.cables {
            controller.connect_named(ModuleId(cable.from.0), &cable.from.1, ModuleId(cable.to.0), &cable.to.1)?;
        }
        controller.commit()?;
        Ok(())
    }

    /// Builds a patch from a graph. Only non-default parameters are saved.
    pub fn from_graph(name: impl Into<String>, graph: &Graph) -> Self {
        let mut patch = Patch::new(name);
        for (id, node) in graph.nodes() {
            let params = node
                .descriptor
                .params
                .iter()
                .zip(&node.params)
                .filter(|(spec, value)| **value != spec.default)
                .map(|(spec, value)| (spec.name.to_string(), *value))
                .collect();
            patch.modules.push(PatchModule {
                id: id.0,
                kind: node.descriptor.kind(),
                label: None,
                params,
                position: None,
            });
        }
        for (from, to) in graph.cables() {
            patch.cables.push(PatchCable {
                from: (from.module.0, port_name(graph, from)),
                to: (to.module.0, input_name(graph, to)),
            });
        }
        patch.canonicalise();
        patch
    }
}

fn port_name(graph: &Graph, output: OutputRef) -> String {
    graph
        .node(output.module)
        .map(|n| n.descriptor.outputs[output.port].name.to_string())
        .unwrap_or_default()
}

fn input_name(graph: &Graph, input: InputRef) -> String {
    graph
        .node(input.module)
        .map(|n| n.descriptor.inputs[input.port].name.to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use loom_core::{EngineConfig, create_engine};

    const SIMPLE: &str = r#"
// Comments are allowed.
Patch(
    version: 1,
    name: "Simple sine",
    modules: [
        (id: 3, kind: "loom.osc.sine@1"),
        (id: 1, kind: "loom.midi.note@1"),
        (id: 2, kind: "loom.phase.generator@1", params: {"tune": 12.0}),
        (id: 4, kind: "loom.io.audio-out@1"),
    ],
    cables: [
        (from: (2, "phase"), to: (3, "phase")),
        (from: (1, "pitch"), to: (2, "pitch")),
        (from: (3, "out"), to: (4, "left")),
    ],
)
"#;

    #[test]
    fn loads_and_round_trips() {
        let patch = Patch::from_ron(SIMPLE).unwrap();
        let mut parts = create_engine(loom_modules::registry(), EngineConfig::default());
        patch.load_into(&mut parts.controller).unwrap();

        let saved = Patch::from_graph("Simple sine", parts.controller.graph());
        let mut expected = patch.clone();
        expected.canonicalise();
        assert_eq!(saved, expected);

        // Canonical text is stable across a parse/print cycle.
        let text = saved.to_ron().unwrap();
        assert_eq!(Patch::from_ron(&text).unwrap().to_ron().unwrap(), text);
    }

    #[test]
    fn rejects_newer_versions() {
        let text = SIMPLE.replace("version: 1", "version: 99");
        assert!(matches!(
            Patch::from_ron(&text),
            Err(PatchError::TooNew { found: 99, .. })
        ));
    }

    #[test]
    fn rejects_unknown_params() {
        let text = SIMPLE.replace("\"tune\"", "\"tuning\"");
        let patch = Patch::from_ron(&text).unwrap();
        let mut parts = create_engine(loom_modules::registry(), EngineConfig::default());
        assert!(matches!(
            patch.load_into(&mut parts.controller),
            Err(PatchError::UnknownParam { module: 2, .. })
        ));
    }
}
