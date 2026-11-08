//! Scaffold construction using typed quadrants and logical recipe references.
use anyhow::{Result, ensure};
use shapez_model::{
    Color, Layer, Material, Node, Operation, Part, Recipe, RecipeDto, RecipeNodeDto, Shape,
};
use std::collections::HashMap;

/// A nonempty stack of layers under construction.
#[derive(Clone)]
struct Stack {
    /// Bottom layer that makes the stack nonempty.
    first: Layer,
    /// Additional layers in bottom-to-top construction order.
    rest: Vec<Layer>,
}
impl Stack {
    /// Freeze the current layers into a validated shape value.
    fn shape(&self) -> Result<Shape> {
        Shape::from_layers(self.first.clone(), self.rest.clone())
    }
    /// Visit all construction layers from bottom to top for in-place operations.
    fn layers_mut(&mut self) -> impl Iterator<Item = &mut Layer> {
        std::iter::once(&mut self.first).chain(&mut self.rest)
    }
    /// Borrow construction layers in native bottom-to-top order.
    fn layers(&self) -> impl Iterator<Item = &Layer> {
        std::iter::once(&self.first).chain(&self.rest)
    }
}
/// A produced logical node together with its nonempty shape.
#[derive(Clone)]
struct Built {
    /// Earlier recipe node producing this shape.
    node: Node,
    /// Nonempty shape carried by the produced node.
    stack: Stack,
}
/// Mutable planning phase; the completed DTO is validated and frozen once.
#[derive(Default)]
struct Planner {
    /// Operations appended in execution order before graph validation.
    nodes: Vec<RecipeNodeDto>,
    /// Reuse an earlier producer for an identical expected shape.
    cache: HashMap<Shape, Node>,
}
impl Planner {
    /// Reuse a matching producer or append an operation with its expected typed shape.
    fn node(
        &mut self,
        op: Operation,
        inputs: &[Node],
        stack: Stack,
        color: Option<Color>,
    ) -> Result<Built> {
        let expected = stack.shape()?;
        if let Some(&node) = self.cache.get(&expected) {
            return Ok(Built { node, stack });
        }
        let node = Node(u32::try_from(self.nodes.len())?);
        self.nodes.push(RecipeNodeDto {
            id: node,
            op,
            inputs: inputs.to_vec(),
            expected: expected.clone(),
            color,
        });
        self.cache.insert(expected, node);
        Ok(Built { node, stack })
    }
    /// Rotate every layer by one quadrant and record the matching native operation.
    fn rotate(&mut self, input: Built, clockwise: bool) -> Result<Built> {
        let mut stack = input.stack;
        for layer in stack.layers_mut() {
            layer.0.rotate_right(if clockwise { 1 } else { 3 });
        }
        self.node(
            if clockwise {
                Operation::RotateCw
            } else {
                Operation::RotateCcw
            },
            &[input.node],
            stack,
            None,
        )
    }
    /// Keep the first two quadrants on every layer through a native cut.
    fn half(&mut self, input: Built) -> Result<Built> {
        let mut stack = input.stack;
        for layer in stack.layers_mut() {
            layer.0[2..].fill(Part::Empty);
        }
        self.node(Operation::Cut, &[input.node], stack, None)
    }
    /// Use two cuts and rotations to retain only the first quadrant.
    fn isolate(&mut self, input: Built) -> Result<Built> {
        let half = self.half(input)?;
        let turned = self.rotate(half, true)?;
        let quarter = self.half(turned)?;
        self.rotate(quarter, false)
    }
    /// Combine disjoint occupied quadrants using a swap, optionally around a rotated axis.
    fn merge(&mut self, a: Built, b: Built, rotated: bool) -> Result<Built> {
        let (a, b) = if rotated {
            (self.rotate(a, true)?, self.rotate(b, true)?)
        } else {
            (a, b)
        };
        let mut stack = a.stack;
        while stack.rest.len() < b.stack.rest.len() {
            stack.rest.push(Layer([Part::Empty; 4]));
        }
        for (left, right) in stack.layers_mut().zip(b.stack.layers()) {
            for (l, r) in left.0.iter_mut().zip(right.0) {
                if *l == Part::Empty {
                    *l = r;
                }
            }
        }
        let merged = self.node(Operation::Swap, &[a.node, b.node], stack, None)?;
        if rotated {
            self.rotate(merged, false)
        } else {
            Ok(merged)
        }
    }
    /// Merge occupied halves while preserving an absent half without adding operations.
    fn optional_merge(
        &mut self,
        a: Option<Built>,
        b: Option<Built>,
        rotated: bool,
    ) -> Result<Option<Built>> {
        match (a, b) {
            (Some(a), Some(b)) => self.merge(a, b, rotated).map(Some),
            (a, b) => Ok(a.or(b)),
        }
    }
    /// Construct one occupied quadrant at the requested clockwise position.
    fn quarter(&mut self, part: Part, position: usize) -> Result<Built> {
        let full = match part {
            Part::Solid(Material::Crystal, color) => {
                let mask = self.supported_layer(Part::Empty)?;
                self.node(
                    Operation::Crystallize,
                    &[mask.node],
                    Stack {
                        first: Layer([part, SUPPORT, SUPPORT, SUPPORT]),
                        rest: vec![],
                    },
                    Some(color),
                )?
            }
            _ => {
                let raw = match part {
                    Part::Solid(m, _) => Part::Solid(m, Color::Uncolored),
                    p => p,
                };
                let source = self.node(
                    Operation::Source,
                    &[],
                    Stack {
                        first: Layer([raw; 4]),
                        rest: vec![],
                    },
                    None,
                )?;
                match part {
                    Part::Solid(_, color) if color != Color::Uncolored => self.node(
                        Operation::Paint,
                        &[source.node],
                        Stack {
                            first: Layer([part; 4]),
                            rest: vec![],
                        },
                        Some(color),
                    )?,
                    _ => source,
                }
            }
        };
        let mut result = self.isolate(full)?;
        for _ in 0..position {
            result = self.rotate(result, true)?;
        }
        Ok(result)
    }
    /// Support quadrants guarantee a result even when the first quadrant is empty.
    fn supported_layer(&mut self, part: Part) -> Result<Built> {
        let q1 = self.quarter(SUPPORT, 1)?;
        let left = if part == Part::Empty {
            q1
        } else {
            let q0 = self.quarter(part, 0)?;
            self.merge(q0, q1, true)?
        };
        let q2 = self.quarter(SUPPORT, 2)?;
        let q3 = self.quarter(SUPPORT, 3)?;
        let right = self.merge(q2, q3, true)?;
        self.merge(left, right, false)
    }
    /// Combine occupied quadrants, returning no producer for an empty layer.
    fn layer(&mut self, layer: Layer) -> Result<Option<Built>> {
        let mut q = [None, None, None, None];
        for (i, part) in layer.0.into_iter().enumerate() {
            if part != Part::Empty {
                q[i] = Some(self.quarter(part, i)?);
            }
        }
        let [q0, q1, q2, q3] = q;
        let a = self.optional_merge(q0, q1, true)?;
        let b = self.optional_merge(q2, q3, true)?;
        self.optional_merge(a, b, false)
    }
    /// Build a solid supported column, rejecting gaps and pins below later crystals.
    fn column(&mut self, parts: &[Part], position: usize) -> Result<Option<Built>> {
        let Some(last) = parts.iter().rposition(|p| *p != Part::Empty) else {
            return Ok(None);
        };
        let column = &parts[..=last];
        ensure!(
            column.iter().all(|p| *p != Part::Empty),
            "Unsupported column gap: candidate strategy needs solid columns"
        );
        let Some((&first, rest)) = column.split_first() else {
            return Ok(None);
        };
        let mut result = self.column_step(None, first)?;
        let mut has_pin = false;
        has_pin |= first == Part::Pin;
        for &part in rest {
            let crystal = matches!(part, Part::Solid(Material::Crystal, _));
            ensure!(
                !crystal || !has_pin,
                "Pin below a later crystal requires another strategy"
            );
            has_pin |= part == Part::Pin;
            result = self.column_step(Some(result), part)?;
        }
        let mut result = self.isolate(result)?;
        for _ in 0..position {
            result = self.rotate(result, true)?;
        }
        Ok(Some(result))
    }
    /// Extend a nonempty scaffold, resolving a crystal before returning it.
    fn column_step(&mut self, result: Option<Built>, part: Part) -> Result<Built> {
        let crystal = matches!(part, Part::Solid(Material::Crystal, _));
        let upper = self.supported_layer(if crystal { Part::Pin } else { part })?;
        let mut built = match result {
            None => upper,
            Some(lower) => {
                let mut stack = lower.stack;
                stack.rest.push(upper.stack.first);
                self.node(Operation::Stack, &[lower.node, upper.node], stack, None)?
            }
        };
        if let Part::Solid(Material::Crystal, color) = part {
            let mut stack = built.stack;
            let top = match stack.rest.last_mut() {
                Some(top) => top,
                None => &mut stack.first,
            };
            top.0[0] = part;
            built = self.node(Operation::Crystallize, &[built.node], stack, Some(color))?;
        }
        Ok(built)
    }
}
/// Uncolored circular scaffold quadrant.
const SUPPORT: Part = Part::Solid(Material::Circle, Color::Uncolored);
/// Plan a supported target and freeze its validated execution graph.
pub fn plan_recipe(target: &str) -> Result<Recipe> {
    let target: Shape = target.parse()?;
    let layers: Vec<_> = target.layers().collect();
    ensure!(layers.len() <= 4, "Prototype supports 1..4 layers");
    ensure!(
        layers.iter().all(|l| l.0.iter().any(|p| *p != Part::Empty)),
        "Empty layers require a different construction strategy"
    );
    let mut p = Planner::default();
    let result = if layers.len() == 1 {
        p.layer(layers[0].clone())?
    } else {
        let mut columns = [None, None, None, None];
        for (i, column) in columns.iter_mut().enumerate() {
            *column = p.column(&layers.iter().map(|l| l.0[i]).collect::<Vec<_>>(), i)?;
        }
        let [c0, c1, c2, c3] = columns;
        let a = p.optional_merge(c0, c1, true)?;
        let b = p.optional_merge(c2, c3, true)?;
        p.optional_merge(a, b, false)?
    };
    let Some(result) = result else {
        anyhow::bail!("Target has no occupied quadrants");
    };
    RecipeDto {
        target,
        root: result.node,
        nodes: p.nodes,
    }
    .try_into()
}

/// The symbol set and case enumeration are owned by this construction task.
pub fn check_suite() -> Result<shapez_runtime::Suite> {
    let mut cases = Vec::new();
    let symbols = ["--", "P-", "Cu", "cw", "cr"];
    for n in 1..625 {
        let mut k = n;
        let mut target = String::new();
        for _ in 0..4 {
            target.push_str(symbols[k % 5]);
            k /= 5;
        }
        cases.push(shapez_runtime::case! { name: target.clone(), operation: "recipe", recipe: plan_recipe(&target)? });
    }
    Ok(shapez_runtime::Suite {
        projects: vec![],
        cases,
    })
}
/// Native execution commands for individual recipes and the construction suite.
mod commands;
pub use commands::{run_check, run_recipe};
