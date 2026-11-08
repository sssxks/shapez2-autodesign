use super::{
    SearchArgs,
    geometry::*,
    network::{Network, Witness},
};
use anyhow::{Result, ensure};
use shapez_solver::{Expr, FeasibleSolution, Var};
use std::collections::{BTreeMap, BTreeSet};

struct Pose {
    cell: PackedCell,
    body: BTreeSet<Point>,
}
pub struct Choice {
    pub enabled: Var,
    pub layer: usize,
    pub index: usize,
    pub cell: PackedCell,
}
pub struct Joint {
    pub network: Network,
    pub choices: Vec<Choice>,
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
}
pub fn build(
    seed: &[Vec<PackedCell>],
    args: &SearchArgs,
    witness: Option<&Witness>,
) -> Result<Joint> {
    ensure!(
        seed.len() == 3 && seed.iter().all(|row| row.len() == 16),
        "Expected 3 layers with 16 cells each"
    );
    let floor = floor();
    let (inputs, outputs) = ports(args.west());
    let approaches: BTreeSet<_> = inputs.iter().map(|p| shift(p.p, p.d, 1, p.p.2)).collect();
    let mut poses = Vec::new();
    let mut fixed = BTreeSet::new();
    let mut own_fixed = Vec::new();
    for (z, layer) in seed.iter().enumerate() {
        for cell in layer {
            let mut candidates = Vec::new();
            for dx in -args.radius..=args.radius {
                for dy in -args.radius..=args.radius {
                    for turn in if args.half_turn {
                        &[0, 2][..]
                    } else {
                        &[0][..]
                    } {
                        let offset = if *turn == 0 {
                            (0, 0)
                        } else {
                            rotate((4, 3), cell.r)
                        };
                        let mut candidate = cell.clone();
                        candidate.x += dx + offset.0;
                        candidate.y += dy + offset.1;
                        candidate.r = (candidate.r + turn) % 4;
                        let body = candidate.body(z as i16);
                        if body.is_subset(&floor) && body.is_disjoint(&approaches) {
                            candidates.push(Pose {
                                cell: candidate,
                                body,
                            });
                        }
                    }
                }
            }
            ensure!(
                !candidates.is_empty(),
                "No candidate poses for cell {}",
                poses.len()
            );
            let common = candidates
                .iter()
                .skip(1)
                .fold(candidates[0].body.clone(), |s, p| {
                    s.intersection(&p.body).copied().collect()
                });
            fixed.extend(common.iter().copied());
            own_fixed.push(common);
            poses.push(candidates);
        }
    }
    for (candidates, own) in poses.iter_mut().zip(&own_fixed) {
        candidates.retain(|pose| {
            pose.body
                .iter()
                .all(|p| !fixed.contains(p) || own.contains(p))
        });
    }
    let free: BTreeSet<_> = floor.difference(&fixed).copied().collect();
    let mut network = Network::default();
    network.transport(&free, &inputs, &outputs, &[4, 1], witness)?;
    let input_map: BTreeMap<_, _> = inputs.iter().map(|p| (p.p, p.d)).collect();
    let output_map: BTreeMap<_, _> = outputs.iter().map(|p| (p.p, p.d)).collect();
    let mut choices = Vec::new();
    let mut terminal_groups = BTreeMap::<(Point, usize), (Expr, Expr)>::new();
    for (cell_id, candidates) in poses.iter().enumerate() {
        let z = (cell_id / 16) as i16;
        let index = cell_id % 16;
        let reference = &seed[z as usize][index];
        let mut cell_choices = Vec::new();
        for pose in candidates {
            let pose_enabled = network.model.boolean();
            let mut pose_choices = Vec::new();
            for &p in pose.body.difference(&fixed) {
                network.occupy(p, pose_enabled);
            }
            for mirror in [false, true] {
                for split in -1..=3 {
                    let mut variants = Vec::new();
                    let mut configured = None;
                    for out in 0..4 {
                        let (cell, ip) = pose.cell.configure(z, mirror, out, split);
                        let a = cell.input(z);
                        let op = cell.output(z);
                        let b = cell.continuation(z);
                        let mut required = vec![a.p, op.p];
                        if let Some(b) = b {
                            required.extend([ip, b.p]);
                        }
                        if required.iter().copied().collect::<BTreeSet<_>>().len() != required.len()
                            || required.iter().any(|p| pose.body.contains(p))
                        {
                            continue;
                        }
                        if required.iter().any(|&p| {
                            !(free.contains(&p)
                                || p == op.p && output_map.get(&p) == Some(&op.d)
                                || p == a.p && input_map.get(&p) == Some(&a.d))
                        }) {
                            continue;
                        }
                        let selected = network.model.boolean();
                        let original = cell.x == reference.x
                            && cell.y == reference.y
                            && cell.r == reference.r
                            && cell.mirror == reference.mirror
                            && cell.out == reference.out
                            && cell.split == reference.split;
                        network
                            .model
                            .hint(selected, i64::from(original && !args.maximize_outputs));
                        variants.push(selected);
                        pose_choices.push(selected);
                        cell_choices.push(selected);
                        network.emit(1, op, selected.into(), selected);
                        configured = Some((a, b, ip));
                        choices.push(Choice {
                            enabled: selected,
                            layer: z as usize,
                            index,
                            cell,
                        });
                    }
                    let Some((input, continuation, ip)) = configured else {
                        continue;
                    };
                    let enabled = network.model.boolean();
                    let mut selection = Expr::sum(variants);
                    selection.term(enabled, -1);
                    network.model.equal(selection, 0);
                    let amount = network.model.flow(enabled, 4);
                    if let Some(b) = continuation {
                        network.model.bounded(
                            Expr {
                                terms: vec![(amount, 1), (enabled, -2)],
                                constant: 0,
                            },
                            0,
                            i64::MAX,
                        );
                        network.occupy(ip, enabled);
                        network.emit(
                            0,
                            b,
                            Expr {
                                terms: vec![(amount, 1), (enabled, -1)],
                                constant: 0,
                            },
                            enabled,
                        );
                    } else {
                        network.model.equal(
                            Expr {
                                terms: vec![(amount, 1), (enabled, -1)],
                                constant: 0,
                            },
                            0,
                        );
                    }
                    let (enabled_sum, amount_sum) =
                        terminal_groups.entry((input.p, input.d)).or_default();
                    enabled_sum.term(enabled, 1);
                    amount_sum.term(amount, 1);
                }
            }
            let mut selected = Expr::sum(pose_choices);
            selected.term(pose_enabled, -1);
            network.model.equal(selected, 0);
        }
        if args.maximize_outputs {
            network.model.at_most_one(cell_choices);
        } else {
            network.model.exactly_one(cell_choices);
        }
    }
    for ((p, d), (enabled, amount)) in terminal_groups {
        network.receive(0, Port { p, d }, 4, enabled, amount);
    }
    if args.maximize_outputs {
        for p in &inputs {
            let amount = network.model.variable(0, 4);
            network.add_flow((0, p.state()), &amount.into(), 1);
            network.fanin.entry((0, p.p)).or_default().constant += 1;
        }
        for p in &outputs {
            let amount = network.model.boolean();
            network.add_flow((1, p.state()), &amount.into(), -1);
            network.capacity.entry(p.p).or_default().constant += 1;
        }
        let mut objective = Expr::default();
        for c in &choices {
            objective.term(c.enabled, -1);
        }
        network.model.minimize(objective);
    } else {
        network.external(&inputs, &outputs, 1, 4, 1);
    }
    network.finish();
    Ok(Joint {
        network,
        choices,
        inputs,
        outputs,
    })
}
impl Joint {
    pub fn buildings(
        &self,
        solution: &FeasibleSolution,
    ) -> Result<Vec<shapez_blueprint::Building>> {
        use shapez_blueprint::{Building, Placement, Rotation, route_buildings};
        let selected: Vec<_> = self
            .choices
            .iter()
            .filter(|c| solution.value(c.enabled) == 1)
            .collect();
        let mut buildings = Vec::new();
        let mut targets = BTreeMap::new();
        let mut sources = Vec::new();
        for &p in &self.inputs {
            sources.push((0, p, true));
        }
        for c in selected {
            let cell = &c.cell;
            let z = c.layer as i16;
            buildings.extend(
                crate::design::variant(cell.out, cell.mirror)
                    .placed(Placement {
                        origin: (cell.x, cell.y, z),
                        rotation: Rotation::new(cell.r),
                    })
                    .buildings,
            );
            if cell.split >= 0 {
                let (name, r) = [
                    ("Splitter1To2LInternalVariant", 0),
                    ("Splitter1To2LInternalVariantMirrored", 0),
                    ("Splitter1To2LInternalVariantMirrored", 3),
                    ("Splitter1To2LInternalVariant", 1),
                ][cell.split as usize];
                let p = cell.position((-1, if cell.mirror { 1 } else { 2 }), z);
                buildings.push(Building::new(
                    name,
                    p.0,
                    p.1,
                    p.2,
                    Rotation::new((cell.r + r) % 4),
                ));
            }
            let input = cell.input(z);
            targets.insert((0, input.p), input.d);
            if let Some(p) = cell.continuation(z) {
                sources.push((0, p, false));
            }
            sources.push((1, cell.output(z), false));
        }
        for p in &self.outputs {
            targets.insert((1, p.p), p.d);
        }
        let next: BTreeMap<_, _> = self
            .network
            .arcs
            .iter()
            .filter(|a| solution.value(a.selected) != 0)
            .map(|a| ((a.commodity, a.from), a.to))
            .collect();
        for (commodity, port, input) in sources {
            let mut state = port.state();
            if input
                && !next.contains_key(&(commodity, state))
                && !targets.contains_key(&(commodity, port.p))
            {
                continue;
            }
            let mut steps = vec![state.step()];
            let mut seen = BTreeSet::new();
            while let Some(&to) = next.get(&(commodity, state)) {
                ensure!(seen.insert(state), "Source-connected cycle");
                state = to;
                steps.push(state.step());
            }
            let &bd = targets
                .get(&(commodity, state.point()))
                .ok_or_else(|| anyhow::anyhow!("Disconnected partial source"))?;
            route_buildings(
                &shapez_model::TileRoute {
                    net: shapez_model::TileNet {
                        a: port.p,
                        ad: port.d,
                        b: state.point(),
                        bd,
                        input,
                        output: commodity == 1,
                        exchange: None,
                    },
                    steps: steps.into_boxed_slice(),
                },
                &mut buildings,
            );
        }
        Ok(buildings)
    }
}
