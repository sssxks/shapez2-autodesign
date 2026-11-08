use crate::shape::{Color, Shape};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Node(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Operation {
    Source,
    Cut,
    Swap,
    RotateCw,
    RotateCcw,
    Stack,
    Paint,
    Crystallize,
}
impl Operation {
    pub fn arity(self) -> usize {
        match self {
            Self::Source => 0,
            Self::Swap | Self::Stack => 2,
            _ => 1,
        }
    }
    pub fn output_count(self) -> usize {
        match self {
            Self::Cut | Self::Swap => 2,
            _ => 1,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecipeNode {
    pub id: Node,
    pub op: Operation,
    pub inputs: Vec<Node>,
    pub expected: Shape,
    pub color: Option<Color>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recipe {
    pub target: Shape,
    pub root: Node,
    pub nodes: Vec<RecipeNode>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct OutputRef {
    pub node: Node,
    pub port: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifiedNode {
    pub op: Operation,
    pub inputs: Vec<OutputRef>,
    pub output: OutputRef,
    pub shape: Shape,
    pub color: Option<Color>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifiedRecipe {
    pub target: Shape,
    pub root: OutputRef,
    pub nodes: Vec<VerifiedNode>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct RecipeTrace {
    pub id: Node,
    pub op: Operation,
    pub expected: Shape,
    pub outputs: Vec<Option<String>>,
    pub selected_port: usize,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct RecipeReport {
    pub verified: bool,
    pub target: Shape,
    pub trace: Vec<RecipeTrace>,
    pub steps: Option<usize>,
    pub failed_node: Option<Node>,
}
impl Recipe {
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
