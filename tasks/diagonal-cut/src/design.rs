//! Four parallel cutters per lane, with separate full-rate output buses.
use anyhow::Result;
use shapez_blueprint::{Building, Cell, CellPort, Placement, Rotation};
use shapez_model::{TileNet as Net, TilePoint as Point};
/// A full-rate diagonal lane owns its four stages and both output buses.
fn lane(ne: bool) -> Cell {
    let x = 5;
    let mut b = Vec::new();
    let mut add = |t: &str, x, y, r| b.push(Building::new(t, x, y, 0, Rotation::new(r)));
    for j in 0..4 {
        let y = 6 + 2 * j;
        add("CutterDefaultInternalVariant", x, y + 1, 0);
        add(
            if ne {
                "RotatorOneQuadInternalVariant"
            } else {
                "RotatorOneQuadCCWInternalVariant"
            },
            x + 1,
            y,
            0,
        );
        add(
            if ne {
                "RotatorOneQuadInternalVariant"
            } else {
                "RotatorOneQuadCCWInternalVariant"
            },
            x + 1,
            y + 1,
            0,
        );
        add("HalvesSwapperDefaultInternalVariant", x + 2, y + 1, 0);
        add("BeltPortSenderInternalVariant", x + 3, y, 0);
        // Jump across the downward output bus to the second downward merger bus.

        add("BeltPortReceiverInternalVariant", x + 5, y, 0);
        add(
            if j == 0 {
                "BeltDefaultLeftInternalVariantMirrored"
            } else {
                "Merger2To1LInternalVariantMirrored"
            },
            x + 6,
            y,
            if j == 0 { 0 } else { 1 },
        );
        add("RotatorOneQuadInternalVariant", x + 3, y + 1, 0);
        add(
            if j == 0 {
                "BeltDefaultLeftInternalVariantMirrored"
            } else {
                "Merger2To1LInternalVariantMirrored"
            },
            x + 4,
            y + 1,
            if j == 0 { 0 } else { 1 },
        );
        add(
            if j < 3 {
                "Splitter1To2LInternalVariant"
            } else {
                "BeltDefaultLeftInternalVariant"
            },
            x - 1,
            y + 1,
            1,
        );
        if j < 3 {
            {
                let dy = 2;
                add("BeltDefaultForwardInternalVariant", x - 1, y + dy, 1);
                add("BeltDefaultForwardInternalVariant", x + 4, y + dy, 1);
            }
            {
                let dy = 1;
                add("BeltDefaultForwardInternalVariant", x + 6, y + dy, 1);
            }
        }
    }
    Cell::new(
        b,
        (0..4).flat_map(|j| [(5, 6 + 2 * j, 0), (7, 6 + 2 * j, 0)]),
        [
            (
                "input",
                CellPort {
                    position: (4, 6, 0),
                    direction: 1,
                },
            ),
            (
                "first",
                CellPort {
                    position: (11, 13, 0),
                    direction: 1,
                },
            ),
            (
                "second",
                CellPort {
                    position: (9, 14, 0),
                    direction: 1,
                },
            ),
        ],
    )
}
/// Place four independent lanes and declare their external connections.
pub fn layout(ne: bool, perms: [[i32; 4]; 3]) -> Result<(Vec<Building>, Vec<Point>, Vec<Net>)> {
    let geometry = shapez_model::FoundationGeometry::strip(2, 1);
    let sockets = sockets();
    let port = |index: usize, lane: i32| {
        let socket = sockets[index];
        let p = geometry.socket_tile(socket, lane, 0);
        ((p.0, p.1), socket.direction())
    };
    let mut b = Vec::new();
    let mut fixed = Vec::new();
    let mut nets = Vec::new();
    for g in 0..4 {
        let cell = lane(ne).placed(Placement {
            origin: (8 * g, 0, 0),
            rotation: Rotation::new(0),
        });
        b.extend(cell.buildings.iter().cloned());
        fixed.extend(cell.footprint.iter().copied());
        let (ip, id) = port(0, perms[0][g as usize]);
        let (op, od) = port(1, perms[1][g as usize]);
        let (qp, qd) = port(2, perms[2][g as usize]);
        nets.push(Net {
            a: (ip.0, ip.1, 0),
            ad: id,
            b: cell.port("input")?.position,
            bd: 1,
            exchange: None,
            input: true,
            output: false,
        });
        nets.push(Net {
            a: cell.port("first")?.position,
            ad: 1,
            b: (op.0, op.1, 0),
            bd: od,
            exchange: None,
            input: false,
            output: true,
        });
        nets.push(Net {
            a: cell.port("second")?.position,
            ad: 1,
            b: (qp.0, qp.1, 0),
            bd: qd,
            exchange: None,
            input: false,
            output: true,
        });
    }
    Ok((b, fixed, nets))
}

/// External interface shared by layout and experiment fixture.
pub fn sockets() -> [shapez_model::FoundationSocket; 3] {
    use shapez_model::{Direction, FoundationSocket};
    [
        FoundationSocket {
            side: Direction::North,
            segment: 0,
            output: false,
            role: "input",
        },
        FoundationSocket {
            side: Direction::South,
            segment: 0,
            output: true,
            role: "first",
        },
        FoundationSocket {
            side: Direction::South,
            segment: 1,
            output: true,
            role: "second",
        },
    ]
}
