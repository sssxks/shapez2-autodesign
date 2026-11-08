use crate::model::{BlueprintContent, BlueprintIcon, IconComponent, Island};
use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Read, path::Path};

/// Envelope version, independent of the game/save version inside the payload.
pub const BLUEPRINT_FORMAT_VERSION: u32 = 5;

/// Decoded blueprint with game version, layout and optional display icon.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Blueprint {
    /// Game/save version inside the payload.
    pub game_version: u32,
    /// Layout or serialized content carried by this envelope.
    pub content: BlueprintContent,
    /// Optional library display icon.
    pub icon: Option<BlueprintIcon>,
    /// Source envelope version, absent for newly constructed layouts.
    pub source_format_version: Option<u32>,
}

/// Borrowed payload serialized inside the compressed blueprint envelope.
#[derive(Serialize)]
struct Payload<'a> {
    #[serde(rename = "V")]
    /// Game/save version inside the payload.
    game_version: u32,
    #[serde(rename = "BP")]
    /// Layout or serialized content carried by this envelope.
    content: &'a BlueprintContent,
}

/// Strict input payload; unknown fields are rejected during deserialization.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    #[serde(rename = "$type", default)]
    /// Newtonsoft type annotation accepted for compatibility.
    _type_name: Option<Box<str>>,
    #[serde(rename = "V")]
    /// Game/save version inside the payload.
    game_version: u32,
    #[serde(rename = "BP")]
    /// Layout or serialized content carried by this envelope.
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
    /// Wire type discriminator.
    kind: Box<str>,
    #[serde(rename = "Content")]
    /// Layout or serialized content carried by this envelope.
    content: Box<str>,
}
/// Assembly-qualified game type name identifying icon metadata.
const ICON_TYPE: &str =
    "BlueprintIcon, SPZGameAssembly, Version=0.0.0.0, Culture=neutral, PublicKeyToken=null";

/// Decode a SHAPEZ2 envelope, checking version, metadata lengths and typed payload structure. Returns an error for malformed or unsupported input.
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

/// Serialize the layout and icon into the current compressed SHAPEZ2 envelope.
pub fn encode_blueprint(blueprint: &Blueprint) -> Result<String> {
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    serde_json::to_writer(
        &mut gz,
        &Payload {
            game_version: blueprint.game_version,
            content: &blueprint.content,
        },
    )?;
    let fallback;
    let icon = match &blueprint.icon {
        Some(icon) => icon,
        None => {
            fallback = blueprint.default_icon();
            &fallback
        }
    };
    let metadata = serde_json::to_string(&[MetadataEntry {
        kind: ICON_TYPE.into(),
        content: serde_json::to_string(&icon)?.into_boxed_str(),
    }])?;
    Ok(format!(
        "SHAPEZ2-{BLUEPRINT_FORMAT_VERSION}-{}{}_{length}$",
        STANDARD.encode(gz.finish()?),
        metadata,
        length = metadata.encode_utf16().count()
    ))
}

impl Blueprint {
    /// Create a blueprint for the supported game version, without source metadata.
    pub fn new(content: BlueprintContent, icon: Option<BlueprintIcon>) -> Self {
        Self {
            game_version: 1138,
            content,
            icon,
            source_format_version: None,
        }
    }

    /// Package one platform; its placement and buildings remain owned by the caller.
    pub fn platform(island: Island, icon: Option<BlueprintIcon>) -> Self {
        Self::new(
            BlueprintContent::Islands(crate::IslandBlueprint::new(vec![island])),
            icon,
        )
    }

    /// Encode directly to an artifact, reporting the destination on I/O failure.
    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        std::fs::write(path, encode_blueprint(self)?)
            .with_context(|| format!("Writing {}", path.display()))
    }

    /// Clone a platform blueprint and append the transport islands for its sockets.
    pub fn with_harness(
        &self,
        geometry: &shapez_model::FoundationGeometry,
        sockets: &[shapez_model::FoundationSocket],
    ) -> Result<Self> {
        let mut blueprint = self.clone();
        let BlueprintContent::Islands(islands) = &mut blueprint.content else {
            bail!("Island blueprint required for a foundation harness");
        };
        islands.entries.extend(sockets.iter().map(|&socket| {
            let (x, y, rotation) = geometry.socket_harness(socket);
            Island::new(
                "SpaceBelt_Forward",
                x,
                y,
                0,
                crate::Rotation::new(rotation.index()),
            )
        }));
        Ok(blueprint)
    }

    /// Read and decode a file, attaching its path to errors.
    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text =
            std::fs::read_to_string(path).with_context(|| format!("Reading {}", path.display()))?;
        decode_blueprint(&text).with_context(|| format!("Decoding {}", path.display()))
    }
    /// Borrow island entries; building-only layouts return an empty slice.
    pub fn islands(&self) -> &[Island] {
        match &self.content {
            BlueprintContent::Islands(b) => &b.entries,
            _ => &[],
        }
    }
    /// Replace island entries; return an error for building-only layouts.
    pub fn replace_islands(&mut self, entries: Vec<Island>) -> Result<()> {
        match &mut self.content {
            BlueprintContent::Islands(b) => {
                b.entries = entries;
                Ok(())
            }
            _ => bail!("Island blueprint required"),
        }
    }
    /// Construct the conventional icon for this layout kind.
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
    /// Collect island types, building counts, labels and train interfaces.
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

/// Borrowed layout inventory with counts and decoded labels.
#[derive(Serialize)]
pub struct BlueprintSummary<'a> {
    /// Game/save version of the summarized blueprint.
    pub version: u32,
    /// Number of island entries.
    pub islands: usize,
    /// Total building count across the layout.
    pub buildings: usize,
    /// Island counts keyed by borrowed definition identifiers.
    pub types: BTreeMap<&'a str, usize>,
    /// Label buildings and decoded configuration text.
    pub labels: Vec<LabelSummary<'a>>,
    /// Islands whose definitions identify train interfaces.
    pub train_interfaces: Vec<TrainInterfaceSummary<'a>>,
}
impl<'a> BlueprintSummary<'a> {
    /// Accumulate building counts and label text, borrowing valid UTF-8 configuration bytes.
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
                        .map(|c| String::from_utf8_lossy(&c.0))
                        .unwrap_or_default(),
                });
            }
        }
    }
}
/// Label configuration associated with its containing island and building.
#[derive(Serialize)]
pub struct LabelSummary<'a> {
    /// Containing island index, or zero for building-only layouts.
    pub island: usize,
    /// Containing island chunk coordinates, or the origin for building-only layouts.
    pub chunk: [i32; 2],
    /// Borrowed label building.
    pub building: &'a crate::Building,
    /// UTF-8 label text with invalid sequences replaced.
    pub decoded: std::borrow::Cow<'a, str>,
}
/// Train interface island and its index in the blueprint.
#[derive(Serialize)]
pub struct TrainInterfaceSummary<'a> {
    /// Index in the blueprint island entries.
    pub index: usize,
    /// Borrowed train interface island.
    pub island: &'a Island,
}
