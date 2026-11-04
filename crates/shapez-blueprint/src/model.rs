use anyhow::{Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// Quarter turns in the game's clockwise coordinate system.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct Rotation(u8);

impl Rotation {
    pub fn new(turns: usize) -> Self {
        Self((turns % 4) as u8)
    }
    pub fn index(self) -> usize {
        self.0 as usize
    }
}
impl TryFrom<u8> for Rotation {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self> {
        ensure!(value < 4, "Rotation must be in 0..4, got {value}");
        Ok(Self(value))
    }
}
impl From<Rotation> for u8 {
    fn from(value: Rotation) -> Self {
        value.0
    }
}

/// Game-owned binary configuration, decoded from its base64 wire representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Configuration(pub Vec<u8>);
impl Serialize for Configuration {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(&self.0))
    }
}
impl<'de> Deserialize<'de> for Configuration {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = wire::Bytes::deserialize(deserializer)?.into_text();
        STANDARD.decode(text).map(Self).map_err(de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "wire::Building")]
pub struct Building {
    #[serde(rename = "X")]
    pub x: i32,
    #[serde(rename = "Y")]
    pub y: i32,
    #[serde(rename = "L")]
    pub layer: i16,
    #[serde(rename = "R")]
    pub rotation: Rotation,
    #[serde(rename = "T")]
    pub definition: String,
    #[serde(rename = "C")]
    pub configuration: Option<Configuration>,
}
impl Building {
    pub fn new(
        definition: impl Into<String>,
        x: i32,
        y: i32,
        layer: i16,
        rotation: Rotation,
    ) -> Self {
        Self {
            x,
            y,
            layer,
            rotation,
            definition: definition.into(),
            configuration: None,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "$type",
    rename = "Building",
    try_from = "wire::BuildingBlueprint"
)]
pub struct BuildingBlueprint {
    #[serde(rename = "Entries")]
    pub entries: Vec<Building>,
    #[serde(skip)]
    pub(crate) legacy_icon: Option<BlueprintIcon>,
}
impl BuildingBlueprint {
    pub fn new(entries: Vec<Building>) -> Self {
        Self {
            entries,
            legacy_icon: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "wire::Island")]
pub struct Island {
    #[serde(rename = "X")]
    pub x: i32,
    #[serde(rename = "Y")]
    pub y: i32,
    #[serde(rename = "Z")]
    pub layer: i16,
    #[serde(rename = "R")]
    pub rotation: Rotation,
    #[serde(rename = "T")]
    pub definition: String,
    #[serde(rename = "S")]
    pub configuration: Option<Configuration>,
    /// Older island records also carried C. Preserve these bytes explicitly.
    #[serde(rename = "C")]
    pub legacy_configuration: Option<Configuration>,
    #[serde(rename = "B")]
    pub buildings: Option<BuildingBlueprint>,
}
impl Island {
    pub fn new(
        definition: impl Into<String>,
        x: i32,
        y: i32,
        layer: i16,
        rotation: Rotation,
    ) -> Self {
        Self {
            x,
            y,
            layer,
            rotation,
            definition: definition.into(),
            configuration: None,
            legacy_configuration: None,
            buildings: None,
        }
    }
    pub fn buildings(&self) -> &[Building] {
        self.buildings.as_ref().map_or(&[], |b| &b.entries)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "Island", try_from = "wire::IslandBlueprint")]
pub struct IslandBlueprint {
    #[serde(rename = "Entries")]
    pub entries: Vec<Island>,
    #[serde(skip)]
    pub(crate) legacy_icon: Option<BlueprintIcon>,
}
impl IslandBlueprint {
    pub fn new(entries: Vec<Island>) -> Self {
        Self {
            entries,
            legacy_icon: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(from = "wire::Content", untagged)]
pub enum BlueprintContent {
    Islands(IslandBlueprint),
    Buildings(BuildingBlueprint),
}

/// A library icon component is either a named game icon or a shape code.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum IconComponent {
    Icon(String),
    Shape(String),
}
impl TryFrom<String> for IconComponent {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self> {
        if let Some(icon) = value.strip_prefix("icon:").filter(|s| !s.is_empty()) {
            Ok(Self::Icon(icon.into()))
        } else if let Some(shape) = value.strip_prefix("shape:").filter(|s| !s.is_empty()) {
            Ok(Self::Shape(shape.into()))
        } else {
            anyhow::bail!("Unknown blueprint icon component: {value}")
        }
    }
}
impl From<IconComponent> for String {
    fn from(value: IconComponent) -> Self {
        match value {
            IconComponent::Icon(icon) => format!("icon:{icon}"),
            IconComponent::Shape(shape) => format!("shape:{shape}"),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlueprintIcon {
    #[serde(rename = "Data")]
    pub components: [Option<IconComponent>; 4],
}

// Deserialization-only DTOs accept Newtonsoft's typed array/byte wrappers.
// Unknown fields fail rather than silently losing data during a rewrite.
pub(crate) mod wire {
    use super::*;

    #[derive(Deserialize)]
    #[serde(tag = "$type", deny_unknown_fields)]
    pub enum Content {
        Island {
            #[serde(rename = "Entries")]
            entries: Array<super::Island>,
            #[serde(rename = "Icon", default)]
            icon: Option<BlueprintIcon>,
        },
        Building {
            #[serde(rename = "Entries")]
            entries: Array<super::Building>,
            #[serde(rename = "Icon", default)]
            icon: Option<BlueprintIcon>,
        },
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    pub enum Array<T> {
        Plain(Vec<T>),
        Typed(TypedArray<T>),
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct TypedArray<T> {
        #[serde(rename = "$type")]
        _type_name: String,
        #[serde(rename = "$values")]
        values: Vec<T>,
    }
    impl<T> Array<T> {
        pub fn into_vec(self) -> Vec<T> {
            match self {
                Self::Plain(v) => v,
                Self::Typed(v) => v.values,
            }
        }
    }
    #[derive(Deserialize)]
    #[serde(untagged)]
    pub enum Bytes {
        Plain(String),
        Typed(TypedBytes),
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct TypedBytes {
        #[serde(rename = "$type")]
        _type_name: String,
        #[serde(rename = "$value")]
        value: String,
    }
    impl Bytes {
        pub fn into_text(self) -> String {
            match self {
                Self::Plain(v) => v,
                Self::Typed(v) => v.value,
            }
        }
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Building {
        #[serde(rename = "$type", default)]
        _type_name: Option<String>,
        #[serde(rename = "X", default)]
        pub x: i32,
        #[serde(rename = "Y", default)]
        pub y: i32,
        #[serde(rename = "L", default)]
        pub layer: i16,
        #[serde(rename = "R", default)]
        pub rotation: Rotation,
        #[serde(rename = "T")]
        pub definition: String,
        #[serde(rename = "C", default)]
        pub configuration: Option<Configuration>,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct BuildingBlueprint {
        #[serde(rename = "$type")]
        pub kind: String,
        #[serde(rename = "Entries")]
        pub entries: Array<super::Building>,
        #[serde(rename = "Icon", default)]
        pub icon: Option<BlueprintIcon>,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Island {
        #[serde(rename = "$type", default)]
        _type_name: Option<String>,
        #[serde(rename = "X", default)]
        pub x: i32,
        #[serde(rename = "Y", default)]
        pub y: i32,
        #[serde(rename = "Z", default)]
        pub layer: i16,
        #[serde(rename = "R", default)]
        pub rotation: Rotation,
        #[serde(rename = "T")]
        pub definition: String,
        #[serde(rename = "S", default)]
        pub configuration: Option<Configuration>,
        #[serde(rename = "C", default)]
        pub legacy_configuration: Option<Configuration>,
        #[serde(rename = "B", default)]
        pub buildings: Option<super::BuildingBlueprint>,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct IslandBlueprint {
        #[serde(rename = "$type")]
        pub kind: String,
        #[serde(rename = "Entries")]
        pub entries: Array<super::Island>,
        #[serde(rename = "Icon", default)]
        pub icon: Option<BlueprintIcon>,
    }
}
impl From<wire::Content> for BlueprintContent {
    fn from(v: wire::Content) -> Self {
        match v {
            wire::Content::Island { entries, icon } => Self::Islands(IslandBlueprint {
                entries: entries.into_vec(),
                legacy_icon: icon,
            }),
            wire::Content::Building { entries, icon } => Self::Buildings(BuildingBlueprint {
                entries: entries.into_vec(),
                legacy_icon: icon,
            }),
        }
    }
}
impl TryFrom<wire::Building> for Building {
    type Error = anyhow::Error;
    fn try_from(v: wire::Building) -> Result<Self> {
        ensure!(!v.definition.is_empty(), "Building definition is empty");
        Ok(Self {
            x: v.x,
            y: v.y,
            layer: v.layer,
            rotation: v.rotation,
            definition: v.definition,
            configuration: v.configuration,
        })
    }
}
impl TryFrom<wire::Island> for Island {
    type Error = anyhow::Error;
    fn try_from(v: wire::Island) -> Result<Self> {
        ensure!(!v.definition.is_empty(), "Island definition is empty");
        Ok(Self {
            x: v.x,
            y: v.y,
            layer: v.layer,
            rotation: v.rotation,
            definition: v.definition,
            configuration: v.configuration,
            legacy_configuration: v.legacy_configuration,
            buildings: v.buildings,
        })
    }
}
impl TryFrom<wire::BuildingBlueprint> for BuildingBlueprint {
    type Error = anyhow::Error;
    fn try_from(v: wire::BuildingBlueprint) -> Result<Self> {
        ensure!(
            v.kind == "Building",
            "Expected Building blueprint, got {}",
            v.kind
        );
        Ok(Self {
            entries: v.entries.into_vec(),
            legacy_icon: v.icon,
        })
    }
}
impl TryFrom<wire::IslandBlueprint> for IslandBlueprint {
    type Error = anyhow::Error;
    fn try_from(v: wire::IslandBlueprint) -> Result<Self> {
        ensure!(
            v.kind == "Island",
            "Expected Island blueprint, got {}",
            v.kind
        );
        Ok(Self {
            entries: v.entries.into_vec(),
            legacy_icon: v.icon,
        })
    }
}
