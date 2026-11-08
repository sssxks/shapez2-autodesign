//! Full-rate input rotation before the original overflow cutter.
use anyhow::Result;
use shapez_blueprint::{Building, BuildingBlueprint, Rotation};
use shapez_model::{FoundationGeometry, TileNet, TileRoutingProblem};
/// Four pairs of rotators feed the original parallel cutters directly.
pub fn layout(source: &BuildingBlueprint) -> Result<Vec<Building>> {
    let mut base: Vec<_> = source
        .entries
        .iter()
        .filter(|b| b.layer == 0 && b.y >= 6)
        .cloned()
        .collect();
    for b in &mut base {
        if (b.y == 16 && [9, 10, 11, 28, 29, 30].contains(&b.x))
            || ([7, 32].contains(&b.x) && [7, 9, 11, 13].contains(&b.y))
            || (b.x == 36 && b.y == 6)
        {
            b.definition = "BeltDefaultForwardInternalVariant".into();
        }
    }
    // Stagger only the second pair down one row to leave both eastbound input routes open.
    let mask = 2;
    for (g, (x, mirror)) in [(3, true), (17, false), (20, true), (34, false)]
        .into_iter()
        .enumerate()
    {
        let y = 3 + ((mask >> g) & 1);
        if y == 3 {
            base.push(Building::new(
                "BeltDefaultForwardInternalVariant",
                if mirror { x } else { x + 2 },
                5,
                0,
                Rotation::new(1),
            ));
        }
        for (name, dx, dy, r) in [
            ("Splitter1To2LInternalVariantMirrored", 0, 0, 0),
            ("RotatorOneQuadInternalVariant", 1, 0, 0),
            ("BeltDefaultLeftInternalVariantMirrored", 2, 0, 0),
            ("BeltDefaultLeftInternalVariant", 0, 1, 1),
            ("RotatorOneQuadInternalVariant", 1, 1, 0),
            ("Merger2To1LInternalVariantMirrored", 2, 1, 1),
        ] {
            let mut name = name.to_owned();
            if mirror && !name.starts_with("Rotator") {
                name = if name.ends_with("Mirrored") {
                    name.trim_end_matches("Mirrored").into()
                } else {
                    format!("{name}Mirrored")
                };
            }
            base.push(Building::new(
                name,
                x + if mirror { 2 - dx } else { dx },
                y + dy,
                0,
                Rotation::new(if mirror { (6 - r) % 4 } else { r }),
            ));
        }
    }
    let mut buildings = base;

    let fixed = buildings.iter().map(|b| (b.x, b.y, b.layer)).collect();
    let nets = (0..4)
        .map(|g| {
            let (x, d) = [
                (6, shapez_model::Direction::West),
                (16, shapez_model::Direction::East),
                (23, shapez_model::Direction::West),
                (33, shapez_model::Direction::East),
            ][g];
            TileNet {
                a: (8 + g as i32, 2, 0),
                ad: shapez_model::Direction::South,
                b: (x, 3 + ((mask >> g) & 1), 0),
                bd: d,
                input: true,
                output: false,
                exchange: None,
            }
        })
        .collect();
    let routes = shapez_routing::solve_tiles_sequential(
        TileRoutingProblem {
            geometry: FoundationGeometry::strip(2, 1),
            fixed,
            nets,
        },
        shapez_model::SearchOptions {
            attempts: 100,
            seed: 1234,
        },
    )?;
    for route in routes {
        shapez_blueprint::route_buildings(&route, &mut buildings);
    }

    let input: Vec<_> = buildings.iter().filter(|b| b.y < 6).cloned().collect();
    for z in 1..3 {
        buildings.extend(input.iter().cloned().map(|mut b| {
            b.layer = z;
            b
        }));
        buildings.extend(
            source
                .entries
                .iter()
                .filter(|b| b.layer == z && b.y >= 6)
                .cloned()
                .map(|mut b| {
                    if (b.y == 16 && [9, 10, 11, 28, 29, 30].contains(&b.x))
                        || ([7, 32].contains(&b.x) && [7, 9, 11, 13].contains(&b.y))
                        || (b.x == 36 && b.y == 6)
                    {
                        b.definition = "BeltDefaultForwardInternalVariant".into();
                    }
                    b
                }),
        );
    }
    Ok(buildings)
}
