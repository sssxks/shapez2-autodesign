//! Owned integer-linear models with optional native solver adapters.
use anyhow::{Result, ensure};
use serde::Serialize;
use std::collections::BTreeMap;

/// Index of an integer variable in allocation order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Var(
    /// Allocation index into the model variable arrays.
    usize,
);

/// Mutable builder for an integer affine expression.
#[derive(Clone, Default, Debug)]
pub struct Expr {
    /// Variable coefficients; repeated variables are summed when constraining.
    pub terms: Vec<(Var, i64)>,
    /// Constant offset added to the weighted variable sum.
    pub constant: i64,
}
impl Expr {
    /// Append a weighted variable to this expression.
    pub fn term(&mut self, variable: Var, coefficient: i64) {
        self.terms.push((variable, coefficient));
    }
    /// Build the unit-weight sum of the supplied variables.
    pub fn sum(variables: impl IntoIterator<Item = Var>) -> Self {
        Self {
            terms: variables.into_iter().map(|v| (v, 1)).collect(),
            constant: 0,
        }
    }
    /// Add a scaled expression, including its constant offset.
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
/// Canonical linear constraint with merged nonzero coefficients.
struct Row {
    /// Immutable nonzero variable coefficients.
    terms: Box<[(Var, i64)]>,
    /// Inclusive lower bound.
    lower: i64,
    /// Inclusive upper bound.
    upper: i64,
}
/// Reusable finite relation, stored once before native model serialization.
#[derive(Clone, Copy)]
pub struct Table(
    /// Registration index into the finite relation array.
    usize,
);
/// Append-only integer model translated into a selected native backend.
#[derive(Default)]
pub struct Model {
    /// Inclusive variable bounds in allocation order.
    domains: Vec<(i64, i64)>,
    /// Canonical linear constraints.
    rows: Vec<Row>,
    /// Last suggested assignment for each hinted variable.
    hints: BTreeMap<Var, i64>,
    /// Mutable minimization expression replaced by minimize.
    objective: Expr,
    /// Registered immutable finite relations.
    relations: Vec<Box<[Box<[i64]>]>>,
    /// Relation applications with ordered variable arguments.
    tables: Vec<(Table, Box<[Var]>)>,
}
impl Model {
    /// Register a finite relation with a uniform nonzero arity.
    ///
    /// Panics for an empty relation, empty tuples, or unequal tuple lengths.
    pub fn relation(&mut self, rows: Vec<Vec<i64>>) -> Table {
        assert!(!rows.is_empty() && !rows[0].is_empty());
        assert!(rows.iter().all(|r| r.len() == rows[0].len()));
        let table = Table(self.relations.len());
        self.relations
            .push(rows.into_iter().map(Vec::into_boxed_slice).collect());
        table
    }
    /// Constrain variables to a tuple in a previously registered relation.
    ///
    /// Panics if the argument count differs from the relation arity.
    pub fn allow(&mut self, table: Table, variables: Vec<Var>) {
        assert_eq!(variables.len(), self.relations[table.0][0].len());
        self.tables.push((table, variables.into_boxed_slice()));
    }
    /// Allocate an integer variable; panics if bounds are reversed.
    pub fn variable(&mut self, lower: i64, upper: i64) -> Var {
        assert!(lower <= upper, "Ordered variable domain");
        let v = Var(self.domains.len());
        self.domains.push((lower, upper));
        v
    }
    /// Allocate a variable whose values are zero or one.
    pub fn boolean(&mut self) -> Var {
        self.variable(0, 1)
    }
    /// Allocate flow that is zero when disabled and between one and capacity when enabled.
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
    /// Constrain an affine expression between inclusive bounds, merging duplicate terms.
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
    /// Require an expression to equal the supplied integer.
    pub fn equal(&mut self, expr: impl Into<Expr>, value: i64) {
        self.bounded(expr.into(), value, value);
    }
    /// Require the variable sum to be at most one.
    pub fn at_most_one(&mut self, vars: impl IntoIterator<Item = Var>) {
        self.bounded(Expr::sum(vars), i64::MIN, 1);
    }
    /// Require the variable sum to equal one.
    pub fn exactly_one(&mut self, vars: impl IntoIterator<Item = Var>) {
        self.equal(Expr::sum(vars), 1);
    }
    /// Suggest a variable value to the backend without adding a constraint.
    pub fn hint(&mut self, v: Var, value: i64) {
        self.hints.insert(v, value);
    }
    /// Replace the minimization objective.
    pub fn minimize(&mut self, expr: Expr) {
        self.objective = expr;
    }
    /// Return variable and constraint counts, counting each table application once.
    pub fn counts(&self) -> (usize, usize) {
        (self.domains.len(), self.rows.len() + self.tables.len())
    }
    /// Check every domain, linear constraint and finite relation against an assignment.
    pub fn validate(&self, solution: &FeasibleSolution) -> Result<()> {
        let values = &solution.values;
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
        for (i, (table, vars)) in self.tables.iter().enumerate() {
            ensure!(
                self.relations[table.0].iter().any(|row| row
                    .iter()
                    .zip(vars)
                    .all(|(&wanted, v)| values[v.0] == wanted)),
                "Finite relation {i} violated"
            );
        }
        Ok(())
    }
}
/// Serializable classification derived from the solve outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    /// Optimal assignment found.
    Optimal,
    /// Incumbent found.
    Feasible,
    /// No satisfying assignment exists.
    Infeasible,
    /// Search stopped without a definitive result.
    Unknown,
}
/// Solver outcome; only incumbent-bearing variants expose variable values.
///
/// Extract an assignment by matching `Optimal` or `Feasible` before reading it.
/// An outcome itself deliberately has no value accessor:
/// ```compile_fail
/// use shapez_solver::{Solution, Var};
/// fn read(outcome: &Solution, variable: Var) -> i64 {
///     outcome.value(variable)
/// }
/// ```
#[derive(Debug)]
pub enum Solution {
    /// Globally optimal assignment.
    Optimal(FeasibleSolution),
    /// Incumbent found before optimality was established.
    Feasible(FeasibleSolution),
    /// Model has no satisfying assignment.
    Infeasible(Metadata),
    /// Search ended without an incumbent or an infeasibility result.
    Unknown(Metadata),
}
/// Timing and backend recorded by one native solve.
#[derive(Debug, Serialize)]
pub struct Metadata {
    /// Native solver wall time in seconds.
    seconds: f64,
    /// Adapter that performed the solve.
    backend: &'static str,
}
impl Metadata {
    /// Native solver wall time in seconds.
    pub fn seconds(&self) -> f64 {
        self.seconds
    }
    /// Adapter name.
    pub fn backend(&self) -> &'static str {
        self.backend
    }
}
/// Complete integer assignment returned by a native solver.
#[derive(Debug)]
pub struct FeasibleSolution {
    /// Values in variable allocation order.
    values: Box<[i64]>,
    /// Measurements for the solve producing this assignment.
    metadata: Metadata,
}
impl FeasibleSolution {
    /// Read a variable's assigned integer value.
    pub fn value(&self, v: Var) -> i64 {
        self.values[v.0]
    }
    /// Measurements for this assignment's solve.
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }
}
impl Solution {
    /// Classification derived solely from the outcome variant.
    pub fn status(&self) -> Status {
        match self {
            Self::Optimal(_) => Status::Optimal,
            Self::Feasible(_) => Status::Feasible,
            Self::Infeasible(_) => Status::Infeasible,
            Self::Unknown(_) => Status::Unknown,
        }
    }
    /// Measurements shared by all outcome kinds.
    pub fn metadata(&self) -> &Metadata {
        match self {
            Self::Optimal(s) | Self::Feasible(s) => &s.metadata,
            Self::Infeasible(m) | Self::Unknown(m) => m,
        }
    }
}
/// Native search resource and logging settings.
pub struct Limits {
    /// Maximum solve time in seconds.
    pub seconds: f64,
    /// Requested number of native worker threads.
    pub workers: i32,
    /// Whether native search progress is printed.
    pub log: bool,
}

