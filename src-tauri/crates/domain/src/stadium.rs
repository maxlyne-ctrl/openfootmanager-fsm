//! A club's ground, as it is built: four stands made of sections, each section
//! a stack of tiers under a roof, the corners between them, a roof system over
//! the whole ground, floodlights, cladding and the pitch.
//!
//! Data only. What a layout *gives* — capacity, cover, atmosphere — is worked
//! out in `ofm_core::stadium`, so the rules live in one place.
//!
//! The JSON shape (camelCase, lowercase variants) is the same one the Stand
//! Builder prototype reads and writes, so a ground can move between the game
//! and the builder unchanged.

use serde::{Deserialize, Serialize};

/// Version of the layout format. Bumped when a change needs a migration.
pub const STADIUM_LAYOUT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StadiumLayout {
    #[serde(default = "default_version")]
    pub version: u32,
    /// Curved corners joining the stands into one bowl.
    #[serde(default)]
    pub bowl: bool,
    #[serde(default)]
    pub roof_system: RoofSystem,
    #[serde(default)]
    pub floodlights: Floodlights,
    #[serde(default)]
    pub cladding: Cladding,
    pub sides: StadiumSides,
    #[serde(default)]
    pub corners: Corners,
    #[serde(default)]
    pub pitch: Pitch,
}

fn default_version() -> u32 {
    STADIUM_LAYOUT_VERSION
}

/// The four sides of the ground. North and south run along the touchlines,
/// west and east behind the goals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StadiumSides {
    pub north: Stand,
    pub south: Stand,
    pub west: Stand,
    pub east: Stand,
}

impl StadiumSides {
    /// The sides in a fixed order: north, east, south, west.
    pub fn iter(&self) -> [(Side, &Stand); 4] {
        [
            (Side::North, &self.north),
            (Side::East, &self.east),
            (Side::South, &self.south),
            (Side::West, &self.west),
        ]
    }

