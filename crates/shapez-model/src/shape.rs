//! Typed factory domain models.
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// Game paint color, including the uncolored state.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum Color {
    /// Unpainted material (u).
    #[serde(rename = "u")]
    Uncolored,
    /// Red paint (r).
    #[serde(rename = "r")]
    Red,
    /// Green paint (g).
    #[serde(rename = "g")]
    Green,
    /// Blue paint (b).
    #[serde(rename = "b")]
    Blue,
    /// Cyan paint (c).
    #[serde(rename = "c")]
    Cyan,
    /// Magenta paint (m).
    #[serde(rename = "m")]
    Magenta,
    /// Yellow paint (y).
    #[serde(rename = "y")]
    Yellow,
    /// White paint (w).
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
    /// Return the single-character game wire code.
    pub fn code(self) -> char {
        ['u', 'r', 'g', 'b', 'c', 'm', 'y', 'w'][self as usize]
    }
}
/// Solid quadrant geometry recognized by the game.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Material {
    /// Circular quadrant (C).
    Circle,
    /// Rectangular quadrant (R).
    Rectangle,
    /// Star quadrant (S).
    Star,
    /// Windmill quadrant (W).
    Windmill,
    /// Colored crystal quadrant (c).
    Crystal,
}
/// One quadrant: empty, a pin, or a material with a color.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Part {
    /// Unoccupied quadrant (--).
    Empty,
    /// Pin quadrant (P-).
    Pin,
    /// Material and color of an occupied quadrant.
    Solid(Material, Color),
}
/// Four quadrants in game wire order.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Layer(pub [Part; 4]);

/// Validated shape code. Serialization preserves the game's wire representation.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Shape(Box<str>);
impl Shape {
    /// Borrow the validated game wire code without allocating.
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// Decode layers lazily in wire order without allocating a collection.
    pub fn layers(&self) -> impl DoubleEndedIterator<Item = Layer> + '_ {
        self.0.split(':').map(|l| {
            Layer(std::array::from_fn(|i| {
                parse_part(&l[i * 2..i * 2 + 2]).expect("validated shape")
            }))
        })
    }
}
/// Decode one ASCII two-byte quadrant, rejecting unknown materials and invalid crystal colors.
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
        Ok(Self(s.into_boxed_str()))
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
        s.0.into_string()
    }
}
impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
