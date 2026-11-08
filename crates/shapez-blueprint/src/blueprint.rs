use crate::model::{BlueprintContent, BlueprintIcon, IconComponent, Island};
use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
};

/// Envelope version, independent of the game/save version inside the payload.
pub const BLUEPRINT_FORMAT_VERSION: u32 = 5;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Blueprint {
    pub game_version: u32,
    pub content: BlueprintContent,
    pub icon: Option<BlueprintIcon>,
    pub source_format_version: Option<u32>,
}

#[derive(Serialize)]
struct Payload<'a> {
    #[serde(rename = "V")]
    game_version: u32,
    #[serde(rename = "BP")]
    content: &'a BlueprintContent,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    #[serde(rename = "$type", default)]
    _type_name: Option<String>,
    #[serde(rename = "V")]
    game_version: u32,
    #[serde(rename = "BP")]
    content: BlueprintContent,
}
impl TryFrom<Document> for Blueprint {
    type Error = anyhow::Error;
    fn try_from(mut value: Document) -> Result<Self> {
        ensure!(value.game_version > 0, "Missing game version");
        let icon = match &mut value.content {
            BlueprintContent::Islands(b) => b.embedded_icon.take(),
            BlueprintContent::Buildings(b) => b.embedded_icon.take(),
        };
        Ok(Self {
            game_version: value.game_version,
            content: value.content,
            icon,
            source_format_version: None,
        })
    }
}

/// Newtonsoft custom-data envelope. Only known, modeled metadata is accepted.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MetadataEntry {
    #[serde(rename = "Type")]
    kind: String,
    #[serde(rename = "Content")]
    content: String,
}
const ICON_TYPE: &str =
    "BlueprintIcon, SPZGameAssembly, Version=0.0.0.0, Culture=neutral, PublicKeyToken=null";

pub fn decode_blueprint(text: &str) -> Result<Blueprint> {
    let text = text.trim();
    let body = text
        .strip_prefix("SHAPEZ2-")
        .context("Missing SHAPEZ2 prefix")?;
    let (version, body) = body
        .split_once('-')
        .context("Missing format version delimiter")?;
    let version: u32 = version.parse().context("Invalid format version")?;
    ensure!(
        (1..=BLUEPRINT_FORMAT_VERSION).contains(&version),
        "Unsupported blueprint format {version}"
    );
    let body = body
        .strip_suffix('$')
        .context("Missing blueprint terminator $")?;
    let (payload, metadata) = if let Some((body, length)) = body.rsplit_once('_') {
        let length: usize = length.parse().context("Invalid metadata length")?;
        // .NET String.Length counts UTF-16 code units, not UTF-8 bytes.
        let mut units = 0;
        let mut start = body.len();
        for (offset, ch) in body.char_indices().rev() {
            if units == length {
                break;
            }
            units += ch.len_utf16();
            start = offset;
        }
        ensure!(
            units == length,
            "Metadata length exceeds content or splits a UTF-16 character"
        );
        (&body[..start], Some(&body[start..]))
    } else {
        ensure!(
            version < 5,
            "Format 5 requires metadata and its length suffix"
        );
        (body, None)
    };
    let mut payload = payload.to_owned();
    while payload.len() % 4 != 0 {
        payload.push('=');
    }
    let bytes = STANDARD
        .decode(payload)
        .context("Invalid blueprint base64")?;
    let mut decoded = Vec::new();
    GzDecoder::new(bytes.as_slice())
        .read_to_end(&mut decoded)
        .context("Invalid blueprint gzip")?;
    let document: Document = serde_json::from_slice(&decoded).context("Invalid blueprint data")?;
    let mut blueprint = Blueprint::try_from(document)?;
    blueprint.source_format_version = Some(version);
    if let Some(metadata) = metadata {
        let entries: Vec<MetadataEntry> =
            serde_json::from_str(metadata).context("Invalid blueprint metadata")?;
        for entry in entries {
            ensure!(
                entry.kind.split(',').next() == Some("BlueprintIcon"),
                "Unsupported blueprint metadata type {}",
                entry.kind
            );
            ensure!(
                blueprint.icon.is_none(),
                "Duplicate blueprint icon metadata"
            );
            blueprint.icon =
                Some(serde_json::from_str(&entry.content).context("Invalid blueprint icon")?);
        }
    }
    Ok(blueprint)
}