    pub fn get(&self, side: Side) -> &Stand {
        match side {
            Side::North => &self.north,
            Side::South => &self.south,
            Side::West => &self.west,
            Side::East => &self.east,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    North,
    South,
    West,
    East,
}

impl Side {
    /// Length of the side in metres: the pitch (105 × 68) plus 6 m of run-off each way.
    pub fn length_m(self) -> f64 {
        match self {
            Side::North | Side::South => 117.0,
            Side::West | Side::East => 80.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stand {
    pub name: String,
    /// Sections along the side, west to east or north to south. One to four.
    pub bays: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    /// Lowest tier first. Empty means an open grass bank behind a rail.
    #[serde(default)]
    pub tiers: Vec<Tier>,
    #[serde(default)]
    pub roof: StandRoof,
    /// Executive boxes between the first and second tiers.
    #[serde(default)]
    pub boxes: bool,
    #[serde(default)]
    pub rake: Rake,
    /// Criss-cross steel balcony on the upper tier front. Appearance only.
    #[serde(default)]
    pub balcony: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tier {
    pub kind: TierKind,
    pub rows: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TierKind {
    #[default]
    Seated,
    /// Seats with a barrier: counts as seating, stood in like a terrace.
    Rail,
    Terrace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StandRoof {
    #[default]
    None,
    Posts,
    Goalpost,
    Pitched,
    Cantilever,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rake {
    Gentle,
    #[default]
    Standard,
    Steep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RoofSystem {
    #[serde(rename = "type", default)]
    pub kind: RoofSystemKind,
    #[serde(default)]
    pub retractable: bool,
    #[serde(default)]
    pub closed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoofSystemKind {
    /// Each section has its own roof (or none).
    #[default]
    Stands,
    /// A cable-net roof over the whole ground.
    Ring,
    /// A ring roof carried by a landmark arch.
    Arch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Floodlights {
    None,
    #[default]
    Pylons,
    Masts,
    Roof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cladding {
    #[default]
    Sheet,
    Brick,
    Glass,
    Slats,
    Lightup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Corners {
    #[serde(default)]
    pub nw: CornerFill,
    #[serde(default)]
    pub ne: CornerFill,
    #[serde(default)]
    pub sw: CornerFill,
    #[serde(default)]
    pub se: CornerFill,
}

impl Corners {
    pub fn iter(&self) -> [(Corner, CornerFill); 4] {
        [
            (Corner::Nw, self.nw),
            (Corner::Ne, self.ne),
            (Corner::Sw, self.sw),
            (Corner::Se, self.se),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Corner {
    Nw,
    Ne,
    Sw,
    Se,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CornerFill {
    #[default]
    Open,
    Terrace,
    Seated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pitch {
    #[serde(default)]
    pub surface: PitchSurface,
    /// Keeps the pitch playable in a freeze.
    #[serde(default)]
    pub undersoil_heating: bool,
    /// Keeps grass growing under deep roofs and in dark months.
    #[serde(default)]
    pub grow_lights: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PitchSurface {
    #[default]
    Natural,
    Hybrid,
    Artificial,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given a ground exported by the Stand Builder prototype, when the game
    /// reads it, then every part arrives — the two share one JSON shape.
    #[test]
    fn a_ground_from_the_stand_builder_reads_into_the_game() {
        let json = r#"{
          "version": 1, "bowl": true,
          "roofSystem": { "type": "arch", "retractable": true, "closed": false },
          "floodlights": "roof", "cladding": "lightup",
          "sides": {
            "north": { "name": "Main Stand", "bays": [ { "tiers": [ { "kind": "seated", "rows": 22 }, { "kind": "rail", "rows": 14 } ], "roof": "cantilever", "boxes": true, "rake": "steep", "balcony": false } ] },
            "south": { "name": "Riverside", "bays": [ { "tiers": [ { "kind": "terrace", "rows": 30 } ], "roof": "goalpost" } ] },
            "west":  { "name": "Kop", "bays": [ { "tiers": [] } ] },
            "east":  { "name": "Town End", "bays": [ { "tiers": [ { "kind": "seated", "rows": 10 } ], "roof": "pitched", "balcony": true } ] }
          },
          "corners": { "nw": "seated", "ne": "open", "sw": "terrace", "se": "seated" },
          "pitch": { "surface": "hybrid", "undersoilHeating": true, "growLights": true }
        }"#;
        let layout: StadiumLayout = serde_json::from_str(json).expect("the builder's shape");
        assert!(layout.bowl);
        assert_eq!(layout.roof_system.kind, RoofSystemKind::Arch);
        assert!(layout.roof_system.retractable);
        assert_eq!(layout.floodlights, Floodlights::Roof);
        assert_eq!(layout.cladding, Cladding::Lightup);
        let main = &layout.sides.north.bays[0];
        assert_eq!(
            main.tiers[1],
            Tier {
                kind: TierKind::Rail,
                rows: 14
            }
        );
        assert_eq!(main.rake, Rake::Steep);
        assert!(main.boxes);
        assert_eq!(
            layout.sides.south.bays[0].rake,
            Rake::Standard,
            "rake defaults to standard"
        );
        assert!(layout.sides.west.bays[0].tiers.is_empty());
        assert_eq!(layout.corners.sw, CornerFill::Terrace);
        assert_eq!(layout.pitch.surface, PitchSurface::Hybrid);
        assert!(layout.pitch.undersoil_heating && layout.pitch.grow_lights);
    }

    /// Given a layout, when it is written and read back, then nothing changes.
    #[test]
    fn a_layout_survives_a_json_round_trip() {
        let section = Section {
            tiers: vec![Tier {
                kind: TierKind::Terrace,
                rows: 40,
            }],
            roof: StandRoof::Posts,
            boxes: false,
            rake: Rake::Gentle,
            balcony: true,
        };
        let stand = |name: &str| Stand {
            name: name.into(),
            bays: vec![section.clone()],
        };
        let layout = StadiumLayout {
            version: STADIUM_LAYOUT_VERSION,
            bowl: false,
            roof_system: RoofSystem::default(),
            floodlights: Floodlights::Masts,
            cladding: Cladding::Brick,
            sides: StadiumSides {
                north: stand("A"),
                south: stand("B"),
                west: stand("C"),
                east: stand("D"),
            },
            corners: Corners {
                nw: CornerFill::Seated,
                ..Corners::default()
            },
            pitch: Pitch {
                surface: PitchSurface::Artificial,
                undersoil_heating: false,
                grow_lights: false,
            },
        };
        let json = serde_json::to_string(&layout).expect("serialises");
        assert!(
            json.contains("\"roofSystem\":{\"type\":\"stands\""),
            "{json}"
        );
        let back: StadiumLayout = serde_json::from_str(&json).expect("reads back");
        assert_eq!(back, layout);
    }

    /// Given a layout written before the pitch, corners and cladding existed,
    /// when it is read, then they take their defaults rather than failing.
    #[test]
    fn a_minimal_layout_takes_defaults() {
        let json = r#"{ "sides": {
            "north": { "name": "N", "bays": [] }, "south": { "name": "S", "bays": [] },
            "west": { "name": "W", "bays": [] }, "east": { "name": "E", "bays": [] } } }"#;
        let layout: StadiumLayout = serde_json::from_str(json).expect("minimal layout");
        assert_eq!(layout.version, STADIUM_LAYOUT_VERSION);
        assert_eq!(layout.pitch, Pitch::default());
        assert_eq!(layout.corners, Corners::default());
        assert_eq!(layout.floodlights, Floodlights::Pylons);
    }
}
