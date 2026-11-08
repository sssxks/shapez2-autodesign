//! Physical search geometry and legacy numeric wire representations.
use serde::{Deserialize, Serialize};
use shapez_model::{Direction, TilePoint, TileRouteStep};
use std::collections::BTreeSet;
/// Native tile coordinates (x, y, building layer).
pub type Point = TilePoint;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// Search arrival state: x, y, layer, heading, and catcher lock. The heading retains its legacy numeric wire representation.
pub struct State(
    /// World x coordinate.
    pub i32,
    /// World y coordinate.
    pub i32,
    /// Native building layer.
    pub i16,
    /// Arrival heading, encoded as the legacy integer.
    #[serde(with = "shapez_model::numeric_direction")]
    pub Direction,
    /// Whether arrival used a jump catcher, requiring straight departure.
    pub bool,
);
impl State {
    /// Construct an arrival state from a tile and a constrained heading.
    pub fn new(p: Point, d: Direction, catch: bool) -> Self {
        Self(p.0, p.1, p.2, d, catch)
    }
    /// Return the occupied tile without arrival metadata.
    pub fn point(self) -> Point {
        (self.0, self.1, self.2)
    }
    /// Convert to the shared native routing movement state.
    pub fn step(self) -> TileRouteStep {
        TileRouteStep {
            p: self.point(),
            d: self.3,
            catch: self.4,
        }
    }
}
#[derive(Clone, Copy, Debug)]
/// Directed connection tile at a device or factory boundary.
pub struct Port {
    /// Connection tile.
    pub p: Point,
    /// Direction of travel through the connection.
    pub d: Direction,
}
impl Port {
    /// Initial arrival state without a jump catcher lock.
    pub fn state(self) -> State {
        State::new(self.p, self.d, false)
    }
}
/// Move along a cardinal heading and select the destination layer.
pub fn shift(p: Point, d: Direction, length: i32, z: i16) -> Point {
    (p.0 + d.delta().0 * length, p.1 + d.delta().1 * length, z)
}
/// Rotate a local coordinate clockwise about the origin.
pub fn rotate(p: (i32, i32), r: Direction) -> (i32, i32) {
    let (x, y) = p;
    [(x, y), (-y, x), (-x, -y), (y, -x)][r.index()]
}
/// Usable tiles of the supported three-layer 1x2 factory.
pub fn floor() -> BTreeSet<Point> {
    let g = shapez_model::FoundationGeometry::strip(2, 3);
    (g.min.0..=g.max.0)
        .flat_map(|x| (g.min.1..=g.max.1).flat_map(move |y| (0..g.layers).map(move |z| (x, y, z))))
        .collect()
}
/// Search and native acceptance derive physical lanes from the same socket declarations.
pub fn ports(west: bool) -> (Vec<Port>, Vec<Port>) {
    let geometry = shapez_model::FoundationGeometry::strip(2, 3);
    let sockets = if west {
        crate::factory_sockets()
    } else {
        crate::original_sockets()
    };
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let geometry = &geometry;
    for socket in sockets {
        let ports = if socket.output {
            &mut outputs
        } else {
            &mut inputs
        };
        ports.extend((0..3).flat_map(|z| {
            (0..4).map(move |lane| Port {
                p: geometry.socket_tile(socket, lane, z),
                d: socket.direction(),
            })
        }));
    }
    (inputs, outputs)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// Serialized placement and terminal geometry of one quarter-cut cell; headings use checked native numeric encoding.
pub struct PackedCell {
    #[serde(default)]
    /// Whether the local device layout is reflected.
    pub mirror: bool,
    /// World x coordinate of the local origin.
    pub x: i32,
    /// World y coordinate of the local origin.
    pub y: i32,
    #[serde(with = "shapez_model::numeric_direction")]
    /// Clockwise orientation of the local coordinate frame.
    pub r: Direction,
    /// Selected local output variant (0 through 3).
    pub out: u8,
    /// Input splitter variant (0 through 3), or negative for no splitter.
    pub split: i32,
    /// World coordinate of the input connection.
    pub a: (i32, i32),
    #[serde(with = "shapez_model::numeric_direction")]
    /// Input travel heading.
    pub ad: Direction,
    /// Optional world coordinate of the splitter continuation.
    pub b: Option<(i32, i32)>,
    #[serde(with = "shapez_model::numeric_direction")]
    /// Continuation travel heading.
    pub bd: Direction,
    /// World coordinate of the selected output.
    pub op: (i32, i32),
    #[serde(with = "shapez_model::numeric_direction")]
    /// Output travel heading.
    pub od: Direction,
}
impl PackedCell {
    /// Transform a local coordinate to a world tile on the requested layer.
    pub fn position(&self, p: (i32, i32), z: i16) -> Point {
        let (x, y) = rotate(p, self.r);
        (self.x + x, self.y + y, z)
    }
    /// Occupied device body tiles, excluding external connection tiles.
    pub fn body(&self, z: i16) -> BTreeSet<Point> {
        [(0, 1), (0, 2), (1, 1), (1, 2)]
            .into_iter()
            .chain((2..5).flat_map(|x| (0..4).map(move |y| (x, y))))
            .map(|p| self.position(p, z))
            .collect()
    }
    /// Input connection on the requested layer.
    pub fn input(&self, z: i16) -> Port {
        Port {
            p: (self.a.0, self.a.1, z),
            d: self.ad,
        }
    }
    /// Selected output connection on the requested layer.
    pub fn output(&self, z: i16) -> Port {
        Port {
            p: (self.op.0, self.op.1, z),
            d: self.od,
        }
    }
    /// Optional splitter continuation on the requested layer.
    pub fn continuation(&self, z: i16) -> Option<Port> {
        self.b.map(|p| Port {
            p: (p.0, p.1, z),
            d: self.bd,
        })
    }
    /// Recompute terminal geometry for a reflection, output variant, and input splitter variant.
    pub fn configure(&self, z: i16, mirror: bool, out: u8, split: i32) -> (Self, Point) {
        let mut local_output = [(5, 3), (4, 4), (5, 1), (5, 2)][usize::from(out)];
        if mirror {
            local_output.1 = 3 - local_output.1;
        }
        let op = self.position(local_output, z);
        let ip = self.position((-1, if mirror { 1 } else { 2 }), z);
        let inc = self.r.rotated(if split < 0 {
            0
        } else {
            [0, 0, 3, 1][split as usize]
        });
        let cont = self.r.rotated(if split < 0 {
            0
        } else {
            [3, 1, 3, 1][split as usize]
        });
        let a = if split < 0 {
            ip
        } else {
            shift(ip, inc.opposite(), 1, z)
        };
        let b = (split >= 0).then(|| shift(ip, cont, 1, z));
        (
            Self {
                x: self.x,
                y: self.y,
                r: self.r,
                mirror,
                out,
                split,
                a: (a.0, a.1),
                ad: inc,
                b: b.map(|p| (p.0, p.1)),
                bd: cont,
                op: (op.0, op.1),
                od: self.r.rotated(if out == 1 {
                    if mirror { 3 } else { 1 }
                } else {
                    0
                }),
            },
            ip,
        )
    }
}

#[cfg(test)]
/// Geometry and numeric wire compatibility regressions.
mod tests {
    use super::*;
    /// State headings retain JSON and postcard integer encoding and reject invalid input.
    #[test]
    fn state_wire_compatibility() {
        for direction in Direction::ALL {
            let state = State::new((5, -2, 1), direction, true);
            let legacy = (5_i32, -2_i32, 1_i16, direction.index(), true);
            let wire = postcard::to_allocvec(&legacy).unwrap();
            assert_eq!(postcard::to_allocvec(&state).unwrap(), wire);
            assert_eq!(postcard::from_bytes::<State>(&wire).unwrap(), state);
            let json = serde_json::to_string(&legacy).unwrap();
            assert_eq!(serde_json::from_str::<State>(&json).unwrap(), state);
            assert_eq!(serde_json::to_string(&state).unwrap(), json);
        }
        for invalid in [4_usize, 255, usize::MAX] {
            let legacy = (0_i32, 0_i32, 0_i16, invalid, false);
            assert!(
                postcard::from_bytes::<State>(&postcard::to_allocvec(&legacy).unwrap()).is_err()
            );
            assert!(
                serde_json::from_str::<State>(&serde_json::to_string(&legacy).unwrap()).is_err()
            );
        }
    }

    /// Every serialized cell heading is independently checked at the parsing boundary.
    #[test]
    fn packed_cell_rejects_invalid_headings() {
        for field in ["r", "ad", "bd", "od"] {
            for invalid in ["4", "-1", "1.5", "18446744073709551615"] {
                let json = r#"{"x":0,"y":0,"r":0,"out":0,"split":-1,"a":[0,0],"ad":0,"b":null,"bd":0,"op":[1,0],"od":0}"#;
                assert!(
                    serde_json::from_str::<PackedCell>(
                        &json.replace(&format!("\"{field}\":0"), &format!("\"{field}\":{invalid}"))
                    )
                    .is_err(),
                    "{field}={invalid}"
                );
            }
        }
    }
    /// Preserve the search's layer-major lane order and both physical factory interfaces.
    #[test]
    fn factory_ports_keep_coordinates_directions_and_order() {
        for west in [false, true] {
            let (inputs, outputs) = ports(west);
            assert_eq!((inputs.len(), outputs.len()), (12, 48));
            for (i, input) in inputs.iter().enumerate() {
                let (lane, z) = ((i % 4) as i32, (i / 4) as i16);
                assert_eq!(
                    (input.p, input.d),
                    if west {
                        ((2, 8 + lane, z), Direction::East)
                    } else {
                        ((8 + lane, 2, z), Direction::South)
                    }
                );
            }
            let banks = if west {
                [(8, 2, 3), (28, 2, 3), (8, 17, 1), (28, 17, 1)]
            } else {
                [(2, 8, 2), (37, 8, 0), (8, 17, 1), (28, 17, 1)]
            };
            for (bank, &(x, y, d)) in banks.iter().enumerate() {
                for i in 0..12 {
                    let p = &outputs[bank * 12 + i];
                    let lane = (i % 4) as i32;
                    assert_eq!(
                        (p.p, p.d),
                        (
                            (
                                x + if d % 2 == 1 { lane } else { 0 },
                                y + if d % 2 == 0 { lane } else { 0 },
                                (i / 4) as i16
                            ),
                            Direction::from_index(d).unwrap()
                        )
                    );
                }
            }
        }
    }
}
