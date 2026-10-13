use super::*;
use crate::seed::rng_from_u64;
use crate::stadium::figures::{capacity, stadium_figures};

fn brief(target_capacity: u32, reputation: u32, founded_year: u32) -> StadiumBrief {
    StadiumBrief {
        target_capacity,
        reputation,
        founded_year,
    }
}

fn all_tiers(layout: &StadiumLayout) -> Vec<Tier> {
    layout
        .sides
        .iter()
        .into_iter()
        .flat_map(|(_, stand)| stand.bays.iter().flat_map(|bay| bay.tiers.clone()))
        .collect()
}

/// Given the same brief and seed, when a ground is generated twice, then the
/// two are identical — a club's ground must not change between loads.
#[test]
fn the_same_seed_builds_the_same_ground() {
    let b = brief(18_000, 600, 1890);
    assert_eq!(
        generate_stadium_layout(&b, &mut rng_from_u64(7)),
        generate_stadium_layout(&b, &mut rng_from_u64(7))
    );
}

/// Given different seeds, when grounds of the same size are generated, then
/// they are not all alike.
#[test]
fn different_seeds_build_different_grounds() {
    let b = brief(18_000, 600, 1890);
    let first = generate_stadium_layout(&b, &mut rng_from_u64(0));
    assert!((1..20).any(|seed| generate_stadium_layout(&b, &mut rng_from_u64(seed)) != first));
}

/// Given targets from non-league to the biggest bowl, when grounds are
/// generated over many seeds, then each holds within 3% of its target — or,
/// at a small ground where one row of one section is already more than 3%,
/// within 60 places.
#[test]
fn a_generated_ground_holds_its_target_capacity() {
    for target in [
        800, 1_900, 4_500, 9_000, 15_000, 28_000, 42_000, 61_000, 88_000,
    ] {
        for seed in 0..25 {
            let layout =
                generate_stadium_layout(&brief(target, 500, 1905), &mut rng_from_u64(seed));
            let capacity = stadium_figures(&layout).capacity;
            let allowed = (f64::from(target) * 0.03).max(60.0);
            assert!(
                f64::from(capacity.abs_diff(target)) <= allowed,
                "target {target}, seed {seed}: built {capacity}"
            );
        }
    }
}

/// Given a big club or a famous one, when its ground is generated, then it has
/// no terraces — rail seats at most.
#[test]
fn big_and_famous_clubs_are_all_seater() {
    for (target, reputation) in [(25_000, 400), (8_000, 850)] {
        for seed in 0..25 {
            let layout =
                generate_stadium_layout(&brief(target, reputation, 1900), &mut rng_from_u64(seed));
            assert!(
                all_tiers(&layout)
                    .iter()
                    .all(|t| t.kind != TierKind::Terrace),
                "target {target}, rep {reputation}, seed {seed}"
            );
            assert_eq!(stadium_figures(&layout).standing, 0);
        }
    }
}

/// Given any target, when a ground is generated, then no tier is longer than
/// the row limit and no section has more than four tiers.
#[test]
fn generated_stands_stay_within_the_builder_limits() {
    for target in [500, 6_000, 30_000, 99_000] {
        for seed in 0..15 {
            let layout =
                generate_stadium_layout(&brief(target, 500, 1950), &mut rng_from_u64(seed));
            for (_, stand) in layout.sides.iter() {
                assert!((1..=4).contains(&stand.bays.len()));
                for bay in &stand.bays {
                    assert!(bay.tiers.len() <= 4);
                    for t in &bay.tiers {
                        let max = if t.kind == TierKind::Terrace { 60 } else { 40 };
                        assert!((3..=max).contains(&t.rows), "{t:?}");
                    }
                }
            }
        }
    }
}

/// Given a tiny target, when a ground is generated, then it is still a valid
/// ground with a main stand and some capacity.
#[test]
fn a_tiny_club_still_gets_a_ground() {
    let layout = generate_stadium_layout(&brief(0, 50, 2010), &mut rng_from_u64(3));
    assert!(stadium_figures(&layout).capacity > 0);
    assert!(layout.sides.north.bays.iter().any(|b| !b.tiers.is_empty()));
}

/// Given a ground, when its stands are named, then the two ends never share a name.
#[test]
fn the_two_ends_have_different_names() {
    for seed in 0..50 {
        let layout = generate_stadium_layout(&brief(10_000, 500, 1900), &mut rng_from_u64(seed));
        assert_ne!(layout.sides.west.name, layout.sides.east.name);
    }
}

/// Given any ground, when its capacity is counted the quick way the generator
/// uses, then it matches the full count.
#[test]
fn the_quick_capacity_count_matches_the_full_one() {
    for seed in 0..40_u64 {
        let target = 500 + (seed as u32 * 2_971) % 80_000;
        let layout = generate_stadium_layout(&brief(target, 500, 1900), &mut rng_from_u64(seed));
        assert_eq!(
            capacity(&layout),
            stadium_figures(&layout).capacity,
            "seed {seed}"
        );
    }
}
