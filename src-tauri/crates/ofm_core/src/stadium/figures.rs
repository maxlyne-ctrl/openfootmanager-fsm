//! What a ground gives: places, seats, cover, views and atmosphere.
//!
//! The geometry is the Stand Builder's, metre for metre: a tier steps back
//! and up from the front of its stand, row by row; an upper tier overhangs
//! the one below; a roof's front edge decides which rows it really keeps dry.
//! Keep the two in step — the builder reads the same layouts.

use domain::stadium::{
    Corner, CornerFill, Rake, RoofSystemKind, Section, Side, StadiumLayout, StandRoof, TierKind,
};

/// Row depth (m), rise per rake (m) and people per metre of row.
struct KindGeometry {
    row_depth: f64,
    rise: [f64; 3],
    per_metre: f64,
}

fn geometry(kind: TierKind) -> KindGeometry {
    match kind {
        // 0.5 m seats with 8% lost to gangways.
        TierKind::Seated | TierKind::Rail => KindGeometry {
            row_depth: 0.8,
            rise: [0.32, 0.42, 0.52],
            per_metre: 1.84,
        },
        TierKind::Terrace => KindGeometry {
            row_depth: 0.42,
            rise: [0.15, 0.19, 0.23],
            per_metre: 1.43,
        },
    }
}

fn rake_index(rake: Rake) -> usize {
    match rake {
        Rake::Gentle => 0,
        Rake::Standard => 1,
        Rake::Steep => 2,
    }
}

/// Where a tier sits in its stand's own frame: `depth` outward from the
/// front of the stand, `height` up from the pitch.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TierProfile {
    pub kind: TierKind,
    pub rows: u16,
    pub row_depth: f64,
    pub rise: f64,
    pub per_metre: f64,
    pub front_depth: f64,
    pub front_height: f64,
    pub back_depth: f64,
    pub top: f64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SectionProfile {
    pub tiers: Vec<TierProfile>,
    pub back_depth: f64,
    pub top: f64,
}

pub(crate) fn section_profile(section: &Section) -> SectionProfile {
    let mut tiers: Vec<TierProfile> = Vec::with_capacity(section.tiers.len());
    for (i, tier) in section.tiers.iter().enumerate() {
        let g = geometry(tier.kind);
        let rise = g.rise[rake_index(section.rake)];
        let (front_depth, front_height) = match tiers.last() {
            None => (2.0, 1.1),
            Some(prev) => {
                // An upper tier overhangs the one below by up to 5 m.
                let overhang = (0.3 * (prev.back_depth - prev.front_depth)).min(5.0);
                let gap = if section.boxes && i == 1 { 3.4 } else { 2.4 };
                (prev.back_depth - overhang, prev.top + gap)
            }
        };
        let rows = f64::from(tier.rows);
        tiers.push(TierProfile {
            kind: tier.kind,
            rows: tier.rows,
            row_depth: g.row_depth,
            rise,
            per_metre: g.per_metre,
            front_depth,
            front_height,
            back_depth: front_depth + rows * g.row_depth,
            top: front_height + rows * rise,
        });
    }
    let back_depth = tiers.iter().map(|t| t.back_depth).fold(0.0, f64::max);
    let top = tiers.iter().map(|t| t.top).fold(0.0, f64::max);
    SectionProfile {
        back_depth: if tiers.is_empty() {
            0.0
        } else {
            back_depth + 0.5
        },
        top,
        tiers,
    }
}

/// Everything the game reads off a ground.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StadiumFigures {
    pub capacity: u32,
    /// Seated, rail and box seats together.
    pub seats: u32,
    pub rail_seats: u32,
    pub standing: u32,
    pub box_seats: u32,
    pub executive_boxes: u32,
    /// Places with any roof over them (at least 15° inside the roof's front edge).
    pub covered: u32,
    /// Places at least 30° inside the roof's front edge: dry in most weather.
    pub well_covered: u32,
    /// Places behind roof columns.
    pub restricted_view: u32,
    /// 150–190 m from the far corner flag.
    pub distant_view: u32,
    /// More than 190 m from the far corner flag.
    pub poor_view: u32,
    /// 0–100. Placeholder weights; will feed home advantage.
    pub atmosphere: u8,
}

