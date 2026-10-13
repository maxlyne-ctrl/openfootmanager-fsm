//! What a club's ground gives (capacity, cover, view, atmosphere) and a
//! generator that builds a believable ground for a club that has none.
//!
//! The layout itself is data in `domain::stadium`; every rule that turns a
//! layout into numbers lives here, so the game and any viewer agree.

pub mod figures;
pub mod generator;
mod names;

pub use figures::{StadiumFigures, stadium_figures};
pub use generator::{StadiumBrief, generate_stadium_layout};

use domain::team::Team;

use crate::seed::rng_from_u64;
use crate::stable_hash::stable_hash;

/// Seed stream for grounds, kept apart from every other use of a club id.
const GROUND_SEED: u64 = 0x6772_6f75_6e64; // "ground"

/// Give every club without a ground one built to its capacity. Each ground
/// is seeded by the club's id alone, so a club gets the same ground whether
/// it is generated with the world or on loading an old save. Returns whether
/// anything changed, so a load knows to re-save.
pub fn backfill_stadium_layouts(teams: &mut [Team]) -> bool {
    let mut changed = false;
    for team in teams
        .iter_mut()
        .filter(|team| team.stadium_layout.is_none())
    {
        let brief = StadiumBrief {
            target_capacity: team.stadium_capacity,
            reputation: team.reputation,
            founded_year: team.founded_year,
        };
        let mut rng = rng_from_u64(stable_hash(team.id.as_bytes(), GROUND_SEED));
        team.stadium_layout = Some(generate_stadium_layout(&brief, &mut rng));
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn team(id: &str, capacity: u32) -> Team {
        Team::new(
            id.into(),
            id.into(),
            "T".into(),
            "GB".into(),
            "Town".into(),
            "Ground".into(),
            capacity,
        )
    }

    /// Given clubs with no ground, when grounds are backfilled, then each gets
    /// one holding about its listed capacity, and the call reports a change.
    #[test]
    fn clubs_without_a_ground_get_one_of_their_size() {
        let mut teams = vec![team("small", 3_000), team("big", 52_000)];
        assert!(backfill_stadium_layouts(&mut teams));
        for t in &teams {
            let built = stadium_figures(t.stadium_layout.as_ref().expect("a ground")).capacity;
            assert!(
                f64::from(built.abs_diff(t.stadium_capacity))
                    <= f64::from(t.stadium_capacity) * 0.03,
                "{}: {built}",
                t.id
            );
        }
    }

    /// Given a club that already has a ground, when grounds are backfilled,
    /// then it keeps its own and nothing is reported as changed.
    #[test]
    fn a_club_keeps_the_ground_it_has() {
        let mut teams = vec![team("built", 9_000)];
        backfill_stadium_layouts(&mut teams);
        let mut edited = teams[0].stadium_layout.clone().expect("a ground");
        edited.sides.north.name = "The Manager's Stand".into();
        teams[0].stadium_layout = Some(edited.clone());
        assert!(!backfill_stadium_layouts(&mut teams));
        assert_eq!(teams[0].stadium_layout, Some(edited));
    }

    /// Given the same club in two worlds, when grounds are backfilled, then it
    /// gets the same ground both times; a different club gets a different one.
    #[test]
    fn a_clubs_ground_depends_only_on_the_club() {
        let mut first = vec![team("canal-street", 14_000), team("other", 14_000)];
        let mut second = vec![team("canal-street", 14_000)];
        backfill_stadium_layouts(&mut first);
        backfill_stadium_layouts(&mut second);
        assert_eq!(first[0].stadium_layout, second[0].stadium_layout);
        assert_ne!(first[0].stadium_layout, first[1].stadium_layout);
    }
}
