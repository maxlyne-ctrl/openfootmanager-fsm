use super::*;
use crate::types::{PlayStyle, Position, TacticsConfig};
use rand::SeedableRng;
use rand::rngs::StdRng;

fn player(id: &str, position: Position, ovr: u8) -> PlayerData {
    PlayerData {
        id: id.into(),
        name: id.into(),
        position,
        ovr,
        condition: 90,
        fitness: 75,
        pace: ovr,
        stamina: ovr,
        strength: ovr,
        agility: ovr,
        passing: ovr,
        shooting: ovr,
        tackling: ovr,
        dribbling: ovr,
        defending: ovr,
        positioning: ovr,
        vision: ovr,
        decisions: ovr,
        composure: ovr,
        aggression: 60,
        teamwork: ovr,
        leadership: ovr,
        handling: ovr,
        reflexes: ovr,
        aerial: ovr,
        traits: vec![],
        role: Default::default(),
    }
}

/// A 4-4-2 of `ovr` players named `<prefix>…`, with a seven-man bench.
fn side(prefix: &str, ovr: u8) -> (TeamData, Vec<PlayerData>) {
    let shape = [
        (Position::Goalkeeper, 1),
        (Position::Defender, 4),
        (Position::Midfielder, 4),
        (Position::Forward, 2),
    ];
    let mut players = Vec::new();
    for (position, count) in shape {
        for n in 0..count {
            players.push(player(&format!("{prefix}-{position:?}-{n}"), position, ovr));
        }
    }
    let bench = [
        Position::Goalkeeper,
        Position::Defender,
        Position::Defender,
        Position::Midfielder,
        Position::Midfielder,
        Position::Forward,
        Position::Forward,
    ]
    .iter()
    .enumerate()
    .map(|(n, &position)| {
        player(
            &format!("{prefix}-bench-{n}"),
            position,
            ovr.saturating_sub(4),
        )
    })
    .collect();
    let team = TeamData {
        id: prefix.into(),
        name: prefix.into(),
        formation: "4-4-2".into(),
        play_style: PlayStyle::Balanced,
        players,
        tactics: TacticsConfig::default(),
    };
    (team, bench)
}

fn play(home_ovr: u8, away_ovr: u8, knockout: bool, seed: u64) -> MatchReport {
    let (home, home_bench) = side("h", home_ovr);
    let (away, away_bench) = side("a", away_ovr);
    let fixture = QuickFixture {
        home: &home,
        away: &away,
        home_bench: &home_bench,
        away_bench: &away_bench,
        allows_extra_time: knockout,
    };
    let mut rng = StdRng::seed_from_u64(seed);
    simulate_quick(
        &fixture,
        &MatchConfig::default(),
        &QuickModel::default(),
        &mut rng,
    )
}

/// Given the same fixture and seed, when it is resolved twice, then both
/// reports agree event for event: a saved career must replay the same way.
#[test]
fn the_same_seed_resolves_the_same_match() {
    let first = play(70, 70, false, 11);
    let second = play(70, 70, false, 11);
    assert_eq!(first.home_goals, second.home_goals);
    assert_eq!(first.away_goals, second.away_goals);
    assert_eq!(first.events.len(), second.events.len());
    let ids = |r: &MatchReport| {
        r.goals
            .iter()
            .map(|g| g.scorer_id.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&first), ids(&second));
}

/// Given many matches, when each report is read, then the scoreline, the goal
/// list and the scorers' own tallies all agree, and every scorer is from the
/// side the goal went to.
#[test]
fn the_scoreline_agrees_with_its_goals_and_scorers() {
    for seed in 0..300 {
        let report = play(72, 68, false, seed);
        let home_goals = report.goals.iter().filter(|g| g.side == Side::Home).count();
        assert_eq!(usize::from(report.home_goals), home_goals);
        assert_eq!(
            usize::from(report.away_goals),
            report.goals.len() - home_goals
        );
        for goal in &report.goals {
            let prefix = if goal.side == Side::Home { "h-" } else { "a-" };
            assert!(
                goal.scorer_id.starts_with(prefix),
                "{} scored for the wrong side",
                goal.scorer_id
            );
            assert_ne!(
                Some(&goal.scorer_id),
                goal.assist_id.as_ref(),
                "nobody assists himself"
            );
        }
        let credited: u32 = report
            .player_stats
            .values()
            .map(|s| u32::from(s.goals))
            .sum();
        assert_eq!(
            credited,
            u32::from(report.home_goals) + u32::from(report.away_goals)
        );
    }
}

/// Given substitutions and dismissals, when goals are credited, then nobody
/// scores after leaving the pitch.
#[test]
fn nobody_scores_after_leaving_the_pitch() {
    for seed in 0..500 {
        let report = play(70, 70, false, seed);
        let mut gone: std::collections::HashMap<String, u8> = std::collections::HashMap::new();
        for event in &report.events {
            match event.event_type {
                EventType::Substitution => {
                    if let Some(off) = &event.secondary_player_id {
                        gone.insert(off.clone(), event.minute);
                    }
                }
                EventType::RedCard | EventType::SecondYellow => {
                    if let Some(id) = &event.player_id {
                        gone.insert(id.clone(), event.minute);
                    }
                }
                EventType::Goal | EventType::ShotSaved | EventType::Foul => {
                    let id = event.player_id.as_ref().expect("credited");
                    assert!(
                        !gone.contains_key(id),
                        "seed {seed}: {id} acted after leaving"
                    );
                }
                _ => {}
            }
        }
    }
}