/// Share of a section's places behind its roof's columns.
fn restricted_share(roof: StandRoof) -> f64 {
    match roof {
        StandRoof::None | StandRoof::Cantilever => 0.0,
        StandRoof::Posts => 0.06,
        StandRoof::Goalpost => 0.02,
        StandRoof::Pitched => 0.07,
    }
}

/// Height of a ground-wide roof: clear of the highest stand.
pub(crate) fn ring_roof_height(layout: &StadiumLayout) -> f64 {
    let top = layout
        .sides
        .iter()
        .iter()
        .flat_map(|(_, stand)| stand.bays.iter().map(|b| section_profile(b).top))
        .fold(0.0, f64::max);
    (top + 6.0).max(16.0)
}

/// The roof's front (drip) edge in a section's frame, if it has a roof.
fn roof_edge(
    layout: &StadiumLayout,
    section: &Section,
    profile: &SectionProfile,
) -> Option<(f64, f64)> {
    if layout.roof_system.kind != RoofSystemKind::Stands {
        return Some((-2.5, ring_roof_height(layout)));
    }
    let first = profile.tiers.first()?;
    let (front_offset, back_lift, front_lift) = match section.roof {
        StandRoof::None => return None,
        StandRoof::Cantilever => (0.8, 3.4, 1.6),
        StandRoof::Pitched => (0.3, 2.6, 0.0),
        StandRoof::Posts | StandRoof::Goalpost => (0.3, 2.6, 0.5),
    };
    Some((
        first.front_depth - front_offset,
        profile.top + back_lift + front_lift,
    ))
}

/// FIFA's drip-line guidance: > 30° inside the roof edge is well protected,
/// 15–30° moderately, under 15° barely.
fn cover_class(angle_deg: f64) -> u8 {
    if angle_deg >= 30.0 {
        2
    } else if angle_deg >= 15.0 {
        1
    } else {
        0
    }
}

/// A seat's position in the ground, from its stand side, its position along
/// the side and its depth and height in the stand.
fn seat_position(side: Side, along: f64, depth: f64, height: f64) -> [f64; 3] {
    match side {
        Side::North => [along, height, -(40.0 + depth)],
        Side::South => [along, height, 40.0 + depth],
        Side::West => [-(58.5 + depth), height, along],
        Side::East => [58.5 + depth, height, along],
    }
}

/// FIFA: seats should sit within 150 m (best) and 190 m (at most) of the
/// furthest corner flag.
fn far_corner_distance([x, y, z]: [f64; 3]) -> f64 {
    let mut furthest: f64 = 0.0;
    for cx in [-52.5, 52.5] {
        for cz in [-34.0, 34.0] {
            furthest = furthest.max(((x - cx).powi(2) + y * y + (z - cz).powi(2)).sqrt());
        }
    }
    furthest
}

#[derive(Debug, Default, Clone, Copy)]
struct SectionTally {
    seated: u32,
    rail: u32,
    standing: u32,
    box_seats: u32,
    boxes: u32,
    covered: u32,
    well: u32,
    restricted: u32,
    distant: u32,
    poor: u32,
    single_tier: bool,
    steepness: i32,
}

impl SectionTally {
    fn places(&self) -> u32 {
        self.seated + self.rail + self.standing + self.box_seats
    }
}

