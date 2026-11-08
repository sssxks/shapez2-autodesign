//! Typed factory domain models.
use crate::shape::{Color, Shape};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// Execution-order recipe index, stored as a u32; semantically a natural-number node reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Node(pub u32);

/// Native shape operation used in a recipe DAG.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Operation {
    /// Introduce the declared source shape.
    Source,
    /// Split a shape into two halves.
    Cut,
    /// Exchange halves between two shapes.
    Swap,
    /// Rotate clockwise by one quadrant.
    RotateCw,
    /// Rotate counterclockwise by one quadrant.
    RotateCcw,
    /// Stack the second input on the first.
    Stack,
    /// Apply the requested color.
    Paint,
    /// Grow crystals using the requested color.
    Crystallize,
}
impl Operation {
    /// Number of input shapes consumed by this operation.
    pub fn arity(self) -> usize {
        match self {
            Self::Source => 0,
            Self::Swap | Self::Stack => 2,
            _ => 1,
        }
    }
    /// Number of output ports produced by this operation.
    pub fn output_count(self) -> usize {
        match self {
            Self::Cut | Self::Swap => 2,
            _ => 1,
        }
    }
}
/// One operation with references to earlier recipe nodes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecipeNode {
    /// Sequence or execution-order identifier.
    pub id: Node,
    /// Native operation to execute.
    pub op: Operation,
    /// Ordered input references or coordinates.
    pub inputs: Vec<Node>,
    /// Shape expected after this operation.
    pub expected: Shape,
    /// Paint or crystal color argument when required.
    pub color: Option<Color>,
}
/// Topologically ordered shape-production recipe.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recipe {
    /// Requested final shape.
    pub target: Shape,
    /// Output producing the requested final shape.
    pub root: Node,
    /// Operations in execution order.
    pub nodes: Vec<RecipeNode>,
}
/// A concrete output port on a native recipe node.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct OutputRef {
    /// Producing recipe node.
    pub node: Node,
    /// Zero-based native output port.
    pub port: usize,
}
/// Operation with logical inputs resolved to native output ports.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifiedNode {
    /// Native operation to execute.
    pub op: Operation,
    /// Ordered input references or coordinates.
    pub inputs: Vec<OutputRef>,
    /// Selected output or external-output status.
    pub output: OutputRef,
    /// Verified shape on the selected output.
    pub shape: Shape,
    /// Paint or crystal color argument when required.
    pub color: Option<Color>,
}
/// Recipe whose port choices have been resolved from a native execution trace.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifiedRecipe {
    /// Requested final shape.
    pub target: Shape,
    /// Output producing the requested final shape.
    pub root: OutputRef,
    /// Operations in execution order.
    pub nodes: Vec<VerifiedNode>,
}
/// Native outputs and selected port for one recipe operation.
#[derive(Debug, Serialize, Deserialize)]
pub struct RecipeTrace {
    /// Sequence or execution-order identifier.
    pub id: Node,
    /// Native operation to execute.
    pub op: Operation,
    /// Shape expected after this operation.
    pub expected: Shape,
    /// Native output values in port order.
    pub outputs: Vec<Option<Box<str>>>,
    /// Native output port matching the expected shape.
    pub selected_port: usize,
}
/// Native acceptance result and per-node execution trace.
#[derive(Debug, Serialize, Deserialize)]
pub struct RecipeReport {
    /// Native semantic acceptance status, when reported.
    pub verified: bool,
    /// Requested final shape.
    pub target: Shape,
    /// Per-node native execution results.
    pub trace: Vec<RecipeTrace>,
    /// Number of native execution steps, when reported.
    pub steps: Option<usize>,
    /// First failed node, when reported.
    pub failed_node: Option<Node>,
}
impl Recipe {
    /// Reject inconsistent inputs before execution.
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (self.root.0 as usize) < self.nodes.len(),
            "Recipe root is missing"
        );
        for (i, n) in self.nodes.iter().enumerate() {
            ensure!(
                n.id.0 as usize == i,
                "Recipe IDs must follow execution order"
            );
            ensure!(
                n.inputs.len() == n.op.arity() && n.inputs.iter().all(|p| (p.0 as usize) < i),
                "Invalid inputs for node {i}"
            );
            ensure!(
                matches!(n.op, Operation::Paint | Operation::Crystallize) == n.color.is_some(),
                "Invalid color argument for node {i}"
            );
        }
        ensure!(
            self.nodes[self.root.0 as usize].expected == self.target,
            "Root shape differs from target"
        );
        Ok(())
    }
    /// Resolve logical reuse into explicit native output-port references.
    pub fn verified(&self, report: &RecipeReport) -> Result<VerifiedRecipe> {
        self.validate()?;
        ensure!(
            report.verified
                && report.target == self.target
                && report.trace.len() == self.nodes.len(),
            "Native recipe verification failed"
        );
        let mut nodes: Vec<VerifiedNode> = Vec::new();
        for (n, t) in self.nodes.iter().zip(&report.trace) {
            ensure!(
                n.id == t.id && n.op == t.op && n.expected == t.expected,
                "Native trace does not match recipe"
            );
            ensure!(
                t.selected_port < n.op.output_count()
                    && t.outputs.get(t.selected_port).and_then(Option::as_deref)
                        == Some(n.expected.as_str()),
                "Invalid native output selection"
            );
            nodes.push(VerifiedNode {
                op: n.op,
                inputs: n
                    .inputs
                    .iter()
                    .map(|p| nodes[p.0 as usize].output)
                    .collect(),
                output: OutputRef {
                    node: n.id,
                    port: t.selected_port,
                },
                shape: n.expected.clone(),
                color: n.color,
            });
        }
        Ok(VerifiedRecipe {
            target: self.target.clone(),
            root: nodes[self.root.0 as usize].output,
            nodes,
        })
    }
}
