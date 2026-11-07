//! Four parallel cutters per lane, with separate full-rate output buses.
use crate::routing::{Net, Point};
use shapez_blueprint::{Building, Rotation};
pub fn layout(
    ne: bool,
    ports: [usize; 3],
    perms: [[i32; 4]; 3],
) -> (Vec<Building>, Vec<Point>, Vec<Net>) {
    let port = |side: usize, g: i32| match side {
        0 => ((8 + g, 2), 1),
        1 => ((28 + g, 2), 1),
        2 => ((8 + g, 17), 3),
        3 => ((28 + g, 17), 3),
        4 => ((2, 8 + g), 0),
        _ => ((37, 8 + g), 2),
    };
    let mut b = Vec::new();
    let mut fixed = Vec::new();
    let mut nets = Vec::new();
    let mut add = |t: &str, x, y, r| {
        b.push(Building::new(t, x, y, 0, Rotation::new(r)));
        fixed.push((x, y));
    };
    for g in 0..4 {
        let x = 5 + 8 * g;
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
            // Jump across the downward output bus to the upward merger bus.

            add("BeltPortReceiverInternalVariant", x + 5, y, 0);
            add("Merger3To1InternalVariant", x + 6, y, 3);
            add("RotatorOneQuadInternalVariant", x + 3, y + 1, 0);
            add("Merger3To1InternalVariant", x + 4, y + 1, 1);
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
                    add("BeltDefaultForwardInternalVariant", x + 6, y + dy, 3);
                }
            }
        }
        let (ip, id) = port(ports[0], perms[0][g as usize]);
        let (op, od) = port(ports[1], perms[1][g as usize]);
        let (qp, qd) = port(ports[2], perms[2][g as usize]);
        nets.push(Net {
            a: ip,
            ad: id,
            b: (x - 1, 6),
            bd: 1,
            input: true,
            output: false,
        });
        nets.push(Net {
            a: (x + 6, 5),
            ad: 3,
            b: op,
            bd: (od + 2) % 4,
            input: false,
            output: true,
        });
        nets.push(Net {
            a: (x + 4, 14),
            ad: 1,
            b: qp,
            bd: (qd + 2) % 4,
            input: false,
            output: true,
        });
    }
    // Reserve non-anchor halves of the cutters and swappers.
    for g in 0..4 {
        for j in 0..4 {
            fixed.push((5 + 8 * g, 6 + 2 * j));
            fixed.push((7 + 8 * g, 6 + 2 * j));
        }
    }
    (b, fixed, nets)
}
