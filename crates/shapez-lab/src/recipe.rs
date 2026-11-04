use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::collections::HashMap;

#[derive(Default)]
struct Planner {
    nodes: Vec<Value>,
    cache: HashMap<String, usize>,
}
impl Planner {
    fn node(&mut self, op: &str, inputs: &[usize], expected: String, color: Option<char>) -> usize {
        if let Some(&id) = self.cache.get(&expected) {
            return id;
        }
        let id = self.nodes.len();
        self.nodes
            .push(json!({"id":id,"op":op,"inputs":inputs,"expected":expected,"color":color}));
        self.cache.insert(expected, id);
        id
    }
    fn shape(&self, id: usize) -> String {
        self.nodes[id]["expected"].as_str().unwrap().into()
    }
    fn rotate(&mut self, id: usize, clockwise: bool) -> usize {
        let s = self.shape(id);
        let n = if clockwise { 6 } else { 2 };
        let result = s
            .split(':')
            .map(|s| format!("{}{}", &s[n..], &s[..n]))
            .collect::<Vec<_>>()
            .join(":");
        self.node(
            if clockwise { "rotate-cw" } else { "rotate-ccw" },
            &[id],
            result,
            None,
        )
    }
    fn half(&mut self, id: usize) -> usize {
        let s = self.shape(id);
        let result = s
            .split(':')
            .map(|s| format!("{}----", &s[..4]))
            .collect::<Vec<_>>()
            .join(":");
        self.node("cut", &[id], result, None)
    }
    fn isolate(&mut self, id: usize) -> usize {
        let id = self.half(id);
        let id = self.rotate(id, true);
        let id = self.half(id);
        self.rotate(id, false)
    }
    fn merge(&mut self, a: Option<usize>, b: Option<usize>, rotated: bool) -> Option<usize> {
        let (a, b) = match (a, b) {
            (Some(a), Some(b)) => (a, b),
            (a, b) => return a.or(b),
        };
        let (a, b) = if rotated {
            (self.rotate(a, true), self.rotate(b, true))
        } else {
            (a, b)
        };
        let sa = self.shape(a);
        let sb = self.shape(b);
        let la: Vec<_> = sa.split(':').collect();
        let lb: Vec<_> = sb.split(':').collect();
        let result = (0..la.len().max(lb.len()))
            .map(|l| {
                let sa = la.get(l).copied().unwrap_or("--------");
                let sb = lb.get(l).copied().unwrap_or("--------");
                (0..4)
                    .map(|i| {
                        if &sa[i * 2..i * 2 + 2] == "--" {
                            &sb[i * 2..i * 2 + 2]
                        } else {
                            &sa[i * 2..i * 2 + 2]
                        }
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(":");
        let id = self.node("swap", &[a, b], result, None);
        Some(if rotated { self.rotate(id, false) } else { id })
    }
    fn quarter(&mut self, part: &str, position: usize) -> usize {
        let chars = part.as_bytes();
        let full = if chars[0] == b'c' {
            let mask = self.layer("--CuCuCu").unwrap();
            self.node(
                "crystallize",
                &[mask],
                format!("{part}CuCuCu"),
                Some(chars[1] as char),
            )
        } else {
            let raw = if chars[0] == b'P' {
                "P-".to_owned()
            } else {
                format!("{}u", chars[0] as char)
            };
            let source = self.node("source", &[], raw.repeat(4), None);
            if chars[0] == b'P' || chars[1] == b'u' {
                source
            } else {
                self.node("paint", &[source], part.repeat(4), Some(chars[1] as char))
            }
        };
        let mut id = self.isolate(full);
        for _ in 0..position {
            id = self.rotate(id, true);
        }
        id
    }
    fn layer(&mut self, s: &str) -> Option<usize> {
        let q: Vec<_> = (0..4)
            .map(|i| {
                let p = &s[i * 2..i * 2 + 2];
                if p == "--" {
                    None
                } else {
                    Some(self.quarter(p, i))
                }
            })
            .collect();
        let a = self.merge(q[0], q[1], true);
        let b = self.merge(q[2], q[3], true);
        self.merge(a, b, false)
    }
    fn column(&mut self, parts: &[&str], position: usize) -> Result<Option<usize>> {
        let Some(last) = parts.iter().rposition(|p| *p != "--") else {
            return Ok(None);
        };
        let mut result = None;
        let mut built = Vec::<String>::new();
        for part in &parts[..=last] {
            ensure!(
                *part != "--",
                "Unsupported column gap: candidate strategy needs solid columns"
            );
            let crystal = part.starts_with('c');
            ensure!(
                !crystal || !built.iter().any(|s| s.starts_with("P-")),
                "Pin below a later crystal requires another strategy"
            );
            let row = format!("{}CuCuCu", if crystal { "P-" } else { part });
            let upper = self.layer(&row).unwrap();
            built.push(row);
            let id = match result {
                None => upper,
                Some(lower) => self.node("stack", &[lower, upper], built.join(":"), None),
            };
            result = Some(if crystal {
                *built.last_mut().unwrap() = format!("{part}CuCuCu");
                self.node(
                    "crystallize",
                    &[id],
                    built.join(":"),
                    Some(part.as_bytes()[1] as char),
                )
            } else {
                id
            });
        }
        let mut id = self.isolate(result.unwrap());
        for _ in 0..position {
            id = self.rotate(id, true);
        }
        Ok(Some(id))
    }
}
pub fn plan_recipe(target: &str) -> Result<Value> {
    let layers: Vec<_> = target.split(':').collect();
    ensure!(
        !layers.is_empty() && layers.len() <= 4,
        "Prototype supports 1..4 layers"
    );
    for layer in &layers {
        ensure!(
            layer.is_ascii() && layer.len() == 8,
            "Each layer needs four two-character parts"
        );
        ensure!(
            *layer != "--------",
            "Empty layers require a different construction strategy"
        );
        for p in layer.as_bytes().as_chunks::<2>().0 {
            ensure!(
                p == b"--"
                    || p == b"P-"
                    || ((b"CRSW".contains(&p[0]) && b"urgbcmyw".contains(&p[1]))
                        || (p[0] == b'c' && b"rgbcmyw".contains(&p[1]))),
                "Unsupported part or unavailable input color"
            );
        }
    }
    let mut p = Planner::default();
    let result = if layers.len() == 1 {
        p.layer(target)
    } else {
        let mut columns = Vec::new();
        for i in 0..4 {
            columns.push(
                p.column(
                    &layers
                        .iter()
                        .map(|l| &l[i * 2..i * 2 + 2])
                        .collect::<Vec<_>>(),
                    i,
                )?,
            );
        }
        let a = p.merge(columns[0], columns[1], true);
        let b = p.merge(columns[2], columns[3], true);
        p.merge(a, b, false)
    };
    Ok(
        json!({"target":target,"strategy":"scaffold-columns-then-swap","root":result.unwrap(),"nodes":p.nodes,"status":"candidate-requires-native-verification"}),
    )
}
