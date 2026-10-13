//! Stand names for a generated ground.
//!
//! Names are world data, like club and stadium names, not interface text: a
//! player can rename a stand, and a database editor can set them. They are
//! drawn from fictional British pools because the generator has no real
//! grounds to copy.

use rand::{Rng, RngExt};

use super::generator::StadiumBrief;

const LANDMARKS: &[&str] = &[
    "Station",
    "Church",
    "Park",
    "Canal",
    "Mill",
    "Bridge",
    "Market",
    "Railway",
    "Town",
    "Gasworks",
    "Brewery",
    "Quarry",
    "Abbey",
    "Harbour",
    "School",
    "Cricket Ground",
];

const SIDE_NAMES: &[&str] = &[
    "South Stand",
    "Popular Side",
    "Riverside",
    "Family Stand",
    "East Stand",
    "Paddock",
];

const KOP_NAMES: &[&str] = &["Kop", "Shed End", "Home End", "North Bank"];

/// Four names in the order main stand, opposite side, home end, away end.
pub(super) fn stand_names(brief: &StadiumBrief, rng: &mut impl Rng) -> [String; 4] {
    let main = if brief.founded_year < 1920 && rng.random_bool(0.2) {
        "Centenary Stand".to_string()
    } else {
        "Main Stand".to_string()
    };
    let opposite = SIDE_NAMES[rng.random_range(0..SIDE_NAMES.len())].to_string();
    let home = if rng.random_bool(0.5) {
        KOP_NAMES[rng.random_range(0..KOP_NAMES.len())].to_string()
    } else {
        end_name(rng, None)
    };
    let away = end_name(rng, Some(&home));
    [main, opposite, home, away]
}

/// "{Landmark} End", never the same as `taken`.
fn end_name(rng: &mut impl Rng, taken: Option<&str>) -> String {
    loop {
        let name = format!("{} End", LANDMARKS[rng.random_range(0..LANDMARKS.len())]);
        if Some(name.as_str()) != taken {
            return name;
        }
    }
}