/// Given a side ten points stronger, when they meet many times, then the
/// stronger side wins clearly more often, at home or away.
#[test]
fn the_stronger_side_wins_more_often() {
    let (mut strong, mut weak) = (0, 0);
    for seed in 0..2000 {
        let home_strong = seed % 2 == 0;
        let report = if home_strong {
            play(76, 66, false, seed)
        } else {
            play(66, 76, false, seed)
        };
        let (strong_goals, weak_goals) = if home_strong {
            (report.home_goals, report.away_goals)
        } else {
            (report.away_goals, report.home_goals)
        };
        if strong_goals > weak_goals {
            strong += 1;
        } else if weak_goals > strong_goals {
            weak += 1;
        }
    }
    assert!(strong > weak * 2, "strong {strong} vs weak {weak}");
}

/// Given equal sides, when they meet many times, then the home side scores
/// more on average — the home advantage the full engine has.
#[test]
fn the_home_side_has_the_advantage() {
    let (mut home, mut away) = (0u32, 0u32);
    for seed in 0..3000 {
        let report = play(70, 70, false, seed);
        home += u32::from(report.home_goals);
        away += u32::from(report.away_goals);
    }
    assert!(home > away, "home {home} away {away}");
}

/// Given a knockout tie, when it is resolved, then it always has a winner —
/// on the scoreline or on penalties — and a league match never goes to
/// penalties.
#[test]
fn a_knockout_tie_always_has_a_winner() {
    let mut shootouts = 0;
    for seed in 0..1500 {
        let report = play(70, 70, true, seed);
        match (report.home_penalties, report.away_penalties) {
            (Some(home), Some(away)) => {
                shootouts += 1;
                assert_eq!(
                    report.home_goals, report.away_goals,
                    "penalties only after a draw"
                );
                assert_ne!(home, away, "seed {seed}: the shootout was not decided");
            }
            (None, None) => assert_ne!(report.home_goals, report.away_goals),
            _ => panic!("one side has a shootout score and the other not"),
        }
        let league = play(70, 70, false, seed);
        assert!(league.home_penalties.is_none() && league.away_penalties.is_none());
    }
    assert!(
        shootouts > 0,
        "a level tie should reach penalties sometimes"
    );
}

/// Given a full match, when the report is built, then starters are rated and
/// the minutes add up: eleven men per side unless someone was sent off.
#[test]
fn starters_play_their_minutes_and_are_rated() {
    let report = play(70, 70, false, 5);
    let starter = report
        .player_stats
        .get("h-Midfielder-0")
        .expect("a starter");
    assert!(starter.minutes_played > 0);
    let full_ninety: Vec<_> = report
        .player_stats
        .iter()
        .filter(|(_, s)| s.minutes_played >= 85)
        .collect();
    assert!(!full_ninety.is_empty());
    assert!(
        full_ninety
            .iter()
            .all(|(_, s)| s.rating >= 1.0 && s.rating <= 10.0)
    );
}

/// Given a side with nobody in it, when the match is resolved, then nothing
/// happens, exactly as the full engine refuses it.
#[test]
fn a_side_with_no_players_cannot_play() {
    let (home, bench) = side("h", 70);
    let empty = TeamData {
        players: vec![],
        ..home.clone()
    };
    let fixture = QuickFixture {
        home: &home,
        away: &empty,
        home_bench: &bench,
        away_bench: &[],
        allows_extra_time: false,
    };
    let mut rng = StdRng::seed_from_u64(1);
    let report = simulate_quick(
        &fixture,
        &MatchConfig::default(),
        &QuickModel::default(),
        &mut rng,
    );
    assert_eq!((report.home_goals, report.away_goals), (0, 0));
    assert!(report.events.is_empty());
}

/// Given the shipped coefficients and reference-quality sides, when many
/// matches are resolved, then the volumes sit where the full engine's do
/// (measured on 30,000 full-engine fixtures: ~2.25 goals, ~13 shots, ~10.7
/// fouls, ~1.3 yellows per side).
#[test]
fn volumes_match_the_full_engine_at_reference_quality() {
    let n = 4000u32;
    let (mut goals, mut shots, mut fouls, mut yellows) = (0u32, 0u32, 0u32, 0u32);
    for seed in 0..u64::from(n) {
        let report = play(70, 70, false, seed);
        for stats in [&report.home_stats, &report.away_stats] {
            goals += u32::from(stats.goals);
            shots += u32::from(stats.shots);
            fouls += u32::from(stats.fouls);
            yellows += u32::from(stats.yellow_cards);
        }
    }
    let per_side = |total: u32| f64::from(total) / f64::from(2 * n);
    assert!(
        (1.6..3.0).contains(&per_side(goals)),
        "goals {}",
        per_side(goals)
    );
    assert!(
        (9.0..18.0).contains(&per_side(shots)),
        "shots {}",
        per_side(shots)
    );
    assert!(
        (7.0..14.0).contains(&per_side(fouls)),
        "fouls {}",
        per_side(fouls)
    );
    assert!(
        (0.7..2.0).contains(&per_side(yellows)),
        "yellows {}",
        per_side(yellows)
    );
}

/// Given a shootout scored 3–3 after five each, it is not decided; at 4–2
/// after four each it is.
#[test]
fn a_shootout_stops_only_when_it_is_decided() {
    assert!(!shootout_decided((5, 3), (5, 3)));
    assert!(shootout_decided((4, 4), (4, 2)));
    assert!(shootout_decided((6, 5), (6, 4)));
    assert!(!shootout_decided((6, 5), (5, 4)));
}