pub fn encode_blueprint(blueprint: &Blueprint) -> Result<String> {
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&serde_json::to_vec(&Payload {
        game_version: blueprint.game_version,
        content: &blueprint.content,
    })?)?;
    let icon = blueprint
        .icon
        .clone()
        .unwrap_or_else(|| blueprint.default_icon());
    let metadata = serde_json::to_string(&[MetadataEntry {
        kind: ICON_TYPE.into(),
        content: serde_json::to_string(&icon)?,
    }])?;
    Ok(format!(
        "SHAPEZ2-{BLUEPRINT_FORMAT_VERSION}-{}{}_{length}$",
        STANDARD.encode(gz.finish()?),
        metadata,
        length = metadata.encode_utf16().count()
    ))
}

impl Blueprint {
    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text =
            std::fs::read_to_string(path).with_context(|| format!("Reading {}", path.display()))?;
        decode_blueprint(&text).with_context(|| format!("Decoding {}", path.display()))
    }
    pub fn islands(&self) -> &[Island] {
        match &self.content {
            BlueprintContent::Islands(b) => &b.entries,
            _ => &[],
        }
    }
    pub fn replace_islands(&mut self, entries: Vec<Island>) -> Result<()> {
        match &mut self.content {
            BlueprintContent::Islands(b) => {
                b.entries = entries;
                Ok(())
            }
            _ => bail!("Island blueprint required"),
        }
    }
    pub fn default_icon(&self) -> BlueprintIcon {
        let (name, shape) = match self.content {
            BlueprintContent::Islands(_) => ("Platforms", "RuRuRuRu"),
            BlueprintContent::Buildings(_) => ("Buildings", "CuCuCuCu"),
        };
        BlueprintIcon {
            components: [
                Some(IconComponent::Icon(name.into())),
                None,
                None,
                Some(IconComponent::Shape(shape.into())),
            ],
        }
    }
    pub fn summary(&self) -> BlueprintSummary<'_> {
        let mut summary = BlueprintSummary {
            version: self.game_version,
            islands: self.islands().len(),
            buildings: 0,
            types: BTreeMap::new(),
            labels: Vec::new(),
            train_interfaces: Vec::new(),
        };
        for (index, island) in self.islands().iter().enumerate() {
            *summary.types.entry(island.definition.as_str()).or_insert(0) += 1;
            summary.add_buildings(index, [island.x, island.y], island.buildings());
            if island.definition.contains("Train") {
                summary
                    .train_interfaces
                    .push(TrainInterfaceSummary { index, island });
            }
        }
        if let BlueprintContent::Buildings(b) = &self.content {
            summary.add_buildings(0, [0, 0], &b.entries);
        }
        summary
    }
}

#[derive(Serialize)]
pub struct BlueprintSummary<'a> {
    pub version: u32,
    pub islands: usize,
    pub buildings: usize,
    pub types: BTreeMap<&'a str, usize>,
    pub labels: Vec<LabelSummary<'a>>,
    pub train_interfaces: Vec<TrainInterfaceSummary<'a>>,
}
impl<'a> BlueprintSummary<'a> {
    fn add_buildings(&mut self, index: usize, chunk: [i32; 2], entries: &'a [crate::Building]) {
        self.buildings += entries.len();
        for building in entries {
            if building.definition.contains("Label") {
                self.labels.push(LabelSummary {
                    island: index,
                    chunk,
                    building,
                    decoded: building
                        .configuration
                        .as_ref()
                        .map(|c| String::from_utf8_lossy(&c.0).into_owned())
                        .unwrap_or_default(),
                });
            }
        }
    }
}
#[derive(Serialize)]
pub struct LabelSummary<'a> {
    pub island: usize,
    pub chunk: [i32; 2],
    pub building: &'a crate::Building,
    pub decoded: String,
}
#[derive(Serialize)]
pub struct TrainInterfaceSummary<'a> {
    pub index: usize,
    pub island: &'a Island,
}
