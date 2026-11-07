//! Owned integer-linear models with optional native solver adapters.
use anyhow::{Result, ensure};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Var(usize);

#[derive(Clone, Default, Debug)]
pub struct Expr {
    pub terms: Vec<(Var, i64)>,
    pub constant: i64,
}
impl Expr {
    pub fn term(&mut self, variable: Var, coefficient: i64) {
        self.terms.push((variable, coefficient));
    }
    pub fn sum(variables: impl IntoIterator<Item = Var>) -> Self {
        Self {
            terms: variables.into_iter().map(|v| (v, 1)).collect(),
            constant: 0,
        }
    }
    pub fn extend(&mut self, other: &Self, scale: i64) {
        self.terms
            .extend(other.terms.iter().map(|&(v, c)| (v, c * scale)));
        self.constant += other.constant * scale;
    }
}
impl From<Var> for Expr {
    fn from(v: Var) -> Self {
        Self::sum([v])
    }
}
struct Row {
    terms: Vec<(Var, i64)>,
    lower: i64,
    upper: i64,
}
#[derive(Default)]
pub struct Model {
    domains: Vec<(i64, i64)>,
    rows: Vec<Row>,
    hints: BTreeMap<Var, i64>,
    objective: Expr,
}
impl Model {
    pub fn variable(&mut self, lower: i64, upper: i64) -> Var {
        assert!(lower <= upper, "Ordered variable domain");
        let v = Var(self.domains.len());
        self.domains.push((lower, upper));
        v
    }
    pub fn boolean(&mut self) -> Var {
        self.variable(0, 1)
    }
    pub fn flow(&mut self, enabled: Var, capacity: i64) -> Var {
        let f = self.variable(0, capacity);
        self.bounded(
            Expr {
                terms: vec![(f, 1), (enabled, -1)],
                constant: 0,
            },
            0,
            i64::MAX,
        );
        self.bounded(
            Expr {
                terms: vec![(f, 1), (enabled, -capacity)],
                constant: 0,
            },
            i64::MIN,
            0,
        );
        f
    }
    pub fn bounded(&mut self, expr: Expr, lower: i64, upper: i64) {
        let mut terms = BTreeMap::<Var, i64>::new();
        for (v, c) in expr.terms {
            *terms.entry(v).or_default() += c;
        }
        let terms = terms.into_iter().filter(|(_, c)| *c != 0).collect();
        self.rows.push(Row {
            terms,
            lower: lower.saturating_sub(expr.constant),
            upper: upper.saturating_sub(expr.constant),
        });
    }
    pub fn equal(&mut self, expr: impl Into<Expr>, value: i64) {
        self.bounded(expr.into(), value, value);
    }
    pub fn at_most_one(&mut self, vars: impl IntoIterator<Item = Var>) {
        self.bounded(Expr::sum(vars), i64::MIN, 1);
    }
    pub fn exactly_one(&mut self, vars: impl IntoIterator<Item = Var>) {
        self.equal(Expr::sum(vars), 1);
    }
    pub fn hint(&mut self, v: Var, value: i64) {
        self.hints.insert(v, value);
    }
    pub fn minimize(&mut self, expr: Expr) {
        self.objective = expr;
    }
    pub fn counts(&self) -> (usize, usize) {
        (self.domains.len(), self.rows.len())
    }
    pub fn validate(&self, solution: &Solution) -> Result<()> {
        let Some(values) = &solution.values else {
            return Ok(());
        };
        ensure!(
            values.len() == self.domains.len(),
            "Solution dimension mismatch"
        );
        for (i, (&(lower, upper), &v)) in self.domains.iter().zip(values).enumerate() {
            ensure!((lower..=upper).contains(&v), "Variable {i} violates domain");
        }
        for (i, r) in self.rows.iter().enumerate() {
            let value: i128 = r
                .terms
                .iter()
                .map(|&(v, c)| i128::from(values[v.0]) * i128::from(c))
                .sum();
            ensure!(
                value >= i128::from(r.lower) && value <= i128::from(r.upper),
                "Constraint {i} violated: {value}"
            );
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Optimal,
    Feasible,
    Infeasible,
    Unknown,
}
#[derive(Serialize)]
pub struct Solution {
    pub status: Status,
    pub seconds: f64,
    pub backend: &'static str,
    #[serde(skip)]
    values: Option<Vec<i64>>,
}
impl Solution {
    pub fn feasible(&self) -> bool {
        self.values.is_some()
    }
    pub fn value(&self, v: Var) -> i64 {
        self.values.as_ref().expect("Solution must be feasible")[v.0]
    }
}
pub struct Limits {
    pub seconds: f64,
    pub workers: i32,
    pub log: bool,
}

#[cfg(feature = "cp-sat")]
impl Model {
    pub fn solve_cp_sat(&self, limits: &Limits) -> Result<Solution> {
        use cpsat_rs::prelude::*;
        let mut model = CpModel::new();
        let mut boolean = Vec::new();
        let variables: Vec<_> = self
            .domains
            .iter()
            .map(|&(lo, hi)| {
                if (lo, hi) == (0, 1) {
                    let b = model.new_bool_var("");
                    boolean.push(Some(b));
                    b.as_int_var()
                } else {
                    boolean.push(None);
                    model.new_int_var(lo..=hi, "")
                }
            })
            .collect();
        for row in &self.rows {
            if row.upper == 1
                && row
                    .terms
                    .iter()
                    .all(|&(v, c)| c == 1 && boolean[v.0].is_some())
                && (row.lower == 1 || row.lower <= 0)
            {
                let bs: Vec<_> = row
                    .terms
                    .iter()
                    .map(|&(v, _)| boolean[v.0].expect("Checked boolean domain"))
                    .collect();
                if row.lower == 1 {
                    model.add_exactly_one(&bs);
                } else {
                    model.add_at_most_one(&bs);
                }
                continue;
            }
            let mut expression = LinearExpr::constant(0);
            for &(v, c) in &row.terms {
                expression.add_term(variables[v.0], c);
            }
            model.add(expression.between(row.lower, row.upper));
        }
        for (&v, &value) in &self.hints {
            model.add_hint(variables[v.0], value);
        }
        if !self.objective.terms.is_empty() {
            let mut objective = LinearExpr::constant(self.objective.constant);
            for &(v, c) in &self.objective.terms {
                objective.add_term(variables[v.0], c);
            }
            model.minimize(objective);
        }
        let parameters = SatParameters {
            max_time_in_seconds: Some(limits.seconds),
            num_search_workers: Some(limits.workers),
            log_search_progress: Some(limits.log),
            ..Default::default()
        };
        let result = CpSolver::solve_with_params(&model, &parameters)?;
        ensure!(
            result.status() != CpSolverStatus::ModelInvalid,
            "Invalid CP-SAT model: {}",
            result.raw_proto().solution_info
        );
        let solution = Solution {
            status: match result.status() {
                CpSolverStatus::Optimal => crate::Status::Optimal,
                CpSolverStatus::Feasible => crate::Status::Feasible,
                CpSolverStatus::Infeasible => crate::Status::Infeasible,
                _ => crate::Status::Unknown,
            },
            seconds: result.wall_time(),
            backend: "cp-sat",
            values: result
                .is_feasible()
                .then(|| variables.iter().map(|&v| result.value(v)).collect()),
        };
        self.validate(&solution)?;
        Ok(solution)
    }
}

#[cfg(feature = "scip")]
impl Model {
    pub fn solve_scip(&self, limits: &Limits) -> Result<Solution> {
        use russcip::prelude::*;
        let mut model = russcip::Model::new()
            .include_default_plugins()
            .create_prob("shapez")
            .set_real_param("limits/time", limits.seconds)
            .map_err(|e| anyhow::anyhow!("SCIP time limit: {e:?}"))?
            .set_int_param("parallel/maxnthreads", limits.workers)
            .map_err(|e| anyhow::anyhow!("SCIP worker limit: {e:?}"))?
            .set_display_verbosity(if limits.log { 4 } else { 0 });
        if self.objective.terms.is_empty() {
            model = model
                .set_int_param("limits/solutions", 1)
                .map_err(|e| anyhow::anyhow!("SCIP solution limit: {e:?}"))?;
        }
        let mut objective = vec![0.; self.domains.len()];
        for &(v, c) in &self.objective.terms {
            objective[v.0] += c as f64;
        }
        let variables: Vec<_> = self
            .domains
            .iter()
            .enumerate()
            .map(|(i, &(lo, hi))| {
                model.add_var(
                    lo as f64,
                    hi as f64,
                    objective[i],
                    &format!("v{i}"),
                    if (lo, hi) == (0, 1) {
                        VarType::Binary
                    } else {
                        VarType::Integer
                    },
                )
            })
            .collect();
        for (i, row) in self.rows.iter().enumerate() {
            model.add_cons(
                row.terms.iter().map(|(v, _)| &variables[v.0]).collect(),
                &row.terms.iter().map(|(_, c)| *c as f64).collect::<Vec<_>>(),
                if row.lower == i64::MIN {
                    f64::NEG_INFINITY
                } else {
                    row.lower as f64
                },
                if row.upper == i64::MAX {
                    f64::INFINITY
                } else {
                    row.upper as f64
                },
                &format!("r{i}"),
            );
        }
        if !self.hints.is_empty() {
            let hint = model.create_partial_sol();
            for (&v, &value) in &self.hints {
                hint.set_val(&variables[v.0], value as f64);
            }
            model
                .add_sol(hint)
                .map_err(|e| anyhow::anyhow!("SCIP hint: {e:?}"))?;
        }
        let result = if limits.workers > 1 {
            model.solve_concurrent()
        } else {
            model.solve()
        };
        let values = result
            .best_sol()
            .map(|s| variables.iter().map(|v| s.val(v).round() as i64).collect());
        let status = match result.status() {
            russcip::Status::Optimal => crate::Status::Optimal,
            russcip::Status::Infeasible => crate::Status::Infeasible,
            _ if values.is_some() => crate::Status::Feasible,
            _ => crate::Status::Unknown,
        };
        let solution = Solution {
            status,
            seconds: result.solving_time(),
            backend: "scip",
            values,
        };
        self.validate(&solution)?;
        Ok(solution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_backends_respect_capacity_and_objective() {
        let mut model = Model::default();
        let a = model.variable(0, 4);
        let b = model.variable(0, 4);
        model.equal(Expr::sum([a, b]), 5);
        model.minimize(a.into());
        let limits = Limits {
            seconds: 5.,
            workers: 1,
            log: false,
        };
        #[cfg(feature = "cp-sat")]
        {
            let s = model.solve_cp_sat(&limits).unwrap();
            assert_eq!((s.value(a), s.value(b)), (1, 4));
        }
        #[cfg(feature = "scip")]
        {
            let s = model.solve_scip(&limits).unwrap();
            assert_eq!((s.value(a), s.value(b)), (1, 4));
        }
        let _ = limits;
    }
}
