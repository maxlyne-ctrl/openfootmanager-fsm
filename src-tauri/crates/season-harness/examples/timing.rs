//! How long a season takes, and what the leagues look like at the end of it.
//!
//! Plays the season harness's three-nation world for N seasons and prints the
//! wall time per season, the player count, and each league's goals per match,
//! draw rate and title/bottom points.
//!
//! Usage: `timing <seasons> [quick|full]` — `full` plays AI fixtures with the
//! minute-by-minute engine instead of the quick resolver, for comparison.

use season_harness::driver::{RunOptions, SeasonObserver, run_seasons};
use season_harness::worlds::WorldSpec;
use std::time::Instant;
struct T {
    t: Instant,
}
impl SeasonObserver for T {
    fn on_before_rollover(&mut self, g: &ofm_core::game::Game, _season: u32) {
        for c in &g.competitions {
            let p: u32 = c.standings.iter().map(|s| s.played).sum();
            if p == 0 {
                continue;
            }
            let gf: u32 = c.standings.iter().map(|s| s.goals_for).sum();
            let draws = c.standings.iter().map(|s| s.drawn).sum::<u32>() as f64 / 2.0;
            let wins: u32 = c.standings.iter().map(|s| s.won).sum();
            let top = c.standings.iter().map(|s| s.points).max().unwrap_or(0);
            let bottom = c.standings.iter().map(|s| s.points).min().unwrap_or(0);
            println!(
                "   {:<32} goals/match {:.2}  draw% {:.1}  champion {top} pts, bottom {bottom} pts",
                c.name,
                2.0 * gf as f64 / p as f64,
                100.0 * draws / (draws + wins as f64)
            );
        }
    }
    fn on_season_end(
        &mut self,
        g: &ofm_core::game::Game,
        _s: &ofm_core::end_of_season::EndOfSeasonSummary,
        season: u32,
    ) {
        println!(
            "season {season}: {:.2}s  players={}",
            self.t.elapsed().as_secs_f64(),
            g.players.len()
        );
        self.t = Instant::now();
    }
}
fn main() {
    let n: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    let t0 = Instant::now();
    let mut game = WorldSpec::gate(42).build().unwrap();
    if std::env::args().nth(2).as_deref() == Some("full") {
        game.unwatched_match_engine = ofm_core::live_match_manager::UnwatchedMatchEngine::Full;
    }
    println!(
        "world built in {:.2}s: teams={} players={} competitions={}",
        t0.elapsed().as_secs_f64(),
        game.teams.len(),
        game.players.len(),
        game.competitions.len()
    );
    for c in &game.competitions {
        println!("  {} ({} teams)", c.name, c.standings.len());
    }
    let mut o = T { t: Instant::now() };
    let t1 = Instant::now();
    run_seasons(&mut game, RunOptions::seasons(n), &mut o).unwrap();
    println!("total {n} seasons: {:.2}s", t1.elapsed().as_secs_f64());
}