fn tally_section(
    layout: &StadiumLayout,
    side: Side,
    index: usize,
    sections: usize,
    section: &Section,
) -> SectionTally {
    let width = side.length_m() / sections as f64;
    let along = -side.length_m() / 2.0 + (index as f64 + 0.5) * width;
    let profile = section_profile(section);
    let edge = roof_edge(layout, section, &profile);
    let ground_roof = layout.roof_system.kind != RoofSystemKind::Stands;
    let mut t = SectionTally {
        single_tier: profile.tiers.len() == 1,
        ..SectionTally::default()
    };
    for tier in &profile.tiers {
        let per_row = (width * tier.per_metre).floor() as u32;
        for r in 0..tier.rows {
            let depth = tier.front_depth + (f64::from(r) + 0.5) * tier.row_depth;
            let height = tier.front_height + f64::from(r + 1) * tier.rise;
            if ground_roof {
                // A ground-wide roof spans the whole bowl, back rows included.
                t.well += per_row;
                t.covered += per_row;
            } else if let Some((edge_depth, edge_height)) = edge.filter(|(d, _)| depth > *d) {
                let angle = (edge_height - height)
                    .atan2(depth - edge_depth)
                    .to_degrees();
                match cover_class(angle) {
                    2 => {
                        t.well += per_row;
                        t.covered += per_row;
                    }
                    1 => t.covered += per_row,
                    _ => {}
                }
            }
            let distance = far_corner_distance(seat_position(side, along, depth, height));
            if distance > 190.0 {
                t.poor += per_row;
            } else if distance > 150.0 {
                t.distant += per_row;
            }
        }
        let places = per_row * u32::from(tier.rows);
        match tier.kind {
            TierKind::Seated => t.seated += places,
            TierKind::Rail => t.rail += places,
            TierKind::Terrace => t.standing += places,
        }
    }
    if section.boxes && profile.tiers.len() >= 2 {
        t.boxes = (width / 4.0).floor() as u32;
        t.box_seats = t.boxes * 12;
        if edge.is_some() {
            t.covered += t.box_seats;
            t.well += t.box_seats;
        }
    }
    let own_roof =
        layout.roof_system.kind == RoofSystemKind::Stands && section.roof != StandRoof::None;
    if own_roof {
        t.restricted = ((t.seated + t.rail + t.standing) as f64 * restricted_share(section.roof))
            .round() as u32;
    }
    t.steepness = match section.rake {
        Rake::Gentle => -1,
        Rake::Standard => 0,
        Rake::Steep => 1,
    };
    t
}

/// The side a corner sits between, and which end of that side.
fn corner_neighbours(corner: Corner) -> [(Side, bool); 2] {
    // `true` = the first section along the side.
    match corner {
        Corner::Nw => [(Side::North, true), (Side::West, true)],
        Corner::Ne => [(Side::North, false), (Side::East, true)],
        Corner::Sw => [(Side::South, true), (Side::West, false)],
        Corner::Se => [(Side::South, false), (Side::East, false)],
    }
}

/// A corner is as deep as the shallower of its two stands, between 6 and 30 m.
pub(crate) fn corner_depth(layout: &StadiumLayout, corner: Corner) -> f64 {
    let depths: Vec<f64> = corner_neighbours(corner)
        .iter()
        .filter_map(|(side, first)| {
            let bays = &layout.sides.get(*side).bays;
            let bay = if *first { bays.first() } else { bays.last() }?;
            Some(section_profile(bay).back_depth).filter(|d| *d > 0.0)
        })
        .collect();
    let depth = if depths.is_empty() {
        10.0
    } else {
        depths.iter().copied().fold(f64::MAX, f64::min)
    };
    depth.clamp(6.0, 30.0)
}

fn corner_places(layout: &StadiumLayout, corner: Corner, fill: CornerFill) -> u32 {
    let density = match fill {
        CornerFill::Open => return 0,
        CornerFill::Seated => 2.3,
        CornerFill::Terrace => 3.4,
    };
    let d = corner_depth(layout, corner);
    let area = if layout.bowl {
        std::f64::consts::FRAC_PI_4 * (d * d - 4.0)
    } else {
        d * d
    };
    (area * density * 0.85).floor() as u32
}

