"""Run the linear routing model through the native SCIP/SoPlex backend."""

from ortools.linear_solver import pywraplp
from ortools.sat.python import cp_model


class MipSolver:
    def __init__(self, seconds: float, workers: int):
        self.solver = pywraplp.Solver.CreateSolver("SCIP")
        if self.solver is None:
            raise RuntimeError("This OR-Tools package does not include SCIP")
        self.solver.SetTimeLimit(int(seconds * 1000))
        self.solver.SetNumThreads(workers)
        self.solver.SetSolverSpecificParametersAsString("limits/solutions = 1")
        self.solver.EnableOutput()
        self.variables = []

    def solve(self, model: cp_model.CpModel):
        infinity = self.solver.infinity()
        for v in model.proto.variables:
            if len(v.domain) != 2:
                raise ValueError("SCIP conversion requires interval variable domains")
            self.variables.append(self.solver.IntVar(v.domain[0], v.domain[1], v.name))
        for i, c in enumerate(model.proto.constraints):
            if c.enforcement_literal:
                raise ValueError(
                    "SCIP conversion requires unconditional linear constraints"
                )
            if c.has_linear():
                if len(c.linear.domain) != 2:
                    raise ValueError("SCIP conversion requires interval linear domains")
                lower, upper = c.linear.domain
                lower = -infinity if lower < -(1 << 60) else lower
                upper = infinity if upper > (1 << 60) else upper
                constraint = self.solver.RowConstraint(lower, upper, f"c{i}")
                for index, coefficient in zip(c.linear.vars, c.linear.coeffs):
                    constraint.SetCoefficient(self.variables[index], coefficient)
            elif c.has_exactly_one() or c.has_bool_or():
                literals = (
                    c.exactly_one.literals
                    if c.has_exactly_one()
                    else c.bool_or.literals
                )
                constant = sum(index < 0 for index in literals)
                constraint = self.solver.RowConstraint(
                    1 - constant,
                    1 - constant if c.has_exactly_one() else infinity,
                    f"c{i}",
                )
                coefficients = {}
                for literal in literals:
                    index = literal if literal >= 0 else -literal - 1
                    coefficients[index] = coefficients.get(index, 0) + (
                        1 if literal >= 0 else -1
                    )
                for index, coefficient in coefficients.items():
                    constraint.SetCoefficient(self.variables[index], coefficient)
            else:
                raise ValueError(f"Unsupported constraint in linear routing model: {c}")
        hint = model.proto.solution_hint
        self.solver.SetHint([self.variables[i] for i in hint.vars], list(hint.values))
        result = self.solver.Solve()
        return {
            pywraplp.Solver.OPTIMAL: cp_model.OPTIMAL,
            pywraplp.Solver.FEASIBLE: cp_model.FEASIBLE,
            pywraplp.Solver.INFEASIBLE: cp_model.INFEASIBLE,
        }.get(result, cp_model.UNKNOWN)

    def value(self, variable: cp_model.IntVar) -> int:
        return round(self.variables[variable.index].solution_value())

    @property
    def wall_time(self) -> float:
        return self.solver.WallTime() / 1000

    @staticmethod
    def status_name(status) -> str:
        return {
            cp_model.OPTIMAL: "OPTIMAL",
            cp_model.FEASIBLE: "FEASIBLE",
            cp_model.INFEASIBLE: "INFEASIBLE",
            cp_model.UNKNOWN: "UNKNOWN",
        }[status]

    def response_stats(self) -> str:
        return (
            f"{self.solver.SolverVersion()}: {self.solver.NumVariables()} variables, "
            f"{self.solver.NumConstraints()} constraints, {self.solver.nodes()} nodes, "
            f"{self.solver.iterations()} LP iterations, {self.wall_time} seconds"
        )
