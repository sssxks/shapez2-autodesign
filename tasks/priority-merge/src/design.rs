//! Physical input sides and the three user-facing merger variants.
use shapez_blueprint::{BlueprintIcon, IconComponent};

/// An input side of the east-facing foundation.
#[derive(Clone, Copy)]
pub(crate) enum Side {
    West,
    North,
    South,
}
impl Side {
    /// Port tile and inward direction, with lane order increasing along the edge.
    pub(crate) fn port(self, lane: i32) -> ((i32, i32), usize) {
        match self {
            Self::West => ((2, 8 + lane), 0),
            Self::North => ((8 + lane, 2), 1),
            Self::South => ((8 + lane, 17), 3),
        }
    }
    /// Native test protocol label.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::West => "west",
            Self::North => "north",
            Self::South => "south",
        }
    }
    /// Adjacent space belt location and rotation for the native fixture.
    pub(crate) fn harness(self) -> (i32, i32, usize) {
        match self {
            Self::West => (-1, 0, 0),
            Self::North => (0, -1, 1),
            Self::South => (0, 1, 3),
        }
    }
}

/// A complete external-interface choice; internal cells always prioritize their A input.
pub(crate) struct Design {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) primary: Side,
    pub(crate) secondary: Side,
}
pub(crate) const DESIGNS: [Design; 3] = [
    Design {
        id: "inline",
        name: "直列优先合并器 Inline Priority Merger",
        primary: Side::West,
        secondary: Side::North,
    },
    Design {
        id: "reverse-inline",
        name: "反向直列优先合并器 Reverse Inline Priority Merger",
        primary: Side::North,
        secondary: Side::West,
    },
    Design {
        id: "priority",
        name: "优先合并器 Priority Merger",
        primary: Side::North,
        secondary: Side::South,
    },
];
impl Design {
    /// Match the library's red/green two-input icon convention: green is preferred.
    pub(crate) fn icon(&self) -> BlueprintIcon {
        let inline = matches!(self.primary, Side::West);
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
                    if matches!(self.secondary, Side::South) {
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
