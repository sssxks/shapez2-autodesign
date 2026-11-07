//! Deterministic assembly from task-owned solved bank definitions.
use anyhow::{Context, Result, ensure};
use shapez_blueprint::{
    Blueprint, BlueprintContent, BlueprintIcon, Building, BuildingBlueprint, IconComponent, Island,
    IslandBlueprint, Rotation,
};
use shapez_model::{Direction, FoundationGeometry, FoundationSocket};
/// Reflect connectors while preserving the actual shape rotation operation.
fn reflect(mut b: Building) -> Building {
    b.y = 13 - b.y;
    b.rotation = Rotation::new((4 - b.rotation.index()) % 4);
    if [
        "CutterDefault",
        "BeltDefaultLeft",
        "Merger2To1L",
        "Splitter1To2L",
    ]
    .iter()
    .any(|prefix| b.definition.starts_with(prefix))
        || b.definition.starts_with("Lift") && b.definition.contains("Left")
    {
        b.definition = if let Some(base) = b.definition.strip_suffix("Mirrored") {
            base.to_owned()
        } else {
            format!("{}Mirrored", b.definition)
        };
    }
    b
}

/// Physical interfaces of the delivered west-input factory.
pub fn factory_sockets() -> [FoundationSocket; 5] {
    [
        (Direction::West, 0, false, "input"),
        (Direction::North, 0, true, "nw"),
        (Direction::North, 1, true, "ne"),
        (Direction::South, 0, true, "sw"),
        (Direction::South, 1, true, "se"),
    ]
    .map(|(side, segment, output, role)| FoundationSocket {
        side,
        segment,
        output,
        role,
    })
}
/// Combine local bank solutions, reflect the southern half, and attach external ports.
pub fn assemble_banks(left: &[Building], right: &[Building]) -> Result<Blueprint> {
    let mut buildings = Vec::new();
    for (bank, x_offset) in [(left, 0), (right, 17)] {
        let layers = bank.iter().map(|b| b.layer).max().context("Empty bank")? + 1;
        ensure!(
            layers == 1 || layers == 3,
            "Banks must use one or three layers"
        );
        for layer in 0..if layers == 1 { 3 } else { 1 } {
            for b in bank {
                ensure!(
                    (0..17).contains(&b.x) && (0..7).contains(&b.y),
                    "Bank outside 17x7"
                );
                for mut b in [b.clone(), reflect(b.clone())] {
                    b.x += 3 + x_offset;
                    b.y += 3;
                    b.layer += layer;
                    buildings.push(b);
                }
            }
        }
    }
    let sockets = factory_sockets();
    let geometry = FoundationGeometry::strip(2, 3);
    for socket in sockets {
        for z in 0..3 {
            for lane in 0..4 {
                let p = geometry.socket_tile(socket, lane, z);
                buildings.push(Building::new(
                    if socket.output {
                        "BeltPortSenderInternalVariant"
                    } else {
                        "BeltPortReceiverInternalVariant"
                    },
                    p.0,
                    p.1,
                    p.2,
                    Rotation::new(socket.direction()),
                ));
            }
        }
    }
    let mut island = Island::new("Foundation_1x2", 0, 0, 0, Rotation::new(0));
    island.buildings = Some(BuildingBlueprint::new(buildings));
    let bp = Blueprint {
        game_version: 1138,
        content: BlueprintContent::Islands(IslandBlueprint::new(vec![island])),
        icon: Some(BlueprintIcon {
            components: [
                Some(IconComponent::Shape("CuCuCuCu".into())),
                Some(IconComponent::Shape("Cu------".into())),
                Some(IconComponent::Shape("Cu------".into())),
                Some(IconComponent::Shape("Cu------".into())),
            ],
        }),
        source_format_version: None,
    };
    Ok(bp)
}
