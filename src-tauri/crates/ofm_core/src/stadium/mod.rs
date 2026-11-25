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

use domain::stadium::StadiumLayout;
use domain::team::Team;
use serde::Serialize;
use ts_rs::TS;

use crate::seed::rng_from_u64;
use crate::stable_hash::stable_hash;

/// Seed stream for grounds, kept apart from every other use of a club id.
const GROUND_SEED: u64 = 0x6772_6f75_6e64; // "ground"

/// Give every club without a ground one built to its capacity, then make every club's capacity
/// what its ground holds: the stands are where capacity comes from, so a generated ground (which
/// lands within a few per cent of the figure it was built to) and a ground from an older save
/// both set it. Each ground is seeded by the club's id alone, so a club gets the same ground
/// whether it is generated with the world or on loading an old save. Returns whether anything
/// changed, so a load knows to re-save.
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
    for team in teams.iter_mut() {
        changed |= capacity_follows_ground(team);
    }
    changed
}

/// A club's ground as the ground screen shows it: the layout, and what the game counts from it.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct GroundView {
    pub team_id: String,
    pub stadium_name: String,
    pub layout: StadiumLayout,
    pub figures: StadiumFigures,
}

/// The ground of the club `team_id`, or `None` for an unknown club or one with no ground yet.
pub fn ground_view(teams: &[Team], team_id: &str) -> Option<GroundView> {
    let team = teams.iter().find(|team| team.id == team_id)?;
    let layout = team.stadium_layout.clone()?;
    Some(GroundView {
        team_id: team.id.clone(),
        stadium_name: team.stadium_name.clone(),
        figures: stadium_figures(&layout),
        layout,
    })
}

/// Set a club's capacity to what its ground holds. Returns whether it changed.
fn capacity_follows_ground(team: &mut Team) -> bool {
    let Some(layout) = &team.stadium_layout else {
        return false;
    };
    let held = stadium_figures(layout).capacity;
    let changed = team.stadium_capacity != held;
    team.stadium_capacity = held;
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
        let targets = [("small", 3_000_u32), ("big", 52_000)];
        let mut teams: Vec<Team> = targets
            .iter()
            .map(|(id, target)| team(id, *target))
            .collect();
        assert!(backfill_stadium_layouts(&mut teams));
        for (t, (_, target)) in teams.iter().zip(targets) {
            let built = stadium_figures(t.stadium_layout.as_ref().expect("a ground")).capacity;
            assert!(
                f64::from(built.abs_diff(target)) <= f64::from(target) * 0.03,
                "{}: {built} for {target}",
                t.id
            );
        }
    }

    /// Given clubs with no ground, when grounds are backfilled, then each club's capacity is
    /// what its new ground holds: the stands are where capacity comes from.
    #[test]
    fn a_clubs_capacity_is_what_its_ground_holds() {
        let mut teams = vec![team("small", 3_000), team("big", 52_000)];

        backfill_stadium_layouts(&mut teams);

        for t in &teams {
            let built = stadium_figures(t.stadium_layout.as_ref().expect("a ground")).capacity;
            assert_eq!(t.stadium_capacity, built, "{}", t.id);
        }
    }

    /// Given a club whose ground was saved before capacity came from the stands, so the two
    /// disagree, when grounds are backfilled, then the capacity is put right and a change is
    /// reported, so the load saves it.
    #[test]
    fn a_capacity_that_disagrees_with_its_ground_is_put_right() {
        let mut teams = vec![team("old", 9_000)];
        backfill_stadium_layouts(&mut teams);
        let built = teams[0].stadium_capacity;
        teams[0].stadium_capacity = built + 250;

        assert!(backfill_stadium_layouts(&mut teams));
        assert_eq!(teams[0].stadium_capacity, built);
    }

    /// Given a game whose clubs have grounds, when one club's ground is asked for, then the view
    /// carries its name, its layout and the figures the game counts from it.
    #[test]
    fn a_clubs_ground_view_is_its_layout_and_what_it_holds() {
        let mut teams = vec![team("home", 12_000), team("away", 30_000)];
        backfill_stadium_layouts(&mut teams);

        let view = ground_view(&teams, "home").expect("a ground");

        let layout = teams[0].stadium_layout.clone().expect("a ground");
        assert_eq!(view.team_id, "home");
        assert_eq!(view.stadium_name, "Ground");
        assert_eq!(view.figures, stadium_figures(&layout));
        assert_eq!(view.figures.capacity, teams[0].stadium_capacity);
        assert_eq!(view.layout, layout);
    }

    /// Given a club that does not exist, or one with no ground yet, when its ground is asked
    /// for, then there is none to show.
    #[test]
    fn there_is_no_ground_view_without_a_club_and_a_ground() {
        let teams = vec![team("unbuilt", 5_000)];

        assert!(ground_view(&teams, "nobody").is_none());
        assert!(ground_view(&teams, "unbuilt").is_none());
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
