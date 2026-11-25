//! A career opened without international football: the world stays club football only
//! through a World Cup rollover, and every invariant the gate checks still holds.

use domain::league::CompetitionType;
use ofm_core::end_of_season::EndOfSeasonSummary;
use ofm_core::game::Game;
use season_harness::driver::{RunOptions, SeasonObserver, run_seasons};
use season_harness::invariants::Invariants;
use season_harness::worlds::WorldSpec;

/// Two seasons from 2033 put a World Cup summer on the first rollover.
const SEASONS: u32 = 2;

/// The gate's invariants, plus a count of national football seen at each rollover.
struct ClubFootballOnly {
    invariants: Invariants,
    national_football_seen: Vec<(u32, usize, usize)>,
}

impl SeasonObserver for ClubFootballOnly {
    fn on_day_end(&mut self, game: &Game, season: u32, day: u32) {
        self.invariants.on_day_end(game, season, day);
    }

    fn on_before_rollover(&mut self, game: &Game, season: u32) {
        self.invariants.on_before_rollover(game, season);
    }

    fn on_season_end(&mut self, game: &Game, summary: &EndOfSeasonSummary, season: u32) {
        self.invariants.on_season_end(game, summary, season);
        let international_competitions = game
            .competitions
            .iter()
            .filter(|competition| competition.kind == CompetitionType::InternationalNation)
            .count();
        self.national_football_seen.push((
            season,
            game.national_teams.len(),
            international_competitions,
        ));
    }
}

/// Given a generated world whose career opens without international football,
/// When it is played through a World Cup rollover and the season after,
/// Then no national team or international competition comes back, and every
///      invariant still holds.
#[test]
fn a_career_without_international_football_stays_club_football_only() {
    let spec = WorldSpec {
        international_football: false,
        ..WorldSpec::gate(1)
    };
    let mut game = spec.build().expect("the world builds");
    assert!(
        game.national_teams.is_empty(),
        "the career opens without national teams"
    );
    let mut observer = ClubFootballOnly {
        invariants: Invariants::new(&game),
        national_football_seen: Vec::new(),
    };

    run_seasons(&mut game, RunOptions::seasons(SEASONS), &mut observer)
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(
        observer.invariants.is_clean(),
        "an invariant broke:\n{}",
        observer.invariants.report()
    );
    assert_eq!(observer.national_football_seen.len(), SEASONS as usize);
    for (season, national_teams, competitions) in observer.national_football_seen {
        assert_eq!(
            (national_teams, competitions),
            (0, 0),
            "season {season} brought national football back"
        );
    }
}
