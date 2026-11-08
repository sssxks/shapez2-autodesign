//! Native building encoding for solved tile routes.
use crate::{Building, Rotation};
use shapez_model::TileRoute;
/// Append native belts, lifts and launchers for a solved route.
pub fn route_buildings(route: &TileRoute, b: &mut Vec<Building>) {
    let n = &route.net;
    let path = &route.steps;
    for (i, s) in path.iter().enumerate() {
        let next = path.get(i + 1);
        let d = next.map_or(n.bd, |v| v.d);
        let z = next.map_or(s.p.2, |v| v.p.2);
        let delta = d.turns_from(s.d);
        let jump = next.is_some_and(|v| (v.p.0 - s.p.0).abs() + (v.p.1 - s.p.1).abs() > 1);
        let t = if z != s.p.2 {
            format!(
                "Lift{}{}{}InternalVariant{}",
                (z - s.p.2).abs(),
                if z > s.p.2 { "Up" } else { "Down" },
                match delta {
                    0 => "Forward",
                    2 => "Backward",
                    _ => "Left",
                },
                if delta == 1 { "Mirrored" } else { "" }
            )
        } else if s.catch || i == 0 && n.input {
            "BeltPortReceiverInternalVariant".into()
        } else if jump || i + 1 == path.len() && n.output {
            "BeltPortSenderInternalVariant".into()
        } else {
            match delta {
                0 => "BeltDefaultForwardInternalVariant",
                1 => "BeltDefaultLeftInternalVariantMirrored",
                3 => "BeltDefaultLeftInternalVariant",
                _ => unreachable!(),
            }
            .into()
        };
        b.push(Building::new(
            t,
            s.p.0,
            s.p.1,
            s.p.2,
            Rotation::new(s.d.index()),
        ));
    }
}
