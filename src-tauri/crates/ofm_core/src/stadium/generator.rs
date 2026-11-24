//! A believable ground for a club that has none yet.
//!
//! The ground's character follows its size: a non-league club gets a small
//! main stand and a covered terrace; a lower-league club a ground of four
//! separate stands, often old; a big club a modernised all-seater; the biggest
//! a bowl, sometimes under one roof. Within that, the dice pick roof types,
//! corners, floodlights and names, so no two clubs look alike — and the rows
//! are then sized until the ground holds what the club's capacity says.

use domain::stadium::{
    Cladding, CornerFill, Corners, Floodlights, Pitch, PitchSurface, Rake, RoofSystem,
    RoofSystemKind, STADIUM_LAYOUT_VERSION, Section, Side, StadiumLayout, StadiumSides, Stand,
    StandRoof, Tier, TierKind,
};
use rand::{Rng, RngExt};

use super::figures::capacity;

/// What the generator knows about the club.
#[derive(Debug, Clone, Copy)]
pub struct StadiumBrief {
    pub target_capacity: u32,
    /// The club's reputation, 0–1000.
    pub reputation: u32,
    pub founded_year: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Character {
    /// A main stand, a covered terrace, open ends.
    NonLeague,
    /// Four separate stands, terraces and seats, often old.
    Traditional,
    /// Four modern stands, all or mostly seated.
    Modernised,
    /// One continuous bowl of two to four tiers.
    Bowl,
}

fn character_for(capacity: u32) -> Character {
    match capacity {
        0..=2_499 => Character::NonLeague,
        2_500..=11_999 => Character::Traditional,
        12_000..=34_999 => Character::Modernised,
        _ => Character::Bowl,
    }
}

/// One stand's design before its rows are sized: tiers as relative row
/// counts, a section count, and whether the first and last sections stay empty.
#[derive(Debug, Clone)]
struct StandPlan {
    name: String,
    sections: usize,
    open_wings: bool,
    tiers: Vec<(TierKind, f64)>,
    roof: StandRoof,
    boxes: bool,
    rake: Rake,
    balcony: bool,
}

#[derive(Debug, Clone)]
struct GroundPlan {
    north: StandPlan,
    south: StandPlan,
    west: StandPlan,
    east: StandPlan,
    bowl: bool,
    roof_system: RoofSystem,
    floodlights: Floodlights,
    cladding: Cladding,
    corners: Corners,
    pitch: Pitch,
}

const SEATED_MAX_ROWS: f64 = 40.0;
const TERRACE_MAX_ROWS: f64 = 60.0;

/// Generate a ground for `brief`. Deterministic for a given `rng` state.
pub fn generate_stadium_layout(brief: &StadiumBrief, rng: &mut impl Rng) -> StadiumLayout {
    let plan = plan_ground(brief, rng);
    let target = brief.target_capacity.max(150);
    // Capacity rises with the row scale, so a bisection lands on the scale
    // whose ground holds closest to the target.
    let (mut low, mut high) = (0.05_f64, 6.0_f64);
    let mut best = (u32::MAX, build(&plan, 1.0));
    for _ in 0..40 {
        let mid = (low + high) / 2.0;
        let layout = build(&plan, mid);
        let held = capacity(&layout);
        let miss = held.abs_diff(target);
        if miss < best.0 {
            best = (miss, layout);
        }
        if held < target {
            low = mid;
        } else {
            high = mid;
        }
    }
    fine_tune(best.1, target)
}

/// One scale moves every tier at once, so the nearest scale can still be a
/// few hundred places out. Close the gap a row at a time, in whichever
/// section's top tier gets nearest, until no single row helps.
fn fine_tune(mut layout: StadiumLayout, target: u32) -> StadiumLayout {
    let mut miss = capacity(&layout).abs_diff(target);
    for _ in 0..64 {
        let mut best: Option<(u32, StadiumLayout)> = None;
        for candidate in one_row_changes(&layout) {
            let candidate_miss = capacity(&candidate).abs_diff(target);
            if candidate_miss < best.as_ref().map_or(miss, |(m, _)| *m) {
                best = Some((candidate_miss, candidate));
            }
        }
        match best {
            Some((m, better)) => {
                miss = m;
                layout = better;
            }
            None => break,
        }
    }
    layout
}

/// Every layout one row away from `layout`: one more or one fewer row in the
/// top tier of a single built section, within the row limits.
fn one_row_changes(layout: &StadiumLayout) -> Vec<StadiumLayout> {
    let mut out = Vec::new();
    for side in [Side::North, Side::South, Side::West, Side::East] {
        for bay in 0..layout.sides.get(side).bays.len() {
            for delta in [-1_i32, 1] {
                let mut next = layout.clone();
                let stand = match side {
                    Side::North => &mut next.sides.north,
                    Side::South => &mut next.sides.south,
                    Side::West => &mut next.sides.west,
                    Side::East => &mut next.sides.east,
                };
                let Some(tier) = stand.bays[bay].tiers.last_mut() else {
                    continue;
                };
                let max = if tier.kind == TierKind::Terrace {
                    TERRACE_MAX_ROWS
                } else {
                    SEATED_MAX_ROWS
                };
                let rows = i32::from(tier.rows) + delta;
                if rows < 3 || f64::from(rows) > max {
                    continue;
                }
                tier.rows = rows as u16;
                out.push(next);
            }
        }
    }
    out
}

fn build(plan: &GroundPlan, scale: f64) -> StadiumLayout {
    StadiumLayout {
        version: STADIUM_LAYOUT_VERSION,
        bowl: plan.bowl,
        roof_system: plan.roof_system,
        floodlights: plan.floodlights,
        cladding: plan.cladding,
        sides: StadiumSides {
            north: build_stand(&plan.north, scale),
            south: build_stand(&plan.south, scale),
            west: build_stand(&plan.west, scale),
            east: build_stand(&plan.east, scale),
        },
        corners: plan.corners,
        pitch: plan.pitch,
    }
}

fn build_stand(plan: &StandPlan, scale: f64) -> Stand {
    let tiers: Vec<Tier> = plan
        .tiers
        .iter()
        .map(|&(kind, rows)| {
            let max = if kind == TierKind::Terrace {
                TERRACE_MAX_ROWS
            } else {
                SEATED_MAX_ROWS
            };
            Tier {
                kind,
                rows: (rows * scale).round().clamp(3.0, max) as u16,
            }
        })
        .collect();
    let built = Section {
        tiers,
        roof: plan.roof,
        boxes: plan.boxes && plan.tiers.len() >= 2,
        rake: plan.rake,
        balcony: plan.balcony,
    };
    let bays = (0..plan.sections)
        .map(|i| {
            let wing = plan.open_wings && plan.sections >= 3 && (i == 0 || i + 1 == plan.sections);
            if wing {
                Section {
                    tiers: vec![],
                    roof: StandRoof::None,
                    boxes: false,
                    rake: Rake::Standard,
                    balcony: false,
                }
            } else {
                built.clone()
            }
        })
        .collect();
    Stand {
        name: plan.name.clone(),
        bays,
    }
}

fn plan_ground(brief: &StadiumBrief, rng: &mut impl Rng) -> GroundPlan {
    let character = character_for(brief.target_capacity);
    let names = super::names::stand_names(brief, rng);
    let heritage = brief.founded_year < 1920 && rng.random_bool(0.45);
    let all_seater = brief.target_capacity >= 20_000 || brief.reputation >= 800;
    let mut plan = match character {
        Character::NonLeague => non_league(rng, &names, brief.target_capacity),
        Character::Traditional => traditional(rng, &names, heritage, all_seater),
        Character::Modernised => modernised(rng, &names, all_seater),
        Character::Bowl => bowl(rng, &names, brief.target_capacity),
    };
    plan.pitch = pitch_for(brief, &plan, rng);
    plan
}

fn stand(name: &str, sections: usize, tiers: &[(TierKind, f64)], roof: StandRoof) -> StandPlan {
    StandPlan {
        name: name.to_string(),
        sections,
        open_wings: false,
        tiers: tiers.to_vec(),
        roof,
        boxes: false,
        rake: Rake::Standard,
        balcony: false,
    }
}

fn pick<T: Copy>(rng: &mut impl Rng, options: &[(T, f64)]) -> T {
    let total: f64 = options.iter().map(|(_, w)| w).sum();
    let mut roll = rng.random_range(0.0..total);
    for &(value, weight) in options {
        if roll < weight {
            return value;
        }
        roll -= weight;
    }
    options[options.len() - 1].0
}

fn non_league(rng: &mut impl Rng, names: &[String; 4], target: u32) -> GroundPlan {
    use TierKind::{Seated, Terrace};
    // Three rows is the smallest stand, so below ~1,500 the ground has to
    // shrink by leaving parts open rather than by thinning rows.
    let tiny = target < 1_500;
    let mut north = stand(&names[0], 3, &[(Seated, 6.0)], StandRoof::Posts);
    north.open_wings = tiny || rng.random_bool(0.6);
    let mut south = stand(
        &names[1],
        3,
        &[(Terrace, 6.0)],
        pick(rng, &[(StandRoof::Posts, 0.6), (StandRoof::Goalpost, 0.4)]),
    );
    south.open_wings = tiny || rng.random_bool(0.5);
    let end = |rng: &mut _, name: &str| {
        if tiny || rng_bool(rng, 0.5) {
            stand(name, 1, &[], StandRoof::None)
        } else {
            stand(name, 1, &[(Terrace, 3.0)], StandRoof::None)
        }
    };
    let west = end(rng, &names[2]);
    let east = end(rng, &names[3]);
    GroundPlan {
        north,
        south,
        west,
        east,
        bowl: false,
        roof_system: RoofSystem::default(),
        floodlights: pick(
            rng,
            &[(Floodlights::Pylons, 0.85), (Floodlights::None, 0.15)],
        ),
        cladding: Cladding::Sheet,
        corners: Corners::default(),
        pitch: Pitch::default(),
    }
}

fn rng_bool(rng: &mut impl Rng, p: f64) -> bool {
    rng.random_bool(p)
}

fn traditional(
    rng: &mut impl Rng,
    names: &[String; 4],
    heritage: bool,
    all_seater: bool,
) -> GroundPlan {
    use TierKind::{Rail, Seated, Terrace};
    let standing = if all_seater { Rail } else { Terrace };
    let sections = |rng: &mut _| pick(rng, &[(2usize, 0.3), (3, 0.5), (4, 0.2)]);
    // The main stand: a seated stand, sometimes over a terraced paddock; at
    // old clubs often a heritage stand with pitched roofs and a balcony.
    let main_tiers: Vec<(TierKind, f64)> = if rng.random_bool(0.4) {
        vec![(standing, 8.0), (Seated, 10.0)]
    } else {
        vec![(Seated, 14.0)]
    };
    let main_roof = if heritage {
        StandRoof::Pitched
    } else {
        pick(
            rng,
            &[
                (StandRoof::Cantilever, 0.5),
                (StandRoof::Goalpost, 0.3),
                (StandRoof::Posts, 0.2),
            ],
        )
    };
    let mut north = stand(&names[0], sections(rng), &main_tiers, main_roof);
    north.balcony = heritage && main_tiers.len() == 2;
    let south_kind = if rng.random_bool(0.55) {
        standing
    } else {
        Seated
    };
    let south = stand(
        &names[1],
        sections(rng),
        &[(south_kind, if south_kind == Terrace { 22.0 } else { 14.0 })],
        pick(
            rng,
            &[
                (StandRoof::Posts, 0.4),
                (StandRoof::Goalpost, 0.35),
                (StandRoof::Cantilever, 0.25),
            ],
        ),
    );
    // The home end is the bigger, steeper one; the away end is sometimes uncovered.
    let mut kop = stand(
        &names[2],
        pick(rng, &[(1usize, 0.4), (2, 0.6)]),
        &[(standing, if standing == Terrace { 30.0 } else { 16.0 })],
        pick(
            rng,
            &[
                (StandRoof::None, 0.3),
                (StandRoof::Goalpost, 0.35),
                (StandRoof::Cantilever, 0.35),
            ],
        ),
    );
    kop.rake = pick(rng, &[(Rake::Steep, 0.5), (Rake::Standard, 0.5)]);
    let away = stand(
        &names[3],
        pick(rng, &[(1usize, 0.5), (2, 0.5)]),
        &[(
            if rng.random_bool(0.5) {
                standing
            } else {
                Seated
            },
            12.0,
        )],
        pick(
            rng,
            &[
                (StandRoof::None, 0.4),
                (StandRoof::Posts, 0.3),
                (StandRoof::Cantilever, 0.3),
            ],
        ),
    );
    let (west, east) = if rng.random_bool(0.5) {
        (kop, away)
    } else {
        (away, kop)
    };
    let corner = |rng: &mut _| {
        if rng_bool(rng, 0.2) {
            if all_seater {
                CornerFill::Seated
            } else {
                CornerFill::Terrace
            }
        } else {
            CornerFill::Open
        }
    };
    GroundPlan {
        north,
        south,
        west,
        east,
        bowl: false,
        roof_system: RoofSystem::default(),
        floodlights: pick(
            rng,
            &[(Floodlights::Pylons, 0.75), (Floodlights::Masts, 0.25)],
        ),
        cladding: if heritage {
            Cladding::Brick
        } else {
            pick(rng, &[(Cladding::Sheet, 0.7), (Cladding::Brick, 0.3)])
        },
        corners: Corners {
            nw: corner(rng),
            ne: corner(rng),
            sw: corner(rng),
            se: corner(rng),
        },
        pitch: Pitch::default(),
    }
}

fn modernised(rng: &mut impl Rng, names: &[String; 4], all_seater: bool) -> GroundPlan {
    use TierKind::{Rail, Seated, Terrace};
    let home_kind = if rng.random_bool(0.4) {
        Rail
    } else if all_seater {
        Seated
    } else {
        pick(rng, &[(Seated, 0.6), (Terrace, 0.4)])
    };
    let mut north = stand(
        &names[0],
        pick(rng, &[(3usize, 0.5), (4, 0.5)]),
        &[(Seated, 18.0), (Seated, 12.0)],
        StandRoof::Cantilever,
    );
    north.boxes = true;
    let south_tiers: Vec<(TierKind, f64)> = if rng.random_bool(0.4) {
        vec![(Seated, 16.0), (Seated, 10.0)]
    } else {
        vec![(Seated, 22.0)]
    };
    let south = stand(
        &names[1],
        pick(rng, &[(3usize, 0.5), (4, 0.5)]),
        &south_tiers,
        StandRoof::Cantilever,
    );
    let mut kop = stand(
        &names[2],
        pick(rng, &[(2usize, 0.5), (3, 0.5)]),
        &[(home_kind, if home_kind == Terrace { 36.0 } else { 22.0 })],
        StandRoof::Cantilever,
    );
    kop.rake = pick(rng, &[(Rake::Steep, 0.6), (Rake::Standard, 0.4)]);
    let away = stand(
        &names[3],
        pick(rng, &[(2usize, 0.5), (3, 0.5)]),
        &[(Seated, 18.0)],
        StandRoof::Cantilever,
    );
    let (west, east) = if rng.random_bool(0.5) {
        (kop, away)
    } else {
        (away, kop)
    };
    let corner = |rng: &mut _| {
        if rng_bool(rng, 0.5) {
            CornerFill::Seated
        } else {
            CornerFill::Open
        }
    };
    GroundPlan {
        north,
        south,
        west,
        east,
        bowl: false,
        roof_system: RoofSystem::default(),
        floodlights: pick(
            rng,
            &[
                (Floodlights::Roof, 0.55),
                (Floodlights::Pylons, 0.35),
                (Floodlights::Masts, 0.1),
            ],
        ),
        cladding: pick(
            rng,
            &[
                (Cladding::Sheet, 0.5),
                (Cladding::Glass, 0.3),
                (Cladding::Slats, 0.2),
            ],
        ),
        corners: Corners {
            nw: corner(rng),
            ne: corner(rng),
            sw: corner(rng),
            se: corner(rng),
        },
        pitch: Pitch::default(),
    }
}

fn bowl(rng: &mut impl Rng, names: &[String; 4], capacity: u32) -> GroundPlan {
    use TierKind::{Rail, Seated};
    let tiers: Vec<(TierKind, f64)> = match capacity {
        0..=44_999 => vec![(Seated, 22.0), (Seated, 16.0)],
        45_000..=74_999 => vec![(Seated, 22.0), (Seated, 16.0), (Seated, 12.0)],
        _ => vec![
            (Seated, 24.0),
            (Seated, 18.0),
            (Seated, 14.0),
            (Seated, 10.0),
        ],
    };
    let side = |rng: &mut _, name: &str, long: bool| {
        let mut s = stand(
            name,
            if long {
                pick(rng, &[(3usize, 0.5), (4, 0.5)])
            } else {
                3
            },
            &tiers,
            StandRoof::Cantilever,
        );
        s.boxes = true;
        s
    };
    let north = side(rng, &names[0], true);
    let south = side(rng, &names[1], true);
    let mut kop = side(rng, &names[2], false);
    if rng.random_bool(0.4) {
        kop.tiers[0].0 = Rail;
    }
    let away = side(rng, &names[3], false);
    let (west, east) = if rng.random_bool(0.5) {
        (kop, away)
    } else {
        (away, kop)
    };
    let roof_kind = if capacity >= 60_000 && rng.random_bool(0.08) {
        RoofSystemKind::Arch
    } else if capacity >= 45_000 && rng.random_bool(0.3) {
        RoofSystemKind::Ring
    } else {
        RoofSystemKind::Stands
    };
    let retractable = roof_kind != RoofSystemKind::Stands && rng.random_bool(0.35);
    let all = CornerFill::Seated;
    GroundPlan {
        north,
        south,
        west,
        east,
        bowl: rng.random_bool(0.6),
        roof_system: RoofSystem {
            kind: roof_kind,
            retractable,
            closed: false,
        },
        floodlights: Floodlights::Roof,
        cladding: pick(
            rng,
            &[
                (Cladding::Glass, 0.35),
                (Cladding::Slats, 0.35),
                (Cladding::Sheet, 0.15),
                (Cladding::Lightup, 0.15),
            ],
        ),
        corners: Corners {
            nw: all,
            ne: all,
            sw: all,
            se: all,
        },
        pitch: Pitch::default(),
    }
}

/// Bigger, richer clubs play on hybrid grass with undersoil heating; small
/// clubs on natural grass, a few on an artificial pitch.
fn pitch_for(brief: &StadiumBrief, plan: &GroundPlan, rng: &mut impl Rng) -> Pitch {
    let covered = plan.roof_system.kind != RoofSystemKind::Stands;
    if brief.reputation >= 750 {
        return Pitch {
            surface: PitchSurface::Hybrid,
            undersoil_heating: true,
            grow_lights: covered || brief.target_capacity >= 40_000,
        };
    }
    if brief.reputation >= 450 {
        let hybrid = rng.random_bool(0.3);
        return Pitch {
            surface: if hybrid {
                PitchSurface::Hybrid
            } else {
                PitchSurface::Natural
            },
            undersoil_heating: rng.random_bool(0.7),
            grow_lights: covered,
        };
    }
    Pitch {
        surface: if rng.random_bool(0.12) {
            PitchSurface::Artificial
        } else {
            PitchSurface::Natural
        },
        undersoil_heating: rng.random_bool(0.15),
        grow_lights: false,
    }
}

#[cfg(test)]
mod tests;
