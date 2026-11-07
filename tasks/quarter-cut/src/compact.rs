//! Native rigid-cell placement plus shared three-dimensional transport.
use anyhow::Result;
use serde::Deserialize;
use shapez_blueprint::{Building, Placement, Rotation};
use shapez_model::{
    Direction, ExchangeGroup, FoundationGeometry, FoundationSocket, TileNet, TilePoint,
    TileRoutingProblem,
};
use std::collections::HashSet;

pub fn sockets() -> [FoundationSocket; 5] {
    sockets_for(std::env::var("QUARTER_PORTS").as_deref() == Ok("west"))
}

/// Explicit interface selection for delivered templates and experimental layouts.
pub fn sockets_for(west: bool) -> [FoundationSocket; 5] {
    let definitions = if !west {
        [
            (Direction::North, 0, false, "input"),
            (Direction::West, 0, true, "west"),
            (Direction::East, 0, true, "east"),
            (Direction::South, 0, true, "sw"),
            (Direction::South, 1, true, "se"),
        ]
    } else {
        [
            (Direction::West, 0, false, "input"),
            (Direction::North, 0, true, "nw"),
            (Direction::North, 1, true, "ne"),
            (Direction::South, 0, true, "sw"),
            (Direction::South, 1, true, "se"),
        ]
    };
    definitions.map(|(side, segment, output, role)| FoundationSocket {
        side,
        segment,
        output,
        role,
    })
}
#[derive(Deserialize, serde::Serialize)]
struct PackedCell {
    #[serde(default)]
    mirror: bool,
    x: i32,
    y: i32,
    r: usize,
    out: u8,
    split: i32,
    a: (i32, i32),
    ad: usize,
    b: Option<(i32, i32)>,
    bd: usize,
    op: (i32, i32),
    od: usize,
}
struct Ports {
    input: TilePoint,
    id: usize,
    continuation: Option<TilePoint>,
    cd: usize,
    output: TilePoint,
    od: usize,
}
pub fn build() -> Result<Vec<Building>> {
    let path = std::env::var("QUARTER_PACK")?;
    let packed: Vec<Vec<PackedCell>> = serde_json::from_slice(&std::fs::read(path)?)?;
    anyhow::ensure!(
        packed.len() == 3 && packed.iter().all(|v| v.len() == 16),
        "Expected sixteen cells per layer"
    );
    let sockets = sockets();
    let mut geometry = FoundationGeometry::strip(2, 3);
    let mut buildings = Vec::new();
    let mut fixed = HashSet::new();
    let mut nets = Vec::new();
    let mut ports = Vec::new();
    for z in 0..3 {
        for c in &packed[z as usize] {
            let cell = crate::design::variant(c.out, c.mirror).placed(Placement {
                origin: (c.x, c.y, z),
                rotation: Rotation::new(c.r),
            });
            fixed.extend(cell.footprint.iter().copied());
            let input = cell.port("input")?.position;
            buildings.extend(cell.buildings);
            if c.split >= 0 {
                let (kind, r) = [
                    ("Splitter1To2LInternalVariant", 0),
                    ("Splitter1To2LInternalVariantMirrored", 0),
                    ("Splitter1To2LInternalVariantMirrored", 3),
                    ("Splitter1To2LInternalVariant", 1),
                ][c.split as usize];
                fixed.insert(input);
                buildings.push(Building::new(
                    kind,
                    input.0,
                    input.1,
                    z,
                    Rotation::new((c.r + r) % 4),
                ));
            }
            ports.push(Ports {
                input: (c.a.0, c.a.1, z),
                id: c.ad,
                continuation: c.b.map(|p| (p.0, p.1, z)),
                cd: c.bd,
                output: (c.op.0, c.op.1, z),
                od: c.od,
            });
        }
    }
    let distance = |a: TilePoint, b: TilePoint| {
        (a.0 - b.0).abs() + (a.1 - b.1).abs() + 4 * i32::from((a.2 - b.2).abs())
    };
    let mut sources = Vec::new();
    let mut continuation_sources = vec![None; ports.len()];
    for z in 0..3 {
        for lane in 0..4 {
            sources.push((
                geometry.socket_tile(sockets[0], lane, z),
                sockets[0].direction(),
                true,
            ));
        }
    }
    for (i, p) in ports.iter().enumerate() {
        if let Some(a) = p.continuation {
            continuation_sources[i] = Some(sources.len());
            sources.push((a, p.cd, false));
        }
    }
    for (i, (a, ad, input)) in sources.into_iter().enumerate() {
        nets.push(TileNet {
            a,
            ad,
            b: ports[i].input,
            bd: ports[i].id,
            input,
            output: false,
            exchange: Some(ExchangeGroup(0)),
        });
    }
    let targets: Vec<_> = sockets[1..]
        .iter()
        .flat_map(|&s| (0..3).flat_map(move |z| (0..4).map(move |k| (s, k, z))))
        .map(|(s, k, z)| (geometry.socket_tile(s, k, z), s.direction()))
        .collect();
    let mut assignment: Vec<_> = (0..targets.len()).collect();
    loop {
        let mut changed = false;
        for i in 0..ports.len() {
            for j in i + 1..ports.len() {
                if distance(ports[i].output, targets[assignment[j]].0)
                    + distance(ports[j].output, targets[assignment[i]].0)
                    < distance(ports[i].output, targets[assignment[i]].0)
                        + distance(ports[j].output, targets[assignment[j]].0)
                {
                    assignment.swap(i, j);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    for (i, p) in ports.iter().enumerate() {
        let (b, bd) = targets[assignment[i]];
        nets.push(TileNet {
            a: p.output,
            ad: p.od,
            b,
            bd,
            input: false,
            output: true,
            exchange: Some(ExchangeGroup(4)),
        });
    }
    for (i, n) in nets.iter().enumerate() {
        anyhow::ensure!(
            !fixed.contains(&n.a) && !fixed.contains(&n.b),
            "Connection {i} overlaps machinery: {:?} {:?}",
            n.a,
            n.b
        );
        if n.output {
            let d = shapez_model::TILE_DIRECTIONS[n.bd];
            geometry
                .exits
                .insert((n.b.0 - d.0, n.b.1 - d.1, n.b.2), n.b);
        }
    }
    let mut problem = TileRoutingProblem {
        geometry,
        fixed,
        nets,
    };
    if let Ok(path) = std::env::var("QUARTER_NETWORK_EXPORT") {
        let net_data: Vec<_> = problem.nets.iter().map(|n| serde_json::json!({"a":n.a,"ad":n.ad,"b":n.b,"bd":n.bd,"input":n.input,"output":n.output})).collect();
        std::fs::write(
            path,
            serde_json::to_vec(
                &serde_json::json!({"fixed":problem.fixed,"nets":net_data,"continuations":continuation_sources,"packed":packed}),
            )?,
        )?;
        anyhow::bail!("Exported compact transport network");
    }
    if let Ok(path) = std::env::var("QUARTER_ROUTES") {
        #[derive(Deserialize)]
        struct SolvedRoute {
            source: usize,
            target: Option<usize>,
            destination: Option<TilePoint>,
            steps: Vec<(i32, i32, i16, usize, bool)>,
        }
        let routes: Vec<SolvedRoute> = serde_json::from_slice(&std::fs::read(path)?)?;
        anyhow::ensure!(
            routes.len() == problem.nets.len(),
            "Expected all 96 transport routes"
        );
        let mut occupied = problem.fixed.clone();
        let mut sources_seen = HashSet::new();
        let mut targets_seen = HashSet::new();
        for route in routes {
            anyhow::ensure!(sources_seen.insert(route.source), "Duplicate route source");
            let mut net = problem.nets[route.source].clone();
            let target = if let Some(target) = route.target {
                target
            } else {
                problem
                    .nets
                    .iter()
                    .position(|n| n.output == net.output && Some(n.b) == route.destination)
                    .ok_or_else(|| anyhow::anyhow!("Unknown imported destination"))?
            };
            anyhow::ensure!(targets_seen.insert(target), "Duplicate route destination");
            net.b = problem.nets[target].b;
            net.bd = problem.nets[target].bd;
            let steps: Vec<_> = route
                .steps
                .into_iter()
                .map(|(x, y, z, d, catch)| shapez_model::TileRouteStep {
                    p: (x, y, z),
                    d,
                    catch,
                })
                .collect();
            anyhow::ensure!(
                steps.first().is_some_and(|s| s.p == net.a)
                    && steps.last().is_some_and(|s| s.p == net.b),
                "Imported route endpoints disagree with native ports"
            );
            for resource in shapez_model::tile_resources(&steps) {
                if let shapez_model::TileResource::Occupancy(p) = resource {
                    anyhow::ensure!(occupied.insert(p), "Imported routes overlap at {p:?}");
                }
            }
            shapez_blueprint::route_buildings(
                &shapez_model::TileRoute { net, steps },
                &mut buildings,
            );
        }
        return Ok(buildings);
    }
    let mut reachable = shapez_routing::reachable_targets(&problem);
    for (cell, source) in continuation_sources.iter().enumerate() {
        if let Some(source) = source {
            reachable[*source].retain(|&target| target != cell);
        }
    }
    let inaccessible: Vec<_> = (0..48).filter(|&i| reachable[48 + i].is_empty()).collect();
    for &i in &inaccessible {
        eprintln!("Isolated output {i}: {:?}", ports[i].output);
    }
    if let Ok(path) = std::env::var("QUARTER_REACH") {
        std::fs::write(path, serde_json::to_vec(&reachable)?)?;
    }
    if let Ok(path) = std::env::var("QUARTER_FEEDBACK") {
        let mut diagnostic = TileRoutingProblem {
            geometry: FoundationGeometry {
                min: problem.geometry.min,
                max: problem.geometry.max,
                layers: 3,
                exits: problem.geometry.exits.clone(),
            },
            fixed: problem.fixed.clone(),
            nets: problem.nets.clone(),
        };
        let mut owners = vec![None; 12];
        owners.extend(
            (0..48)
                .filter(|&i| continuation_sources[i].is_some())
                .map(Some),
        );
        owners.extend((0..48).map(Some));
        for (i, p) in ports.iter().enumerate() {
            if p.continuation.is_some() {
                let d = shapez_model::TILE_DIRECTIONS[p.id];
                let a = (p.input.0 + d.0, p.input.1 + d.1, p.input.2);
                diagnostic.fixed.remove(&a);
                diagnostic.nets.push(TileNet {
                    a,
                    ad: 0,
                    b: a,
                    bd: 0,
                    input: false,
                    output: false,
                    exchange: None,
                });
                owners.push(Some(i));
            }
        }
        let input_markers = diagnostic.nets.len();
        for (i, p) in ports.iter().enumerate() {
            diagnostic.nets.push(TileNet {
                a: p.input,
                ad: p.id,
                b: p.input,
                bd: p.id,
                input: false,
                output: false,
                exchange: None,
            });
            owners.push(Some(i));
        }
        let mut failures = Vec::new();
        for (id, rs) in reachable.iter().enumerate() {
            if rs.is_empty() {
                let own_input = if id < 48 { owners[id] } else { None };
                if let Some(cell) = own_input {
                    diagnostic.nets[cell].exchange = None;
                    if cell == id {
                        (diagnostic.nets[id].b, diagnostic.nets[id].bd) = (
                            problem.nets[(cell + 1) % 48].b,
                            problem.nets[(cell + 1) % 48].bd,
                        );
                    }
                }
                let blockers = shapez_routing::source_blockers(&diagnostic, id);
                if let Some(cell) = own_input {
                    diagnostic.nets[cell] = problem.nets[cell].clone();
                }
                let hard = blockers.is_none();
                let mut blocked = blockers.unwrap_or_default();
                blocked.push(id);
                let terms:Vec<_>=blocked.iter().filter_map(|&j|owners[j].map(|owner|serde_json::json!({"owner":owner,"kind":if j<48 {"continuation"} else if j<96 {"output"} else if j<input_markers {"split"} else {"input"},"position":diagnostic.nets[j].a}))).collect();
                let mut implicated: Vec<_> =
                    blocked.into_iter().filter_map(|j| owners[j]).collect();
                if let Some(owner) = owners[id] {
                    implicated.push(owner);
                }
                implicated.sort_unstable();
                implicated.dedup();
                failures.push(
                    serde_json::json!({"source":id,"owners":implicated,"terms":terms,"hard":hard}),
                );
            }
        }
        for (i, port) in ports.iter().enumerate() {
            if !reachable[..48].iter().any(|rs| rs.contains(&i)) {
                failures.push(serde_json::json!({"target":i,"owners":[i],"terms":[{"owner":i,"kind":"input","position":port.input}],"hard":false}));
            }
        }
        if failures.is_empty() {
            fn augment(
                i: usize,
                edges: &[Vec<usize>],
                seen: &mut [bool],
                matched: &mut [Option<usize>],
            ) -> bool {
                for &j in &edges[i] {
                    let j = j - 48;
                    if !seen[j] {
                        seen[j] = true;
                        if matched[j].is_none_or(|previous| augment(previous, edges, seen, matched))
                        {
                            matched[j] = Some(i);
                            return true;
                        }
                    }
                }
                false
            }
            let edges = &reachable[48..];
            let mut matched = vec![None; 48];
            if let Some(root) =
                (0..48).find(|&i| !augment(i, edges, &mut [false; 48], &mut matched))
            {
                let mut sources = HashSet::from([root]);
                let mut targets = HashSet::new();
                let mut queue = vec![root];
                while let Some(i) = queue.pop() {
                    for &j in &edges[i] {
                        let j = j - 48;
                        targets.insert(j);
                        if let Some(other) = matched[j]
                            && sources.insert(other)
                        {
                            queue.push(other);
                        }
                    }
                }
                let outside = (0..48).find(|j| !targets.contains(j)).unwrap();
                let mut terms = Vec::new();
                for &i in &sources {
                    let id = 48 + i;
                    for j in 0..48 {
                        diagnostic.nets[48 + j].exchange = if targets.contains(&j) {
                            None
                        } else {
                            Some(ExchangeGroup(4))
                        };
                    }
                    diagnostic.nets[id].exchange = Some(ExchangeGroup(4));
                    (diagnostic.nets[id].b, diagnostic.nets[id].bd) =
                        (problem.nets[48 + outside].b, problem.nets[48 + outside].bd);
                    let mut blocked =
                        shapez_routing::source_blockers(&diagnostic, id).unwrap_or_default();
                    blocked.push(id);
                    terms.extend(blocked.into_iter().filter_map(|j|owners[j].map(|owner|serde_json::json!({"owner":owner,"kind":if j<48 {"continuation"} else if j<96 {"output"} else if j<input_markers {"split"} else {"input"},"position":diagnostic.nets[j].a}))));
                    for j in 0..48 {
                        (diagnostic.nets[48 + j].b, diagnostic.nets[48 + j].bd) =
                            (problem.nets[48 + j].b, problem.nets[48 + j].bd);
                    }
                }
                let mut implicated: Vec<_> = sources.into_iter().collect();
                implicated.sort_unstable();
                failures.push(serde_json::json!({"source":48+root,"owners":implicated,"terms":terms,"hard":false,"matching_deficit":true}));
            }
        }
        std::fs::write(path, serde_json::to_vec(&failures)?)?;
        anyhow::ensure!(
            failures.is_empty(),
            "{} isolated terminals reported",
            failures.len()
        );
    }
    let Some(assignment) = crate::distribution::assign(&reachable[..48], &continuation_sources)?
    else {
        if let Ok(path) = std::env::var("QUARTER_FEEDBACK") {
            let dead: Vec<_> = (0..48).collect();
            let terms: Vec<_> = dead
                .iter()
                .map(|&i| serde_json::json!({"owner":i,"kind":"input","position":ports[i].input}))
                .collect();
            std::fs::write(
                path,
                serde_json::to_vec(&vec![
                    serde_json::json!({"source":0,"owners":dead,"terms":terms,"hard":false,"supply_deficit":true}),
                ])?,
            )?;
        }
        anyhow::bail!("No four-cell distribution cover for this placement");
    };
    eprintln!("SAT matched all 48 cells into twelve four-cell chains");
    for (source, &cell) in assignment.iter().enumerate() {
        let net = &mut problem.nets[source];
        net.b = ports[cell].input;
        net.bd = ports[cell].id;
        net.exchange = None;
    }
    for root in 0..12 {
        let mut source = root;
        for stage in 0..4 {
            problem.nets[source].exchange = Some(ExchangeGroup(stage));
            if let Some(next) = continuation_sources[assignment[source]] {
                source = next;
            }
        }
    }
    let exchanges: Vec<_> = problem.nets.iter().map(|n| n.exchange).collect();
    shapez_routing::assign_reachable_targets(&mut problem)?;
    if std::env::var_os("QUARTER_EXCHANGE").is_some() {
        for (net, exchange) in problem.nets.iter_mut().zip(exchanges) {
            net.exchange = exchange;
        }
    }
    anyhow::ensure!(
        std::env::var_os("QUARTER_ASSESS").is_none(),
        "READY: placement has full supply coverage and reachable output matching"
    );
    for route in shapez_routing::solve_tiles_negotiated(problem, 180)? {
        shapez_blueprint::route_buildings(&route, &mut buildings);
    }
    Ok(buildings)
}
