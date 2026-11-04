use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
};

pub fn decode_blueprint(text: &str) -> Result<Value> {
    let mut parts = text.trim().splitn(3, '-');
    if parts.next() != Some("SHAPEZ2") {
        bail!("Missing SHAPEZ2 prefix")
    }
    let _version = parts.next().context("Missing format version")?;
    let body = parts.next().context("Missing payload")?;
    let end = body
        .find(|c: char| !c.is_ascii_alphanumeric() && !['+', '/', '='].contains(&c))
        .unwrap_or(body.len());
    let mut payload = body[..end].to_owned();
    while payload.len() % 4 != 0 {
        payload.push('=')
    }
    let bytes = STANDARD
        .decode(payload)
        .context("Invalid blueprint base64")?;
    let mut decoded = String::new();
    GzDecoder::new(bytes.as_slice()).read_to_string(&mut decoded)?;
    let mut value: Value = serde_json::from_str(&decoded)?;
    normalize(&mut value);
    Ok(value)
}
fn normalize(v: &mut Value) {
    if let Some(a) = v.get_mut("$values") {
        let mut a = a.take();
        normalize(&mut a);
        *v = a;
        return;
    }
    if let Some(s) = v.get_mut("$value") {
        *v = s.take();
        return;
    }
    match v {
        Value::Object(m) => {
            if m.get("$type")
                .and_then(Value::as_str)
                .is_some_and(|s| s.contains(','))
            {
                m.remove("$type");
            }
            for a in m.values_mut() {
                normalize(a)
            }
        }
        Value::Array(a) => {
            for v in a {
                normalize(v)
            }
        }
        _ => {}
    }
}
pub fn encode_blueprint(v: &Value) -> Result<String> {
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&serde_json::to_vec(v)?)?;
    Ok(format!("SHAPEZ2-1-{}$", STANDARD.encode(gz.finish()?)))
}
pub struct Blueprint {
    pub value: Value,
}
impl Blueprint {
    pub fn read(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(Self {
            value: if text.trim_start().starts_with('{') {
                serde_json::from_str(&text)?
            } else {
                decode_blueprint(&text)?
            },
        })
    }
    pub fn islands(&self) -> &[Value] {
        self.value["BP"]["Entries"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
    pub fn summary(&self) -> Value {
        let mut types = BTreeMap::new();
        let mut buildings = 0;
        let mut labels = vec![];
        let mut interfaces = vec![];
        for (index, island) in self.islands().iter().enumerate() {
            let kind = island["T"].as_str().unwrap_or("?");
            *types.entry(kind).or_insert(0) += 1;
            let entries = island["B"]["Entries"].as_array();
            if let Some(entries) = entries {
                buildings += entries.len();
                for e in entries {
                    if e["T"].as_str().unwrap_or("").contains("Label") {
                        labels.push(json!({"island":index,"chunk":[n(island,"X"),n(island,"Y")],"building":e,"decoded":config_text(e)}));
                    }
                }
            }
            if kind.contains("Train") {
                interfaces.push(json!({"index":index,"island":island}));
            }
        }
        json!({"version":self.value["V"],"islands":self.islands().len(),"buildings":buildings,"types":types,"labels":labels,"train_interfaces":interfaces})
    }
}
fn config_text(e: &Value) -> String {
    e["C"]
        .as_str()
        .and_then(|s| STANDARD.decode(s).ok())
        .map(|v| String::from_utf8_lossy(&v).into_owned())
        .unwrap_or_default()
}
fn n(v: &Value, k: &str) -> i64 {
    v[k].as_i64().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_and_metadata_roundtrip() {
        let v = json!({"V":1138,"BP":{"$type":"Island","Entries":[]}});
        let encoded = encode_blueprint(&v).unwrap();
        assert_eq!(decode_blueprint(&encoded).unwrap(), v);
        let metadata = encoded.trim_end_matches('$').to_owned() + "[{\"Type\":\"icon\"}]_171$";
        assert_eq!(decode_blueprint(&metadata).unwrap(), v);
    }
    #[test]
    fn typed_arrays() {
        let mut v = json!({"Entries":{"$type":"x","$values":[{"C":{"$value":"AA=="}}]}});
        normalize(&mut v);
        assert_eq!(v["Entries"][0]["C"], "AA==");
    }
}