#[cfg(feature = "cp-sat")]
impl Model {
    /// Translate and solve with CP-SAT, validating the returned incumbent.
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
        let mut bs = Vec::new();
        for row in &self.rows {
            if row.upper == 1
                && row
                    .terms
                    .iter()
                    .all(|&(v, c)| c == 1 && boolean[v.0].is_some())
                && (row.lower == 1 || row.lower <= 0)
            {
                bs.clear();
                bs.extend(row.terms.iter().filter_map(|&(v, _)| boolean[v.0]));
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
        let relations: Vec<Vec<Vec<i64>>> = self
            .relations
            .iter()
            .map(|rows| rows.iter().map(|r| r.to_vec()).collect())
            .collect();
        let mut table_vars = Vec::new();
        for (table, vars) in &self.tables {
            table_vars.clear();
            table_vars.extend(vars.iter().map(|v| variables[v.0]));
            model.add_table(&table_vars, &relations[table.0], false);
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
        let metadata = Metadata {
            seconds: result.wall_time(),
            backend: "cp-sat",
        };
        let solution = match result.status() {
            CpSolverStatus::Optimal | CpSolverStatus::Feasible => {
                let feasible = FeasibleSolution {
                    values: variables.iter().map(|&v| result.value(v)).collect(),
                    metadata,
                };
                self.validate(&feasible)?;
                if result.status() == CpSolverStatus::Optimal {
                    Solution::Optimal(feasible)
                } else {
                    Solution::Feasible(feasible)
                }
            }
            CpSolverStatus::Infeasible => Solution::Infeasible(metadata),
            _ => Solution::Unknown(metadata),
        };
        Ok(solution)
    }
}

#[cfg(feature = "scip")]
impl Model {
    /// Solve linear constraints with SCIP; finite relations return an error.
    pub fn solve_scip(&self, limits: &Limits) -> Result<Solution> {
        ensure!(
            self.tables.is_empty(),
            "Finite relations require the CP-SAT backend"
        );
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
        let mut coefficients = Vec::new();
        for (i, row) in self.rows.iter().enumerate() {
            coefficients.clear();
            coefficients.extend(row.terms.iter().map(|(_, c)| *c as f64));
            model.add_cons(
                row.terms.iter().map(|(v, _)| &variables[v.0]).collect(),
                &coefficients,
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
        let metadata = Metadata {
            seconds: result.solving_time(),
            backend: "scip",
        };
        let solution = if let Some(incumbent) = result.best_sol() {
            let feasible = FeasibleSolution {
                values: variables
                    .iter()
                    .map(|v| incumbent.val(v).round() as i64)
                    .collect(),
                metadata,
            };
            self.validate(&feasible)?;
            if result.status() == russcip::Status::Optimal {
                Solution::Optimal(feasible)
            } else {
                Solution::Feasible(feasible)
            }
        } else if result.status() == russcip::Status::Infeasible {
            Solution::Infeasible(metadata)
        } else {
            Solution::Unknown(metadata)
        };
        Ok(solution)
    }
}

/// Outcome invariant and native adapter regression tests.
#[cfg(test)]
mod tests {
    use super::*;
    /// Exercise all outcome variants and reject invalid incumbent assignments.
    #[test]
    fn outcomes_and_validation() {
        let metadata = || Metadata {
            seconds: 0.25,
            backend: "test",
        };
        for (outcome, status) in [
            (Solution::Infeasible(metadata()), Status::Infeasible),
            (Solution::Unknown(metadata()), Status::Unknown),
        ] {
            assert_eq!(outcome.status(), status);
            assert_eq!(outcome.metadata().seconds(), 0.25);
        }
        let mut model = Model::default();
        let x = model.variable(0, 2);
        model.equal(x, 1);
        for optimal in [false, true] {
            let feasible = FeasibleSolution {
                values: Box::new([1]),
                metadata: metadata(),
            };
            model.validate(&feasible).unwrap();
            assert_eq!(feasible.value(x), 1);
            let outcome = if optimal {
                Solution::Optimal(feasible)
            } else {
                Solution::Feasible(feasible)
            };
            assert_eq!(
                outcome.status(),
                if optimal {
                    Status::Optimal
                } else {
                    Status::Feasible
                }
            );
            assert_eq!(outcome.metadata().backend(), "test");
        }
        for values in [vec![], vec![3], vec![2]] {
            assert!(
                model
                    .validate(&FeasibleSolution {
                        values: values.into_boxed_slice(),
                        metadata: metadata()
                    })
                    .is_err()
            );
        }
    }
    /// Contradictory equalities must yield infeasible on every enabled adapter.
    #[cfg(any(feature = "cp-sat", feature = "scip"))]
    #[test]
    fn native_infeasible() {
        let mut model = Model::default();
        let x = model.boolean();
        model.equal(x, 0);
        model.equal(x, 1);
        let limits = Limits {
            seconds: 5.,
            workers: 1,
            log: false,
        };
        #[cfg(feature = "cp-sat")]
        assert!(matches!(
            model.solve_cp_sat(&limits).unwrap(),
            Solution::Infeasible(_)
        ));
        #[cfg(feature = "scip")]
        assert!(matches!(
            model.solve_scip(&limits).unwrap(),
            Solution::Infeasible(_)
        ));
    }

    /// Verify finite relations preserve nonlinear tuples under minimization.
    #[cfg(feature = "cp-sat")]
    #[test]
    fn finite_relation_preserves_non_linear_pairs() {
        let mut model = Model::default();
        let x = model.variable(2, 3);
        let y = model.variable(0, 9);
        let squares = model.relation(vec![vec![0, 0], vec![1, 1], vec![2, 4], vec![3, 9]]);
        model.allow(squares, vec![x, y]);
        model.minimize(y.into());
        let solution = model
            .solve_cp_sat(&Limits {
                seconds: 5.,
                workers: 1,
                log: false,
            })
            .unwrap();
        let Solution::Optimal(solution) = solution else {
            panic!("Expected optimum")
        };
        assert_eq!((solution.value(x), solution.value(y)), (2, 4));
    }
    /// Verify both adapters honor domains, equalities and minimization.
    #[cfg(any(feature = "cp-sat", feature = "scip"))]
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
            let Solution::Optimal(s) = s else {
                panic!("Expected optimum")
            };
            assert_eq!((s.value(a), s.value(b)), (1, 4));
        }
        #[cfg(feature = "scip")]
        {
            let s = model.solve_scip(&limits).unwrap();
            let Solution::Optimal(s) = s else {
                panic!("Expected optimum")
            };
            assert_eq!((s.value(a), s.value(b)), (1, 4));
        }
        let _ = limits;
    }
}
