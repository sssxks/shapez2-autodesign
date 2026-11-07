use serde::{Deserialize, Serialize};
use shapez_model::{TILE_DIRECTIONS as D, TilePoint, TileRouteStep};
use std::collections::BTreeSet;
pub type Point = TilePoint;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct State(pub i32, pub i32, pub i16, pub usize, pub bool);
impl State {
    pub fn new(p: Point, d: usize, catch: bool) -> Self {
        Self(p.0, p.1, p.2, d, catch)
    }
    pub fn point(self) -> Point {
        (self.0, self.1, self.2)
    }
    pub fn step(self) -> TileRouteStep {
        TileRouteStep {
            p: self.point(),
            d: self.3,
            catch: self.4,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Port {
    pub p: Point,
    pub d: usize,
}
impl Port {
    pub fn state(self) -> State {
        State::new(self.p, self.d, false)
    }
}
pub fn shift(p: Point, d: usize, length: i32, z: i16) -> Point {
    (p.0 + D[d].0 * length, p.1 + D[d].1 * length, z)
}
pub fn rotate(p: (i32, i32), r: usize) -> (i32, i32) {
    let (x, y) = p;
    [(x, y), (-y, x), (-x, -y), (y, -x)][r % 4]
}
pub fn floor(chunks: i32) -> BTreeSet<Point> {
    (3..chunks * 20 - 3)
        .flat_map(|x| (3..17).flat_map(move |y| (0..3).map(move |z| (x, y, z))))
        .collect()
}
pub fn ports(chunks: i32, west: bool) -> (Vec<Port>, Vec<Port>) {
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    for z in 0..3 {
        for lane in 0..4 {
            inputs.push(if chunks == 3 {
                Port {
                    p: (28 + lane, 2, z),
                    d: 1,
                }
            } else if west {
                Port {
                    p: (2, 8 + lane, z),
                    d: 0,
                }
            } else {
                Port {
                    p: (8 + lane, 2, z),
                    d: 1,
                }
            });
        }
    }
    if chunks == 3 || west {
        for (y, d) in [(2, 3), (17, 1)] {
            for base in [8, chunks * 20 - 12] {
                for z in 0..3 {
                    for lane in 0..4 {
                        outputs.push(Port {
                            p: (base + lane, y, z),
                            d,
                        });
                    }
                }
            }
        }
    } else {
        for (x, d) in [(2, 2), (37, 0)] {
            for z in 0..3 {
                for lane in 0..4 {
                    outputs.push(Port {
                        p: (x, 8 + lane, z),
                        d,
                    });
                }
            }
        }
        for base in [8, 28] {
            for z in 0..3 {
                for lane in 0..4 {
                    outputs.push(Port {
                        p: (base + lane, 17, z),
                        d: 1,
                    });
                }
            }
        }
    }
    (inputs, outputs)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PackedCell {
    #[serde(default)]
    pub mirror: bool,
    pub x: i32,
    pub y: i32,
    pub r: usize,
    pub out: u8,
    pub split: i32,
    pub a: (i32, i32),
    pub ad: usize,
    pub b: Option<(i32, i32)>,
    pub bd: usize,
    pub op: (i32, i32),
    pub od: usize,
}
impl PackedCell {
    pub fn position(&self, p: (i32, i32), z: i16) -> Point {
        let (x, y) = rotate(p, self.r);
        (self.x + x, self.y + y, z)
    }
    pub fn body(&self, z: i16) -> BTreeSet<Point> {
        [(0, 1), (0, 2), (1, 1), (1, 2)]
            .into_iter()
            .chain((2..5).flat_map(|x| (0..4).map(move |y| (x, y))))
            .map(|p| self.position(p, z))
            .collect()
    }
    pub fn input(&self, z: i16) -> Port {
        Port {
            p: (self.a.0, self.a.1, z),
            d: self.ad,
        }
    }
    pub fn output(&self, z: i16) -> Port {
        Port {
            p: (self.op.0, self.op.1, z),
            d: self.od,
        }
    }
    pub fn continuation(&self, z: i16) -> Option<Port> {
        self.b.map(|p| Port {
            p: (p.0, p.1, z),
            d: self.bd,
        })
    }
    pub fn configure(&self, z: i16, mirror: bool, out: u8, split: i32) -> (Self, Point) {
        let mut local_output = [(5, 3), (4, 4), (5, 1), (5, 2)][usize::from(out)];
        if mirror {
            local_output.1 = 3 - local_output.1;
        }
        let op = self.position(local_output, z);
        let ip = self.position((-1, if mirror { 1 } else { 2 }), z);
        let inc = (self.r
            + if split < 0 {
                0
            } else {
                [0, 0, 3, 1][split as usize]
            })
            % 4;
        let cont = (self.r
            + if split < 0 {
                0
            } else {
                [3, 1, 3, 1][split as usize]
            })
            % 4;
        let a = if split < 0 {
            ip
        } else {
            shift(ip, (inc + 2) % 4, 1, z)
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
                od: (self.r
                    + if out == 1 {
                        if mirror { 3 } else { 1 }
                    } else {
                        0
                    })
                    % 4,
            },
            ip,
        )
    }
}
