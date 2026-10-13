//! Which engine plays a fixture nobody is watching.
//!
//! A fixture between two AI clubs is resolved by the quick statistical
//! resolver; the player's own club always gets the full minute-by-minute
//! engine, delegated or not. The quick resolver logs no pass-by-pass events,
//! so `PassCompleted` events in a report are the full engine's fingerprint.

use chrono::{TimeZone, Utc};
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::team::Team;
use engine::EventType;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::live_match_manager::{self, UnwatchedMatchEngine};

fn default_attrs(pos: Position) -> PlayerAttributes {
    let group = pos.to_group_position();
    let is_gk = matches!(group, Position::Goalkeeper);
    let is_def = matches!(group, Position::Defender);
    let is_fwd = matches!(group, Position::Forward);
    PlayerAttributes {
        pace: 65,
        stamina: 65,
        strength: 65,
        agility: 65,
        passing: 65,
        shooting: if is_gk { 30 } else { 65 },
        tackling: if is_gk || is_fwd { 35 } else { 65 },
        dribbling: if is_gk { 30 } else { 65 },
        defending: if is_gk {
            30
        } else if is_def {
            75
        } else {
            55
        },
        positioning: 65,
        vision: 65,
        decisions: 65,
        composure: 65,
        aggression: 50,
        teamwork: 65,
        leadership: 50,
        handling: if is_gk { 75 } else { 20 },
        reflexes: if is_gk { 75 } else { 30 },
        aerial: 60,
    }
}

fn make_player(id: &str, name: &str, team_id: &str, pos: Position) -> Player {
    let attrs = default_attrs(pos.clone());
    let mut p = Player::new(
        id.to_string(),
        name.to_string(),
        format!("Full {}", name),
        "1995-01-01".to_string(),
        "GB".to_string(),
        pos,
        attrs,
    );
    p.team_id = Some(team_id.to_string());
    p.morale = 70;
    p.condition = 90;
    p
}

fn make_team(id: &str, name: &str) -> Team {
    Team::new(
        id.to_string(),
        name.to_string(),
        name[..3].to_string(),
        "England".to_string(),
        "London".to_string(),
        "Stadium".to_string(),
        40_000,
    )
}

/// Build a full squad of 22 players for a team (4-4-2 formation ready).
fn make_squad(team_id: &str) -> Vec<Player> {
    let mut players = Vec::new();
    // 2 GK
    for i in 0..2 {
        players.push(make_player(
            &format!("{}_gk{}", team_id, i),
            &format!("GK{}", i),
            team_id,
            Position::Goalkeeper,
        ));
    }
    // 7 DEF
    for i in 0..7 {
        players.push(make_player(
            &format!("{}_def{}", team_id, i),
            &format!("Def{}", i),
            team_id,
            Position::Defender,
        ));
    }
    // 7 MID
    for i in 0..7 {
        players.push(make_player(
            &format!("{}_mid{}", team_id, i),
            &format!("Mid{}", i),
            team_id,
            Position::Midfielder,
        ));
    }
    // 6 FWD
    for i in 0..6 {
        players.push(make_player(
            &format!("{}_fwd{}", team_id, i),
            &format!("Fwd{}", i),
            team_id,
            Position::Forward,
        ));
    }
    players
}

/// Two clubs meeting at home and away, and a third club the manager runs.
fn make_game(manager_club: &str) -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire(manager_club.to_string());
    let teams = vec![
        make_team("home", "Home FC"),
        make_team("away", "Away FC"),
        make_team("mine", "Mine FC"),
    ];
    let mut players = make_squad("home");
    players.extend(make_squad("away"));
    players.extend(make_squad("mine"));
    let fixture = Fixture {
        id: "fix1".to_string(),
        matchday: 1,
        date: "2025-06-15".to_string(),
        home_team_id: "home".to_string(),
        away_team_id: "away".to_string(),
        competition: FixtureCompetition::League,
        status: FixtureStatus::Scheduled,
        result: None,
        ..Default::default()
    };
    let mut game = Game::new(clock, manager, teams, players, vec![], vec![]);
    game.league = Some(League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![fixture],
        standings: ["home", "away", "mine"]
            .iter()
            .map(|id| StandingEntry::new((*id).to_string()))
            .collect(),
        ..Default::default()
    });
    game
}

fn played_in_full(report: &engine::MatchReport) -> bool {
    report
        .events
        .iter()
        .any(|event| event.event_type == EventType::PassCompleted)
}

/// Given a fixture between two AI clubs, when it is played unwatched, then
/// the quick resolver plays it: no pass-by-pass events, but every starter
/// still has minutes and passes in his line of the report.
#[test]
fn a_fixture_between_ai_clubs_is_resolved_quickly() {
    let mut game = make_game("mine");
    let played = live_match_manager::play_unwatched_fixture(&mut game, 0).expect("played");
    assert!(
        !played_in_full(&played.report),
        "an AI fixture should not need the full engine"
    );
    let starters = played
        .report
        .player_stats
        .values()
        .filter(|stats| stats.minutes_played > 0)
        .count();
    assert!(
        starters >= 20,
        "both XIs should have minutes, got {starters}"
    );
    assert!(played.report.home_stats.passes_completed > 0);
}

/// Given a fixture involving the player's own club, when it is played
/// unwatched (delegated), then the full engine still plays it.
#[test]
fn the_players_own_fixture_always_gets_the_full_engine() {
    let mut game = make_game("home");
    let played = live_match_manager::play_unwatched_fixture(&mut game, 0).expect("played");
    assert!(played_in_full(&played.report));
}

/// Given the full engine chosen for every fixture, when an AI fixture is
/// played unwatched, then the full engine plays it.
#[test]
fn choosing_the_full_engine_plays_ai_fixtures_in_full() {
    let mut game = make_game("mine");
    game.unwatched_match_engine = UnwatchedMatchEngine::Full;
    let played = live_match_manager::play_unwatched_fixture(&mut game, 0).expect("played");
    assert!(played_in_full(&played.report));
}

/// Given an AI fixture resolved quickly, when its goals are read, then every
/// scorer is a player of the club the goal went to.
#[test]
fn quick_scorers_belong_to_the_club_that_scored() {
    for _ in 0..5 {
        let mut game = make_game("mine");
        let played = live_match_manager::play_unwatched_fixture(&mut game, 0).expect("played");
        for goal in &played.report.goals {
            let club = match goal.side {
                engine::Side::Home => "home",
                engine::Side::Away => "away",
            };
            assert!(
                goal.scorer_id.starts_with(club),
                "{} scored for {club}",
                goal.scorer_id
            );
        }
    }
}

/// Given an already-played fixture, when the quick path is asked to play it,
/// then it refuses exactly as the full engine does.
#[test]
fn the_quick_path_refuses_a_completed_fixture() {
    let mut game = make_game("mine");
    if let Some(league) = game.league.as_mut() {
        league.fixtures[0].status = FixtureStatus::Completed;
    }
    let error = live_match_manager::play_unwatched_fixture(&mut game, 0)
        .expect_err("a completed fixture cannot be replayed");
    assert_eq!(error, "be.error.liveMatch.fixtureNotScheduled");
}
