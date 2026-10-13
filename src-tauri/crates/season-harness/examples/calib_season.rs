//! In-season calibration data: what each engine actually produces in a career.
//!
//! `calib_collect` pairs clubs at random on day one. This plays real seasons
//! through the game's own day loop and records, for every league fixture in
//! the active scope, both sides as they stood the evening before (line
//! ratings, condition, style, reputation) and the result — so the quick
//! resolver can be checked against the full engine in the conditions a
//! career actually has: within-league pairings, worn squads, rotations.
//!
//! Usage: `calib_season <seasons> <out.csv> [full|quick]`

use domain::league::FixtureStatus;
use engine::{Position, TeamData};
use ofm_core::game::Game;
use ofm_core::live_match_manager::{UnwatchedMatchEngine, match_side};
use season_harness::driver::{RunOptions, SeasonObserver, run_seasons};
use season_harness::worlds::WorldSpec;
use std::collections::HashMap;
use std::fmt::Write as _;

fn line(team: &TeamData, position: Position) -> f64 {
    let v: Vec<f64> = team
        .players
        .iter()
        .filter(|p| p.position == position)
        .map(|p| f64::from(p.ovr))
        .collect();
    if v.is_empty() {
        f64::NAN
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

fn finishing(team: &TeamData) -> f64 {
    let v: Vec<f64> = team
        .players
        .iter()
        .filter(|p| p.position == Position::Forward)
        .map(|p| (f64::from(p.shooting) + f64::from(p.composure) + f64::from(p.decisions)) / 3.0)
        .collect();
    if v.is_empty() {
        f64::NAN
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

fn keeping(team: &TeamData) -> f64 {
    team.players
        .iter()
        .find(|p| p.position == Position::Goalkeeper)
        .map_or(f64::NAN, |p| {
            (f64::from(p.handling) + f64::from(p.reflexes) + f64::from(p.positioning)) / 3.0
        })
}

fn describe(game: &Game, team_id: &str) -> String {
    let (team, _) = match_side(game, team_id);
    let cond = team
        .players
        .iter()
        .map(|p| f64::from(p.condition))
        .sum::<f64>()
        / team.players.len().max(1) as f64;
    let rep = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .map_or(0, |t| t.reputation);
    format!(
        "{:.2},{:.2},{:.2},{:.2},{:.2},{:?},{},{:.2},{:.2},{:.2},{:.2},{},{},{},{:?},{:.2},{:.2}",
        line(&team, Position::Forward),
        line(&team, Position::Midfielder),
        line(&team, Position::Defender),
        line(&team, Position::Goalkeeper),
        cond,
        team.play_style,
        rep,
        team.attack_rating(),
        team.midfield_rating(),
        team.defense_rating(),
        team.goalkeeper_rating(),
        team.count_position(Position::Forward),
        team.count_position(Position::Midfielder),
        team.count_position(Position::Defender),
        format!(
            "{:?}|{:?}|{:?}|{:?}",
            team.tactics.pressing_intensity,
            team.tactics.defensive_line,
            team.tactics.tempo,
            team.tactics.width
        )
        .replace(',', ";"),
        finishing(&team),
        keeping(&team)
    )
}

struct Recorder {
    pending: HashMap<String, (String, String)>,
    csv: String,
    users_club: Option<String>,
}

impl SeasonObserver for Recorder {
    fn on_day_end(&mut self, game: &Game, season: u32, _day: u32) {
        let today = game.clock.current_date.format("%Y-%m-%d").to_string();
        let mut done = Vec::new();
        for competition in &game.competitions {
            if !game.competition_in_active_scope(competition) || competition.standings.len() < 4 {
                continue;
            }
            for fixture in &competition.fixtures {
                if let Some((league, features)) = self.pending.get(&fixture.id) {
                    if fixture.status == FixtureStatus::Completed {
                        if let Some(result) = &fixture.result {
                            let stats = result.report.as_ref().map_or_else(
                                || ",,,,,,,,,".to_string(),
                                |r| {
                                    let (h, a) = (&r.home_stats, &r.away_stats);
                                    format!(
                                        "{},{},{},{},{},{},{},{},{}",
                                        h.shots,
                                        a.shots,
                                        h.shots_on_target,
                                        a.shots_on_target,
                                        h.fouls,
                                        a.fouls,
                                        h.yellow_cards,
                                        a.yellow_cards,
                                        h.possession_pct
                                    )
                                },
                            );
                            let _ = writeln!(
                                self.csv,
                                "{season},{league},{features},{},{},{stats}",
                                result.home_goals, result.away_goals
                            );
                        }
                        done.push(fixture.id.clone());
                    }
                    continue;
                }
                let involves_user = self
                    .users_club
                    .as_deref()
                    .is_some_and(|c| c == fixture.home_team_id || c == fixture.away_team_id);
                if fixture.date == today
                    && fixture.status == FixtureStatus::Scheduled
                    && !involves_user
                {
                    let features = format!(
                        "{},{},{},{}",
                        fixture.home_team_id,
                        fixture.away_team_id,
                        describe(game, &fixture.home_team_id),
                        describe(game, &fixture.away_team_id)
                    );
                    self.pending.insert(
                        fixture.id.clone(),
                        (competition.name.replace(',', " "), features),
                    );
                }
            }
        }
        for id in done {
            self.pending.remove(&id);
        }
    }
}

fn main() {
    let seasons: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    let out = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "season.csv".into());
    let full = std::env::args().nth(3).as_deref() == Some("full");
    let seed: u64 = std::env::args()
        .nth(4)
        .and_then(|s| s.parse().ok())
        .unwrap_or(42);
    let mut game = WorldSpec::gate(seed).build().expect("world");
    if full {
        game.unwatched_match_engine = UnwatchedMatchEngine::Full;
    }
    let mut recorder = Recorder {
        pending: HashMap::new(),
        csv: String::from(
            "season,league,h_id,a_id,h_fw,h_md,h_df,h_gk,h_cond,h_style,h_rep,h_att,h_mid,h_def,h_gkr,h_nfw,h_nmd,h_ndf,h_tac,h_fin,h_keep,a_fw,a_md,a_df,a_gk,a_cond,a_style,a_rep,a_att,a_mid,a_def,a_gkr,a_nfw,a_nmd,a_ndf,a_tac,a_fin,a_keep,hg,ag,h_shots,a_shots,h_sot,a_sot,h_fouls,a_fouls,h_yel,a_yel,h_poss\n",
        ),
        users_club: game.manager.team_id.clone(),
    };
    run_seasons(&mut game, RunOptions::seasons(seasons), &mut recorder).expect("seasons");
    std::fs::write(&out, recorder.csv).expect("write");
}
