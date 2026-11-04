use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum Color {
    #[serde(rename = "u")]
    Uncolored,
    #[serde(rename = "r")]
    Red,
    #[serde(rename = "g")]
    Green,
    #[serde(rename = "b")]
    Blue,
    #[serde(rename = "c")]
    Cyan,
    #[serde(rename = "m")]
    Magenta,
    #[serde(rename = "y")]
    Yellow,
    #[serde(rename = "w")]
    White,
}
impl TryFrom<char> for Color {
    type Error = anyhow::Error;
    fn try_from(c: char) -> Result<Self> {
        Ok(match c {
            'u' => Self::Uncolored,
            'r' => Self::Red,
            'g' => Self::Green,
            'b' => Self::Blue,
            'c' => Self::Cyan,
            'm' => Self::Magenta,
            'y' => Self::Yellow,
            'w' => Self::White,
            _ => bail!("Unknown color {c}"),
        })
    }
}
impl Color {
    pub fn code(self) -> char {
        ['u', 'r', 'g', 'b', 'c', 'm', 'y', 'w'][self as usize]
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Material {
    Circle,
    Rectangle,
    Star,
    Windmill,
    Crystal,
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Part {
    Empty,
    Pin,
    Solid(Material, Color),
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Layer(pub [Part; 4]);

/// Validated shape code. Serialization preserves the game's wire representation.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Shape(String);
impl Shape {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn layers(&self) -> Vec<Layer> {
        self.0
            .split(':')
            .map(|l| {
                Layer(std::array::from_fn(|i| {
                    parse_part(&l[i * 2..i * 2 + 2]).expect("validated shape")
                }))
            })
            .collect()
    }
}
fn parse_part(s: &str) -> Result<Part> {
    if s == "--" {
        return Ok(Part::Empty);
    }
    if s == "P-" {
        return Ok(Part::Pin);
    }
    let b = s.as_bytes();
    let material = match b[0] {
        b'C' => Material::Circle,
        b'R' => Material::Rectangle,
        b'S' => Material::Star,
        b'W' => Material::Windmill,
        b'c' => Material::Crystal,
        _ => bail!("Unknown part {s}"),
    };
    let color = Color::try_from(b[1] as char)?;
    ensure!(
        material != Material::Crystal || color != Color::Uncolored,
        "Crystal needs a color"
    );
    Ok(Part::Solid(material, color))
}
impl TryFrom<String> for Shape {
    type Error = anyhow::Error;
    fn try_from(s: String) -> Result<Self> {
        ensure!(!s.is_empty(), "Shape must contain a layer");
        for layer in s.split(':') {
            ensure!(
                layer.is_ascii() && layer.len() == 8,
                "Each layer needs four two-character parts"
            );
            for i in 0..4 {
                parse_part(&layer[i * 2..i * 2 + 2])?;
            }
        }
        Ok(Self(s))
    }
}
impl FromStr for Shape {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        Self::try_from(s.to_owned())
    }
}
impl From<Shape> for String {
    fn from(s: Shape) -> String {
        s.0
    }
}
impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
