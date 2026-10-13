//! The editable patch graph, owned by the control thread.

use crate::descriptor::{ModuleDescriptor, ModuleRole};
use std::collections::BTreeMap;
use std::fmt;

/// Stable identifier of a module in a patch. Saved in patch files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(pub u32);

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// An output port: module plus output index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutputRef {
    /// The module.
    pub module: ModuleId,
    /// Output index.
    pub port: usize,
}

/// An input port: module plus input index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputRef {
    /// The module.
    pub module: ModuleId,
    /// Input index.
    pub port: usize,
}

/// A module in the graph.
#[derive(Debug)]
pub struct Node {
    /// The module type.
    pub descriptor: &'static ModuleDescriptor,
    /// Current parameter values, one per [`ModuleDescriptor::params`].
    pub params: Vec<f32>,
    /// Engine slot holding the instance. `None` for engine-handled roles.
    pub(crate) slot: Option<usize>,
}

impl Node {
    /// True for modules the engine runs itself (unit delay, audio output).
    pub fn is_intrinsic(&self) -> bool {
        !matches!(self.descriptor.role, ModuleRole::Dsp(_))
    }
}

/// Modules and cables. Each input has at most one cable.
#[derive(Debug, Default)]
pub struct Graph {
    pub(crate) nodes: BTreeMap<ModuleId, Node>,
    /// Cables, keyed by the input they feed.
    pub(crate) cables: BTreeMap<InputRef, OutputRef>,
}

impl Graph {
    /// All modules, sorted by ID.
    pub fn nodes(&self) -> impl Iterator<Item = (ModuleId, &Node)> {
        self.nodes.iter().map(|(id, node)| (*id, node))
    }

    /// One module.
    pub fn node(&self, id: ModuleId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// All cables as `(from, to)`, sorted by input.
    pub fn cables(&self) -> impl Iterator<Item = (OutputRef, InputRef)> + '_ {
        self.cables.iter().map(|(to, from)| (*from, *to))
    }

    /// The cable feeding an input, if any.
    pub fn source_of(&self, input: InputRef) -> Option<OutputRef> {
        self.cables.get(&input).copied()
    }

    /// Number of modules.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// True when there are no modules.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}
