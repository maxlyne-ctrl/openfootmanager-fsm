//! Wall time per season for a chosen world, scope and unwatched-match engine,
//! with a breakdown of the world's size. The speed gate for a deep pyramid is
//! read off this.
//!
//! Usage: `timing_world <seasons> <gate|shipped> <followed|everything> <full|quick> [NATIONS] [nointl]`
//! where NATIONS is a comma-separated list of nation codes to keep (default: the world's own).

use ofm_core::live_match_manager::UnwatchedMatchEngine;
use season_harness::driver::{RunOptions, SeasonObserver, run_seasons};
use season_harness::worlds::{Scope, WorldSpec};
use std::time::Instant;

struct SeasonClock {
    started: Instant,
}

impl SeasonObserver for SeasonClock {
    fn on_season_end(
        &mut self,
        game: &ofm_core::game::Game,
        _summary: &ofm_core::end_of_season::EndOfSeasonSummary,
        season: u32,
    ) {
        println!(
            "season {season}: {:.2}s  players={}",
            self.started.elapsed().as_secs_f64(),
            game.players.len()
        );
        let international = game
            .competitions
            .iter()
            .filter(|c| c.kind == domain::league::CompetitionType::InternationalNation)
            .count();
        println!(
            "  international competitions={international} national teams={} journal posts={} archived editions={} messages={}",
            game.national_teams.len(),
            game.cash_journal.len(),
            game.edition_archive.len(),
            game.messages.len()
        );
        self.started = Instant::now();
    }
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let seasons: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut spec = match args.get(2).map(String::as_str) {
        Some("shipped") => WorldSpec::shipped(42),
        _ => WorldSpec::gate(42),
    };
    if let Some(nations) = args.get(5) {
        let codes: Vec<&'static str> = nations
            .split(',')
            .map(|code| &*Box::leak(code.to_string().into_boxed_str()))
            .collect();
        spec.nations = Box::leak(codes.into_boxed_slice());
    }
    spec.scope = match args.get(3).map(String::as_str) {
        Some("everything") => Scope::Everything,
        _ => Scope::Followed,
    };
    // `nointl` as a sixth argument opens a career without international football.
    spec.international_football = args.get(6).map(String::as_str) != Some("nointl");
    let built = Instant::now();
    let mut game = spec.build()?;
    game.unwatched_match_engine = match args.get(4).map(String::as_str) {
        Some("quick") => UnwatchedMatchEngine::Quick,
        _ => UnwatchedMatchEngine::Full,
    };
    let active = game
        .active_team_ids()
        .map_or(game.teams.len(), |ids| ids.len());
    println!(
        "world built in {:.2}s: clubs={} active={} players={} competitions={}",
        built.elapsed().as_secs_f64(),
        game.teams.len(),
        active,
        game.players.len(),
        game.competitions.len()
    );
    let mut clock = SeasonClock {
        started: Instant::now(),
    };
    let total = Instant::now();
    run_seasons(&mut game, RunOptions::seasons(seasons), &mut clock)
        .map_err(|error| format!("{error:?}"))?;
    println!(
        "total {seasons} season(s): {:.2}s",
        total.elapsed().as_secs_f64()
    );
    Ok(())
}
