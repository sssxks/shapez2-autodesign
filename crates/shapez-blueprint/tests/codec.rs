use base64::{Engine, engine::general_purpose::STANDARD};
use flate2::{Compression, write::GzEncoder};
use shapez_blueprint::{
    Blueprint, BlueprintContent, BlueprintIcon, Building, BuildingBlueprint, Configuration,
    IconComponent, Island, IslandBlueprint, Rotation, decode_blueprint, encode_blueprint,
};
use std::io::Write;

fn legacy(version: u32, payload: &str) -> String {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(payload.as_bytes()).unwrap();
    format!(
        "SHAPEZ2-{version}-{}$",
        STANDARD.encode(gzip.finish().unwrap())
    )
}

fn sample() -> Blueprint {
    let mut building = Building::new("LabelDefaultInternalVariant", -3, 12, 2, Rotation::new(3));
    building.configuration = Some(Configuration("形状 🏭".as_bytes().to_vec()));
    let mut island = Island::new("Foundation_1x1", -20, 4, -1, Rotation::new(2));
    island.configuration = Some(Configuration(vec![0, 1, 2, 255]));
    island.buildings = Some(BuildingBlueprint::new(vec![building]));
    Blueprint {
        game_version: 1138,
        content: BlueprintContent::Islands(IslandBlueprint::new(vec![island])),
        icon: Some(BlueprintIcon {
            components: [
                Some(IconComponent::Icon("平台🏭".into())),
                None,
                Some(IconComponent::Shape("RuRuRuRu".into())),
                None,
            ],
        }),
        source_format_version: None,
    }
}

#[test]
fn current_envelope_preserves_typed_data_bytes_and_unicode_metadata() {
    let original = sample();
    let encoded = encode_blueprint(&original).unwrap();
    assert!(encoded.starts_with("SHAPEZ2-5-"));
    let mut decoded = decode_blueprint(&encoded).unwrap();
    assert_eq!(decoded.source_format_version, Some(5));
    decoded.source_format_version = None;
    assert_eq!(decoded, original);
    let json = serde_json::to_string(&decoded).unwrap();
    assert_eq!(serde_json::from_str::<Blueprint>(&json).unwrap(), original);
    assert_eq!(decoded.summary().labels[0].decoded, "形状 🏭");
}

#[test]
fn reads_newtonsoft_records_arrays_and_binary_wrappers_without_json_storage() {
    let text = legacy(
        1,
        r#"{
      "$type":"Game.Core.Blueprint.Serialization.SerializableBlueprint, Game.Core.Blueprint.Serialization",
      "V":1138,"BP":{"$type":"Island","Entries":{
        "$type":"Game.Core.Blueprint.Serialization.SerializableIslandEntry[], Game.Core.Blueprint.Serialization",
        "$values":[{"$type":"Game.Core.Blueprint.Serialization.SerializableIslandEntry, Game.Core.Blueprint.Serialization",
          "T":"Rail_Forward","X":-2,"S":{"$type":"System.Byte[], mscorlib","$value":"AQAAAAA="},
          "B":{"$type":"Building","Entries":{"$type":"Building[]","$values":[{"T":"BeltDefaultForwardInternalVariant"}]}}
        }]}}}"#,
    );
    let blueprint = decode_blueprint(&text).unwrap();
    assert_eq!(blueprint.source_format_version, Some(1));
    let island = &blueprint.islands()[0];
    assert_eq!((island.x, island.y, island.layer), (-2, 0, 0));
    assert_eq!(island.configuration.as_ref().unwrap().0, [1, 0, 0, 0, 0]);
    assert_eq!(island.buildings()[0].rotation, Rotation::default());
    let upgraded = decode_blueprint(&encode_blueprint(&blueprint).unwrap()).unwrap();
    assert_eq!(upgraded.content, blueprint.content);
    assert_eq!(upgraded.icon, Some(blueprint.default_icon()));
}

#[test]
fn migrates_embedded_v4_icon_to_v5_metadata() {
    let text = legacy(
        4,
        r#"{"V":1138,"BP":{"$type":"Building","Entries":[{"T":"BeltDefaultForwardInternalVariant","X":3}],"Icon":{"Data":["icon:Buildings",null,null,"shape:CuCuCuCu"]}}}"#,
    );
    let original = decode_blueprint(&text).unwrap();
    assert!(matches!(original.content, BlueprintContent::Buildings(_)));
    assert_eq!(original.summary().buildings, 1);
    let encoded = encode_blueprint(&original).unwrap();
    let decoded = decode_blueprint(&encoded).unwrap();
    assert_eq!(decoded.icon, original.icon);
    assert_eq!(decoded.content, original.content);
    assert!(encoded.starts_with("SHAPEZ2-5-"));
}

#[test]
fn rejects_unknown_fields_wrong_types_and_out_of_range_coordinates() {
    for payload in [
        r#"{"V":1138,"BP":{"$type":"Island","Entries":[{"T":"Rail_Forward","X":"bad"}]}}"#,
        r#"{"V":1138,"BP":{"$type":"Island","Entries":[{"T":"Rail_Forward","X":2147483648}]}}"#,
        r#"{"V":1138,"BP":{"$type":"Island","Entries":[{"T":"Rail_Forward","Z":32768}]}}"#,
        r#"{"V":1138,"BP":{"$type":"Building","Entries":[{"T":"Belt","R":4}]}}"#,
        r#"{"V":1138,"BP":{"$type":"Building","Entries":[{"T":"Belt","C":"bad!"}]}}"#,
        r#"{"V":1138,"BP":{"$type":"Island","Entries":[{"T":"Rail_Forward","Unexpected":123}]}}"#,
        r#"{"V":1138,"BP":{"$type":"Island","Entries":[{}]}}"#,
        r#"{"V":1138,"BP":{"$type":"Unknown","Entries":[]}}"#,
        r#"{"V":1138,"BP":{"$type":"Island"}}"#,
    ] {
        assert!(
            decode_blueprint(&legacy(1, payload)).is_err(),
            "Accepted {payload}"
        );
    }
}

#[test]
fn rejects_broken_envelopes_instead_of_truncating_the_payload() {
    let encoded = encode_blueprint(&sample()).unwrap();
    for broken in [
        encoded.trim_end_matches('$').to_owned(),
        encoded.replacen("SHAPEZ2-5-", "SHAPEZ2-6-", 1),
        encoded.rsplit_once('_').unwrap().0.to_owned() + "_999999999$",
        encoded.rsplit_once('_').unwrap().0.to_owned() + "_invalid$",
        "SHAPEZ2-5-invalid$".into(),
    ] {
        assert!(decode_blueprint(&broken).is_err());
    }
    assert!(decode_blueprint("SHAPEZ2-1-invalid$").is_err());
}
