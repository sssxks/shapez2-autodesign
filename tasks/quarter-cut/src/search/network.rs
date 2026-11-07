use super::geometry::{Point, Port, State, shift};
use anyhow::{Result, ensure};
use shapez_solver::{Expr, Model, Var};
use std::collections::{BTreeMap, BTreeSet};
pub type Key = (u8, State);
pub type Witness = BTreeMap<(u8, State, State), i64>;
#[derive(Clone, Copy)]
pub struct Arc {
    pub commodity: u8,
    pub from: State,
    pub to: State,
    pub selected: Var,
    pub amount: Var,
}
type Jump = (Point, Point, usize);
#[derive(Default)]
pub struct Network {
    pub model: Model,
    pub balance: BTreeMap<Key, Expr>,
    pub capacity: BTreeMap<Point, Expr>,
    pub fanin: BTreeMap<(u8, Point), Expr>,
    pub arcs: Vec<Arc>,
    jumps: BTreeMap<Jump, Vec<Var>>,
}
impl Network {
    pub fn add_flow(&mut self, key: Key, flow: &Expr, scale: i64) {
        self.balance.entry(key).or_default().extend(flow, scale);
    }
    pub fn occupy(&mut self, p: Point, v: Var) {
        self.capacity.entry(p).or_default().term(v, 1);
    }
    pub fn emit(&mut self, commodity: u8, port: Port, amount: Expr, enabled: Var) {
        self.add_flow((commodity, port.state()), &amount, 1);
        self.fanin
            .entry((commodity, port.p))
            .or_default()
            .term(enabled, 1);
    }
    pub fn receive(&mut self, commodity: u8, port: Port, max: i64, enabled: Expr, amount: Expr) {
        let mut flags = Expr::default();
        let mut flows = Expr::default();
        for d in 0..4 {
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
                    (0..4).flat_map(move |d| [false, true].map(|caught| State::new(p, d, caught)))
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
                    for d in 0..4 {
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
        let mut receivers = BTreeMap::<(Point, usize), Vec<Jump>>::new();
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
