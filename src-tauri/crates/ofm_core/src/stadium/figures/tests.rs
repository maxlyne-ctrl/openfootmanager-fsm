use super::*;
use domain::stadium::{
    Cladding, Corners, Floodlights, Pitch, RoofSystem, STADIUM_LAYOUT_VERSION, StadiumSides, Stand,
    Tier,
};

fn section(tiers: &[(TierKind, u16)], roof: StandRoof) -> Section {
    Section {
        tiers: tiers
            .iter()
            .map(|&(kind, rows)| Tier { kind, rows })
            .collect(),
        roof,
        boxes: false,
        rake: Rake::Standard,
        balcony: false,
    }
}

fn empty() -> Stand {
    Stand {
        name: "Bank".into(),
        bays: vec![section(&[], StandRoof::None)],
    }
}

/// A ground with only a north stand of one section.
fn north_only(s: Section) -> StadiumLayout {
    StadiumLayout {
        version: STADIUM_LAYOUT_VERSION,
        bowl: false,
        roof_system: RoofSystem::default(),
        floodlights: Floodlights::Pylons,
        cladding: Cladding::Sheet,
        sides: StadiumSides {
            north: Stand {
                name: "Main".into(),
                bays: vec![s],
            },
            south: empty(),
            west: empty(),
            east: empty(),
        },
        corners: Corners::default(),
        pitch: Pitch::default(),
    }
}

/// Given one seated tier of ten rows along the whole 117 m north side, when
/// the ground is counted, then it holds 10 × ⌊117 × 1.84⌋ = 2,150 seats.
#[test]
fn a_seated_tier_holds_a_row_of_seats_per_metre_times_its_rows() {
    let f = stadium_figures(&north_only(section(
        &[(TierKind::Seated, 10)],
        StandRoof::None,
    )));
    assert_eq!(f.capacity, 2150);
    assert_eq!(f.seats, 2150);
    assert_eq!(f.standing, 0);
}

/// Given a terrace and rail seats, when counted, then the terrace is
/// standing and the rail seats count as seats.
#[test]
fn rail_seats_count_as_seats_and_terraces_as_standing() {
    let f = stadium_figures(&north_only(section(
        &[(TierKind::Terrace, 10), (TierKind::Rail, 5)],
        StandRoof::None,
    )));
    assert_eq!(f.standing, 10 * 167);
    assert_eq!(f.rail_seats, 5 * 215);
    assert_eq!(f.seats, f.rail_seats);
}

/// Given no roof, when counted, then nothing is covered; given a cantilever,
/// then most of the stand is covered but the front rows are not well covered.
#[test]
fn a_roof_covers_by_the_angle_from_its_front_edge() {
    let open = stadium_figures(&north_only(section(
        &[(TierKind::Seated, 20)],
        StandRoof::None,
    )));
    assert_eq!((open.covered, open.well_covered), (0, 0));
    let roofed = stadium_figures(&north_only(section(
        &[(TierKind::Seated, 20)],
        StandRoof::Cantilever,
    )));
    assert!(roofed.covered > roofed.capacity / 2, "{roofed:?}");
    assert!(
        roofed.well_covered < roofed.covered,
        "front rows are only partly protected: {roofed:?}"
    );
}

/// Given a roof on posts, when counted, then 6% of the places are behind
/// columns; a cantilever has none.
#[test]
fn roof_columns_restrict_the_view() {
    let posts = stadium_figures(&north_only(section(
        &[(TierKind::Seated, 10)],
        StandRoof::Posts,
    )));
    assert_eq!(posts.restricted_view, (2150.0_f64 * 0.06).round() as u32);
    let cantilever = stadium_figures(&north_only(section(
        &[(TierKind::Seated, 10)],
        StandRoof::Cantilever,
    )));
    assert_eq!(cantilever.restricted_view, 0);
}

/// Given a ground-wide roof, when counted, then every place is well covered.
#[test]
fn a_ground_wide_roof_covers_everyone() {
    let mut layout = north_only(section(&[(TierKind::Seated, 20)], StandRoof::None));
    layout.roof_system.kind = RoofSystemKind::Ring;
    layout.corners.nw = CornerFill::Seated;
    let f = stadium_figures(&layout);
    assert_eq!(f.well_covered, f.capacity);
}

/// Given the same stand built gentle and steep, when counted, then the steep
/// one has the better atmosphere.
#[test]
fn a_steeper_stand_is_louder() {
    let mut gentle = section(&[(TierKind::Terrace, 30)], StandRoof::None);
    gentle.rake = Rake::Gentle;
    let mut steep = gentle.clone();
    steep.rake = Rake::Steep;
    assert!(
        stadium_figures(&north_only(steep)).atmosphere
            > stadium_figures(&north_only(gentle)).atmosphere
    );
}

/// Given a four-tier stand of 40-row tiers, when counted, then its upper rows
/// are flagged as far from the far corner flag.
#[test]
fn very_tall_stands_have_distant_seats() {
    let tall = section(&[(TierKind::Seated, 40); 4], StandRoof::Cantilever);
    let f = stadium_figures(&north_only(tall));
    assert!(f.distant_view + f.poor_view > 0, "{f:?}");
}

/// Given executive boxes between two tiers, when counted, then each 4 m of
/// section is a box of 12 seats.
#[test]
fn boxes_hold_twelve_per_four_metres() {
    let mut s = section(
        &[(TierKind::Seated, 10), (TierKind::Seated, 8)],
        StandRoof::Cantilever,
    );
    s.boxes = true;
    let f = stadium_figures(&north_only(s));
    assert_eq!(f.executive_boxes, 29);
    assert_eq!(f.box_seats, 29 * 12);
}

/// Given an empty ground, when counted, then everything is zero.
#[test]
fn an_empty_ground_holds_nobody() {
    let f = stadium_figures(&north_only(section(&[], StandRoof::None)));
    assert_eq!(f, StadiumFigures::default());
}
