//! Physical input sides and the three user-facing merger variants.
use shapez_blueprint::{BlueprintIcon, IconComponent};

use shapez_model::{Direction, FoundationSocket};
/// A complete external-interface choice; internal cells always prioritize their A input.
pub(crate) struct Design {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) primary: Direction,
    pub(crate) secondary: Direction,
}
pub(crate) const DESIGNS: [Design; 3] = [
    Design {
        id: "inline",
        name: "直列优先合并器 Inline Priority Merger",
        primary: Direction::West,
        secondary: Direction::North,
    },
    Design {
        id: "reverse-inline",
        name: "反向直列优先合并器 Reverse Inline Priority Merger",
        primary: Direction::North,
        secondary: Direction::West,
    },
    Design {
        id: "priority",
        name: "优先合并器 Priority Merger",
        primary: Direction::North,
        secondary: Direction::South,
    },
];
impl Design {
    pub(crate) fn sockets(&self) -> [FoundationSocket; 3] {
        [
            FoundationSocket {
                side: self.primary,
                segment: 0,
                output: false,
                role: "primary",
            },
            FoundationSocket {
                side: self.secondary,
                segment: 0,
                output: false,
                role: "secondary",
            },
            FoundationSocket {
                side: Direction::East,
                segment: 0,
                output: true,
                role: "output",
            },
        ]
    }

    /// Match the library's red/green two-input icon convention: green is preferred.
    pub(crate) fn icon(&self) -> BlueprintIcon {
        let inline = matches!(self.primary, Direction::West);
        BlueprintIcon {
            components: [
                Some(IconComponent::Icon("Empty".into())),
                Some(IconComponent::Shape(
                    if inline { "CrCrCrCr" } else { "CgCgCgCg" }.into(),
                )),
                Some(IconComponent::Shape(
                    if inline { "CgCgCgCg" } else { "CrCrCrCr" }.into(),
                )),
                Some(IconComponent::Icon(
                    if matches!(self.secondary, Direction::South) {
                        "building.MergerTShapeVariant"
                    } else {
                        "building.Merger2To1Variant"
                    }
                    .into(),
                )),
            ],
        }
    }
}
