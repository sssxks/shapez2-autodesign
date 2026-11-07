//! Match input terminals while enforcing the four conservation stages.
use anyhow::Result;
use varisat::{ExtendFormula, Lit, Solver};

fn exactly_one(solver: &mut Solver<'_>, values: &[Lit]) {
    solver.add_clause(values);
    if values.len() < 2 {
        return;
    }
    let prefix: Vec<_> = (0..values.len() - 1).map(|_| solver.new_lit()).collect();
    solver.add_clause(&[!values[0], prefix[0]]);
    for i in 1..values.len() - 1 {
        solver.add_clause(&[!values[i], prefix[i]]);
        solver.add_clause(&[!prefix[i - 1], prefix[i]]);
        solver.add_clause(&[!values[i], !prefix[i - 1]]);
    }
    solver.add_clause(&[!values[values.len() - 1], !prefix[prefix.len() - 1]]);
}

/// An external belt feeds four cells. A continuation advances exactly one
/// stage, eliminating self-feeding cycles and every chain of the wrong length.
pub fn assign(
    reachable: &[Vec<usize>],
    continuations: &[Option<usize>],
) -> Result<Option<Vec<usize>>> {
    for limit in [4, 8, 12, 20, 32, 48] {
        let local: Vec<_> = reachable
            .iter()
            .map(|v| v.iter().copied().take(limit).collect())
            .collect();
        if let Some(result) = assign_limited(&local, continuations)? {
            eprintln!("Supply matched with at most {limit} nearest candidates per source");
            return Ok(Some(result));
        }
    }
    Ok(None)
}

fn assign_limited(
    reachable: &[Vec<usize>],
    continuations: &[Option<usize>],
) -> Result<Option<Vec<usize>>> {
    let count = continuations.len();
    let roots = count - continuations.iter().flatten().count();
    let mut solver = Solver::new();
    let stages: Vec<[Lit; 4]> = (0..count)
        .map(|_| std::array::from_fn(|_| solver.new_lit()))
        .collect();
    for (i, stage) in stages.iter().enumerate() {
        exactly_one(&mut solver, stage);
        solver.add_clause(&[if continuations[i].is_some() {
            !stage[3]
        } else {
            stage[3]
        }]);
    }
    let mut owner = vec![None; count];
    for (cell, source) in continuations.iter().enumerate() {
        if let Some(source) = source {
            owner[*source] = Some(cell);
        }
    }
    let mut incoming = vec![Vec::new(); count];
    let mut arcs = Vec::new();
    for source in 0..count {
        let mut outgoing = Vec::new();
        for &target in &reachable[source] {
            if target >= count || owner[source] == Some(target) {
                continue;
            }
            let edge = solver.new_lit();
            outgoing.push(edge);
            incoming[target].push(edge);
            arcs.push((source, target, edge));
            if source < roots {
                solver.add_clause(&[!edge, stages[target][0]]);
            } else if let Some(cell) = owner[source] {
                for level in 0..3 {
                    solver.add_clause(&[!edge, !stages[cell][level], stages[target][level + 1]]);
                }
            }
        }
        exactly_one(&mut solver, &outgoing);
    }
    for row in &incoming {
        exactly_one(&mut solver, row);
    }
    if !solver.solve()? {
        return Ok(None);
    }
    let model = solver
        .model()
        .expect("Satisfiable supply graph has a model");
    let mut result = vec![usize::MAX; count];
    for (source, target, edge) in arcs {
        if model.contains(&edge) {
            result[source] = target;
        }
    }
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::assign;

    #[test]
    fn supplies_four_cells_from_one_external_belt() {
        let continuations = [Some(1), Some(2), Some(3), None];
        let reachable = vec![vec![0], vec![1], vec![2], vec![3]];
        assert_eq!(
            assign(&reachable, &continuations).unwrap(),
            Some(vec![0, 1, 2, 3])
        );
    }

    #[test]
    fn rejects_a_perfect_matching_that_only_feeds_itself() {
        let continuations = [Some(1), Some(2), Some(3), None];
        let reachable = vec![vec![3], vec![1], vec![2], vec![0]];
        assert_eq!(assign(&reachable, &continuations).unwrap(), None);
    }
}
