//! Multi-commodity transport constraints shared by candidate cell placements.
use super::geometry::{Point, Port, State, shift};
use anyhow::{Result, ensure};
use shapez_solver::{Expr, Model, Var};
use std::collections::{BTreeMap, BTreeSet};
/// Material class and directed state whose net flow must balance.
pub type Key = (u8, State);
/// Restart flow amounts indexed by material class and directed move endpoints.
pub type Witness = BTreeMap<(u8, State, State), i64>;
/// A candidate transport move and its solver variables.
#[derive(Clone, Copy)]
pub struct Arc {
    /// Material class transported by this move.
    pub commodity: u8,
    /// Departure state including arrival direction and jump history.
    pub from: State,
    /// Arrival state after the move.
    pub to: State,
    /// Boolean selecting this physical transport move.
    pub selected: Var,
    /// Integer flow; aliases selection for unit-capacity material.
    pub amount: Var,
}
/// Launcher tile, catcher tile, and the shared straight-line travel heading.
type Jump = (Point, Point, shapez_model::Direction);
/// Transport model with pending conservation and physical occupancy constraints.
#[derive(Default)]
pub struct Network {
    /// Variables and constraints shared with cell placement.
    pub model: Model,
    /// Signed net inflow accumulated for each material state.
    pub balance: BTreeMap<Key, Expr>,
    /// Selected objects competing for each physical tile.
    pub capacity: BTreeMap<Point, Expr>,
    /// Selected arrivals competing for each material input tile.
    pub fanin: BTreeMap<(u8, Point), Expr>,
    /// Candidate moves retained for hints, checkpoints and blueprint extraction.
    pub arcs: Vec<Arc>,
    /// Selection variables grouped by physical jump for interference constraints.
    jumps: BTreeMap<Jump, Vec<Var>>,
}
impl Network {
    /// Add a signed contribution to a material conservation equation.
    pub fn add_flow(&mut self, key: Key, flow: &Expr, scale: i64) {
        self.balance.entry(key).or_default().extend(flow, scale);
    }
    /// Reserve a tile whenever the selection variable is true.
    pub fn occupy(&mut self, p: Point, v: Var) {
        self.capacity.entry(p).or_default().term(v, 1);
    }
    /// Inject cell output into transport and count its arrival at the port.
    pub fn emit(&mut self, commodity: u8, port: Port, amount: Expr, enabled: Var) {
        self.add_flow((commodity, port.state()), &amount, 1);
        self.fanin
            .entry((commodity, port.p))
            .or_default()
            .term(enabled, 1);
    }
    /// Consume cell input through one legal arrival state when enabled.
    pub fn receive(&mut self, commodity: u8, port: Port, max: i64, enabled: Expr, amount: Expr) {
        let mut flags = Expr::default();
        let mut flows = Expr::default();
        for d in shapez_model::Direction::ALL {
            for caught in [false, true] {
                let state = State::new(port.p, d, caught);
                if !state.step().allows_destination(port.d, false) {
                    continue;
                }
                let v = self.model.boolean();
                let f = self.model.flow(v, max);
                self.add_flow((commodity, state), &f.into(), -1);
                self.occupy(port.p, v);
                flags.term(v, 1);
                flows.term(f, 1);
            }
        }
        flows.extend(&amount, -1);
        flags.extend(&enabled, -1);
        self.model.equal(flows, 0);
        self.model.equal(flags, 0);
    }
    /// Enumerate legal moves; a witness fixes all move amounts and rejects unsupported moves.
    pub fn transport(
        &mut self,
        free: &BTreeSet<Point>,
        inputs: &[Port],
        outputs: &[Port],
        capacities: &[i64],
        witness: Option<&Witness>,
    ) -> Result<()> {
        let input_map: BTreeMap<_, _> = inputs.iter().map(|p| (p.p, p.d)).collect();
        let output_map: BTreeMap<_, _> = outputs.iter().map(|p| (p.p, p.d)).collect();
        let mut seen = BTreeSet::new();
        for (c, &max_flow) in capacities.iter().enumerate() {
            let commodity = c as u8;
            let states: Vec<_> = free
                .iter()
                .flat_map(|&p| {
                    shapez_model::Direction::ALL
                        .into_iter()
                        .flat_map(move |d| [false, true].map(|caught| State::new(p, d, caught)))
                })
                .chain(
                    if c == 0 {
                        inputs
                    } else if c + 1 == capacities.len() {
                        outputs
                    } else {
                        &[]
                    }
                    .iter()
                    .map(|p| p.state()),
                )
                .collect();
            for from in states {
                let p = from.point();
                if output_map.contains_key(&p) {
                    continue;
                }
                for z in 0..3 {
                    let shaft: Vec<_> = (p.2.min(z)..=p.2.max(z))
                        .filter(|&k| k != p.2)
                        .map(|k| (p.0, p.1, k))
                        .collect();
                    if shaft.iter().any(|p| !free.contains(p)) {
                        continue;
                    }
                    for d in shapez_model::Direction::ALL {
                        for length in 1..=5 {
                            let jump = length > 1;
                            if !from
                                .step()
                                .allows_move(d, z, jump, input_map.contains_key(&p))
                            {
                                continue;
                            }
                            let q = shift(p, d, length, z);
                            if !(free.contains(&q)
                                || c + 1 == capacities.len()
                                    && !jump
                                    && output_map.get(&q) == Some(&d))
                            {
                                continue;
                            }
                            let to = State::new(q, d, jump);
                            let selected = self.model.boolean();
                            let amount = if max_flow == 1 {
                                selected
                            } else {
                                self.model.flow(selected, max_flow)
                            };
                            if let Some(witness) = witness {
                                let key = (commodity, from, to);
                                let value = witness.get(&key).copied().unwrap_or(0);
                                self.model.equal(selected, i64::from(value != 0));
                                self.model.equal(amount, value);
                                if value != 0 {
                                    seen.insert(key);
                                }
                            }
                            self.add_flow((commodity, from), &amount.into(), -1);
                            self.add_flow((commodity, to), &amount.into(), 1);
                            self.fanin
                                .entry((commodity, q))
                                .or_default()
                                .term(selected, 1);
                            self.occupy(p, selected);
                            for &p in &shaft {
                                self.occupy(p, selected);
                            }
                            self.arcs.push(Arc {
                                commodity,
                                from,
                                to,
                                selected,
                                amount,
                            });
                            if jump {
                                self.jumps.entry((p, q, d)).or_default().push(selected);
                            }
                        }
                    }
                }
            }
        }
        if let Some(witness) = witness {
            ensure!(
                witness.keys().all(|k| seen.contains(k)),
                "Native witness contains unsupported transport moves"
            );
        }
        Ok(())
    }
    /// Apply fixed per-port supply for material zero and demand for the last material.
    pub fn external(
        &mut self,
        inputs: &[Port],
        outputs: &[Port],
        last: u8,
        input_rate: i64,
        output_rate: i64,
    ) {
        for p in inputs {
            self.balance.entry((0, p.state())).or_default().constant += input_rate;
            self.fanin.entry((0, p.p)).or_default().constant += 1;
        }
        for p in outputs {
            self.balance.entry((last, p.state())).or_default().constant -= output_rate;
            self.capacity.entry(p.p).or_default().constant += 1;
        }
    }
    /// Emit pending balance and occupancy constraints and forbid interfering jump pairs.
    pub fn finish(&mut self) {
        for (_, expr) in std::mem::take(&mut self.balance) {
            self.model.equal(expr, 0);
        }
        for (_, expr) in std::mem::take(&mut self.capacity).into_iter().chain(
            std::mem::take(&mut self.fanin)
                .into_iter()
                .map(|((_, p), e)| (p, e)),
        ) {
            self.model.bounded(expr, i64::MIN, 1);
        }
        let mut receivers = BTreeMap::<(Point, shapez_model::Direction), Vec<Jump>>::new();
        for &jump in self.jumps.keys() {
            receivers.entry((jump.1, jump.2)).or_default().push(jump);
        }
        let mut pairs = BTreeSet::new();
        for &a in self.jumps.keys() {
            for k in 2..=5 {
                let p = shift(a.0, a.2, k, a.0.2);
                if p == a.1 {
                    continue;
                }
                if let Some(others) = receivers.get(&(p, a.2)) {
                    for &b in others {
                        if a != b {
                            pairs.insert((a.min(b), a.max(b)));
                        }
                    }
                }
            }
        }
        for (a, b) in pairs {
            self.model
                .at_most_one(self.jumps[&a].iter().chain(&self.jumps[&b]).copied());
        }
    }
}