/// Places only, without cover, views or atmosphere: the generator counts
/// thousands of candidate grounds and needs nothing else.
pub(crate) fn capacity(layout: &StadiumLayout) -> u32 {
    let mut total = 0;
    for (side, stand) in layout.sides.iter() {
        let width = side.length_m() / stand.bays.len().max(1) as f64;
        for section in &stand.bays {
            for tier in &section.tiers {
                total +=
                    (width * geometry(tier.kind).per_metre).floor() as u32 * u32::from(tier.rows);
            }
            if section.boxes && section.tiers.len() >= 2 {
                total += (width / 4.0).floor() as u32 * 12;
            }
        }
    }
    total
        + layout
            .corners
            .iter()
            .iter()
            .map(|&(corner, fill)| corner_places(layout, corner, fill))
            .sum::<u32>()
}

/// Everything the game reads off a ground.
pub fn stadium_figures(layout: &StadiumLayout) -> StadiumFigures {
    let mut sections: Vec<SectionTally> = Vec::new();
    for (side, stand) in layout.sides.iter() {
        for (i, section) in stand.bays.iter().enumerate() {
            sections.push(tally_section(layout, side, i, stand.bays.len(), section));
        }
    }
    let ground_roof = layout.roof_system.kind != RoofSystemKind::Stands;
    let mut f = StadiumFigures::default();
    for t in &sections {
        f.rail_seats += t.rail;
        f.standing += t.standing;
        f.box_seats += t.box_seats;
        f.executive_boxes += t.boxes;
        f.seats += t.seated + t.rail + t.box_seats;
        f.covered += t.covered;
        f.well_covered += t.well;
        f.restricted_view += t.restricted;
        f.distant_view += t.distant;
        f.poor_view += t.poor;
    }
    let mut filled_corners = 0;
    for (corner, fill) in layout.corners.iter() {
        let places = corner_places(layout, corner, fill);
        if places > 0 {
            filled_corners += 1;
        }
        match fill {
            CornerFill::Seated => f.seats += places,
            CornerFill::Terrace => f.standing += places,
            CornerFill::Open => {}
        }
        if ground_roof {
            f.covered += places;
            f.well_covered += places;
        }
    }
    f.capacity = f.seats + f.standing;
    f.atmosphere = atmosphere(layout, &f, &sections, filled_corners);
    f
}

/// Placeholder weights, shared with the Stand Builder: big single-tier ends,
/// standing, steep rakes, enclosed corners and a roof raise it; distant seats
/// lower it.
fn atmosphere(
    layout: &StadiumLayout,
    f: &StadiumFigures,
    sections: &[SectionTally],
    filled_corners: u32,
) -> u8 {
    if f.capacity == 0 {
        return 0;
    }
    let capacity = f64::from(f.capacity);
    let mut kop: f64 = 0.0;
    let mut start = 0;
    for (_, stand) in layout.sides.iter() {
        let end = start + stand.bays.len();
        let single: u32 = sections[start..end]
            .iter()
            .filter(|t| t.single_tier)
            .map(SectionTally::places)
            .sum();
        kop = kop.max(f64::from(single));
        start = end;
    }
    let (rake_sum, rake_places) = sections.iter().fold((0.0, 0.0), |(s, n), t| {
        (
            s + f64::from(t.steepness) * f64::from(t.places()),
            n + f64::from(t.places()),
        )
    });
    let rake = if rake_places > 0.0 {
        rake_sum / rake_places
    } else {
        0.0
    };
    let bowl_bonus = if layout.bowl && filled_corners == 4 {
        4.0
    } else {
        0.0
    };
    let closed = layout.roof_system.retractable && layout.roof_system.closed;
    let parts = [
        (kop / 1000.0 * 1.5).min(18.0),
        16.0 * f64::from(f.standing + f.rail_seats) / capacity,
        6.0 * rake,
        f64::from(filled_corners) * 1.5 + bowl_bonus,
        8.0 * (f64::from(f.covered) / capacity).min(1.0) + if closed { 5.0 } else { 0.0 },
        -10.0 * (f64::from(f.poor_view) + 0.4 * f64::from(f.distant_view)) / capacity,
    ];
    (40.0 + parts.iter().sum::<f64>()).round().clamp(0.0, 100.0) as u8
}

#[cfg(test)]
mod tests;
