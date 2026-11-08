//! Adapt the delivered blueprint and saved material-flow witness to the shared viewer.
use crate::{FactorySolution, FlowState, Layout};
use anyhow::{Context, Result, ensure};
use shapez_blueprint::{Blueprint, Building};
use shapez_viz::{Category, Connection, Point, Port, Region, Scene, Tile};
use std::collections::BTreeMap;

/// Clockwise horizontal moves followed by upward and downward layer moves.
const DIRECTIONS: [Point; 6] = [
    [1, 0, 0],
    [0, 1, 0],
    [-1, 0, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

/// Decode the task's integer shape/rate token; internal cutter wires carry one half-item unit.
fn material(value: i64) -> Option<(i64, f64)> {
    let normal = if value >= 1000 {
        (value - 1000) % 128
    } else {
        value
    };
    if (1..=64).contains(&normal) {
        Some(((normal - 1) / 4, ((normal - 1) % 4 + 1) as f64 * 0.5))
    } else if (80..104).contains(&value) {
        Some(([12, 8, 4][((value - 80) % 3) as usize], 0.5))
    } else {
        None
    }
}

/// A material mask uses the same clockwise quadrant order as game shape codes.
fn shape(mask: i64) -> String {
    (0..4)
        .map(|i| if mask & (1 << i) != 0 { "Cu" } else { "--" })
        .collect()
}

/// Label a directed connection using the circle-shaped visualization input.
fn connection(from: Point, to: Point, mask: i64, rate: f64, jump: bool) -> Connection {
    Connection {
        from,
        to,
        material: shape(mask).into(),
        rate,
        highlighted: mask == 15,
        jump,
    }
}

/// Reflect and replicate the exact same bank coordinates used by assembly.
fn states(solution: &FactorySolution) -> Result<BTreeMap<Point, FlowState>> {
    let layout = solution.layout;
    let mut result = BTreeMap::new();
    let repeats = |region: &crate::RegionSolution| {
        if region.buildings.iter().map(|b| b.layer).max() == Some(0) {
            3
        } else {
            1
        }
    };
    let banks = [
        (&solution.left.flow, 0, repeats(&solution.left)),
        (&solution.right.flow, 17, repeats(&solution.right)),
    ];
    for (bank, x_offset, repeat) in banks {
        for z in 0..repeat {
            for state in bank {
                for reflected in [false, true] {
                    if reflected && layout == Layout::Original {
                        continue;
                    }
                    let mut s = state.clone();
                    s.position[0] += x_offset;
                    s.position[2] += z;
                    if reflected {
                        s.position[1] = 13 - s.position[1];
                        s.ports.swap(1, 3);
                        s.ports.swap(7, 9);
                    }
                    ensure!(
                        result.insert(s.position, s).is_none(),
                        "Overlapping bank witnesses"
                    );
                }
            }
        }
    }
    Ok(result)
}

/// Native footprints for the building families present in the delivered factory.
fn footprint(b: &Building) -> Vec<Point> {
    let p = [b.x - 3, b.y - 3, i32::from(b.layer)];
    let mut positions = vec![p];
    if b.definition.starts_with("CutterDefault") {
        let side = if b.definition.ends_with("Mirrored") {
            1
        } else {
            3
        };
        let d = DIRECTIONS[(b.rotation.index() + side) % 4];
        positions.push([p[0] + d[0], p[1] + d[1], p[2]]);
    } else if b.definition.starts_with("Lift") {
        let levels = if b.definition.starts_with("Lift2") {
            2
        } else {
            1
        };
        let dz = if b.definition.contains("Up") { 1 } else { -1 };
        for step in 1..=levels {
            positions.push([p[0], p[1], p[2] + step * dz]);
        }
    }
    positions
}

/// Build an inspectable view from actual buildings, including every occupied auxiliary tile.
pub(crate) fn scene(blueprint: &Blueprint, solution: &FactorySolution) -> Result<Scene> {
    let layout = solution.layout;
    let buildings = blueprint
        .islands()
        .first()
        .context("Missing factory island")?
        .buildings();
    let states = states(solution)?;
    let mut occupied = BTreeMap::new();
    let mut tiles = Vec::new();
    let mut origins = BTreeMap::new();
    for (owner, b) in buildings.iter().enumerate() {
        let (category, label) = if b.definition.starts_with("Cutter") {
            (0, "完整切割器")
        } else if b.definition.starts_with("Rotator") {
            (
                1,
                if b.definition.contains("CCW") {
                    "逆时针旋转 90°"
                } else if b.definition.contains("Half") {
                    "旋转 180°"
                } else {
                    "顺时针旋转 90°"
                },
            )
        } else if b.definition.starts_with("Merger") {
            (2, "合流器")
        } else if b.definition.starts_with("Splitter") {
            (3, "分流器")
        } else if b.definition.starts_with("Lift") {
            (4, "升降机")
        } else if b.definition.starts_with("BeltPort") {
            (
                5,
                if b.definition.contains("Sender") {
                    "发射器 / 输出口"
                } else {
                    "接收器 / 输入口"
                },
            )
        } else {
            (6, "传送带")
        };
        for (index, p) in footprint(b).into_iter().enumerate() {
            ensure!(
                occupied.insert(p, owner).is_none(),
                "Duplicate building footprint {p:?}"
            );
            if index == 0 {
                origins.insert(p, b);
            }
            tiles.push(Tile {
                position: p,
                owner,
                category,
                label: label.into(),
                direction: b.rotation.index(),
                auxiliary: index != 0,
                description: b.definition.clone().into(),
            });
        }
    }
    let mut connections = Vec::new();
    for (&p, state) in &states {
        if state.ports.iter().any(|&t| t != 0) {
            ensure!(
                occupied.contains_key(&p),
                "Flow witness has no building at {p:?}"
            );
        }
        for (direction, &token) in state.ports[6..].iter().enumerate() {
            if let Some((mask, rate)) = material(token) {
                let d = DIRECTIONS[direction];
                let to = [p[0] + d[0], p[1] + d[1], p[2] + d[2]];
                ensure!(
                    occupied.contains_key(&to),
                    "Flow witness has no destination {to:?}"
                );
                connections.push(connection(p, to, mask, rate, false));
            }
        }
        if let Some(b) = origins
            .get(&p)
            .filter(|b| b.definition == "BeltPortSenderInternalVariant")
        {
            let d = DIRECTIONS[b.rotation.index()];
            let targets: Vec<_> = (2..=5)
                .map(|n| [p[0] + n * d[0], p[1] + n * d[1], p[2]])
                .filter(|q| {
                    origins.get(q).is_some_and(|other| {
                        other.definition == "BeltPortReceiverInternalVariant"
                            && other.rotation == b.rotation
                    })
                })
                .collect();
            ensure!(targets.len() == 1, "Ambiguous jump from {p:?}: {targets:?}");
            let (mask, rate) = state.ports[..6]
                .iter()
                .find_map(|&t| material(t))
                .context("Sender has no material")?;
            connections.push(connection(p, targets[0], mask, rate, true));
        }
    }
    for z in 0..3 {
        for lane in 5..9 {
            let (from, to) = if layout == Layout::Original {
                ([lane, -1, z], [lane, 0, z])
            } else {
                ([-1, lane, z], [0, lane, z])
            };
            connections.push(connection(from, to, 15, 2., false));
        }
    }
    let regions = if layout == Layout::Original {
        vec![
            (
                [0, 0, 17, 14],
                "左区 · 北侧供料",
                "17×14，单层解复制三层：北侧 12 带输入，西侧及南侧共 24 带输出；6 带完整形状向右区供料。核心位置、镜像与线路联合求解。",
            ),
            (
                [17, 0, 34, 14],
                "右区 · 东侧与南侧输出",
                "17×14，单层解复制三层：接收左区 6 带原料，东侧及南侧共 24 带输出。两区在 y=5、8 连接。",
            ),
        ]
    } else {
        vec![
            (
                [0, 0, 17, 7],
                "左上 · A",
                "17×7×3：6 带进入，12 带四分之一片向上输出，3 带完整形状向右穿越。",
            ),
            (
                [17, 0, 34, 7],
                "右上 · B",
                "17×7，单层解复制三层：接收 A 的 3 带，12 带四分之一片向上输出。",
            ),
            (
                [0, 7, 17, 14],
                "左下 · A 的反射",
                "6 带进入，12 带四分之一片向下输出，3 带完整形状向右穿越。",
            ),
            (
                [17, 7, 34, 14],
                "右下 · B 的反射",
                "接收左下的 3 带，12 带四分之一片向下输出。",
            ),
        ]
    };
    let ports = if layout == Layout::Original {
        [
            ([5, -1], [8, -1], "输入 12 带", true),
            ([-1, 5], [-1, 8], "输出 12 带", false),
            ([34, 5], [34, 8], "输出 12 带", false),
            ([5, 14], [8, 14], "输出 12 带", false),
            ([25, 14], [28, 14], "输出 12 带", false),
        ]
    } else {
        [
            ([-1, 5], [-1, 8], "输入 12 带", true),
            ([5, -1], [8, -1], "输出 12 带", false),
            ([25, -1], [28, -1], "输出 12 带", false),
            ([5, 14], [8, 14], "输出 12 带", false),
            ([25, 14], [28, 14], "输出 12 带", false),
        ]
    };
    Ok(Scene {
        description: "1×2 实际布局 · 有效地面 34×14 · 3 层 · 12 条满带输入 → 48 条满带输出。建筑取自重建蓝图，连接及流量取自对应的求解见证；没有示意性改动线路。".into(),
        width: 34, height: 14, layers: 3,
        categories: [("切割","C"),("旋转","R"),("合流","M"),("分流","S"),("升降","L"),("跳跃/端口","J"),("传送带","·")]
            .map(|(name,glyph)| Category { name:name.into(), glyph:glyph.into() }).into(),
        regions: regions.into_iter().map(|(bounds,name,description)| Region { bounds,name:name.into(),description:description.into() }).collect(),
        ports: ports
            .map(|(from,to,name,input)| Port {from,to,name:name.into(),input}).into(),
        tiles, connections,
    })
}
