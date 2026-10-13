//! Calibration data for the quick (statistical) match resolver.
//!
//! Plays many fixtures between real generated clubs through the full
//! minute-by-minute engine, with an AI manager on both touchlines exactly as an
//! unwatched fixture is played, and writes what went in and what came out:
//!
//! * `matches.csv` — one row per match: both sides' composite ratings and the
//!   team stats the engine produced.
//! * `players.csv` — one row per player who appeared: attributes and the
//!   individual stats and rating he got.
//! * `subs.csv` — one row per substitution: minute and the positions involved.
//!
//! Usage: `calib_collect <matches> <out_dir> [nations] [full|quick]`
//!
//! `quick` plays the identical pairings through the quick resolver instead, so
//! the two engines can be compared fixture for fixture.

use engine::ai::{self, AiPersonality, AiProfile};
use engine::{EventType, LiveMatchState, MatchConfig, PlayerData, Position, Side, TeamData};
use ofm_core::live_match_manager::match_side;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha12Rng;
use season_harness::worlds::WorldSpec;
use std::collections::HashMap;
use std::fmt::Write as _;

fn pos(p: Position) -> &'static str {
    match p {
        Position::Goalkeeper => "GK",
        Position::Defender => "DEF",
        Position::Midfielder => "MID",
        Position::Forward => "FWD",
    }
}

fn mean(xs: impl Iterator<Item = f64>) -> f64 {
    let (s, n) = xs.fold((0.0, 0usize), |(s, n), x| (s + x, n + 1));
    if n == 0 { 0.0 } else { s / n as f64 }
}

fn features(t: &TeamData) -> String {
    let ovr = mean(t.players.iter().map(|p| p.ovr as f64));
    let cond = mean(t.players.iter().map(|p| p.condition as f64));
    format!(
        "{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:?}",
        t.attack_rating(),
        t.midfield_rating(),
        t.defense_rating(),
        t.goalkeeper_rating(),
        ovr,
        cond,
        t.play_style
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5000);
    let out = args.get(2).cloned().unwrap_or_else(|| "calib".into());
    let nations = args.get(3).cloned().unwrap_or_else(|| "ENG,PT,AR".into());
    let quick = args.get(4).is_some_and(|m| m == "quick");
    let quick_model = engine::QuickModel::default();
    std::fs::create_dir_all(&out).unwrap();

    let list: Vec<&'static str> = nations
        .split(',')
        .map(|s| &*Box::leak(s.to_string().into_boxed_str()))
        .collect();
    let spec = WorldSpec {
        seed: 7,
        nations: Box::leak(list.into_boxed_slice()),
        user_nation: "ENG",
        start_year: 2033,
    };
    let game = spec.build().expect("world");

    // Clubs grouped by league so most pairings are realistic; some cross-league
    // pairings widen the strength gaps the fit sees.
    let leagues: Vec<Vec<String>> = game
        .competitions
        .iter()
        .filter(|c| c.standings.len() >= 4)
        .map(|c| c.standings.iter().map(|s| s.team_id.clone()).collect())
        .collect();
    let all: Vec<String> = game.teams.iter().map(|t| t.id.clone()).collect();
    let rep: HashMap<&str, u32> = game
        .teams
        .iter()
        .map(|t| (t.id.as_str(), t.reputation))
        .collect();
    let sides: HashMap<String, (TeamData, Vec<PlayerData>)> = all
        .iter()
        .map(|id| (id.clone(), match_side(&game, id)))
        .collect();
    eprintln!("{} clubs, {} leagues", all.len(), leagues.len());

    let mut m_csv = String::from(
        "match,h_att,h_mid,h_def,h_gk,h_ovr,h_cond,h_style,a_att,a_mid,a_def,a_gk,a_ovr,a_cond,a_style,\
hg,ag,h_shots,a_shots,h_sot,a_sot,h_blocked,a_blocked,h_fouls,a_fouls,h_yel,a_yel,h_red,a_red,h_pens,a_pens,\
h_corners,a_corners,h_fk,a_fk,h_pc,a_pc,h_pi,a_pi,h_tk,a_tk,h_int,a_int,h_poss,minutes,h_subs,a_subs,injuries\n",
    );
    let mut p_csv = String::from(
        "match,side,id,position,started,ovr,shooting,passing,tackling,defending,dribbling,positioning,vision,pace,aggression,\
minutes,goals,assists,shots,sot,pa,pc,tk,int,fouls,yel,red,rating\n",
    );
    let mut s_csv = String::from("match,side,minute,off_pos,on_pos\n");
    let mut sel = ChaCha12Rng::seed_from_u64(99);
    let config = MatchConfig::default();
    let t0 = std::time::Instant::now();

    for i in 0..n {
        let (h, a) = loop {
            let pool = if sel.random_range(0.0..1.0f64) < 0.75 {
                &leagues[sel.random_range(0..leagues.len())]
            } else {
                &all
            };
            let h = &pool[sel.random_range(0..pool.len())];
            let a = &pool[sel.random_range(0..pool.len())];
            if h != a {
                break (h.clone(), a.clone());
            }
        };
        let (hx, hb) = sides[&h].clone();
        let (ax, ab) = sides[&a].clone();
        let feat = format!("{},{}", features(&hx), features(&ax));
        let roster: HashMap<String, (PlayerData, bool, Side)> = hx
            .players
            .iter()
            .map(|p| (p.id.clone(), (p.clone(), true, Side::Home)))
            .chain(
                hb.iter()
                    .map(|p| (p.id.clone(), (p.clone(), false, Side::Home))),
            )
            .chain(
                ax.players
                    .iter()
                    .map(|p| (p.id.clone(), (p.clone(), true, Side::Away))),
            )
            .chain(
                ab.iter()
                    .map(|p| (p.id.clone(), (p.clone(), false, Side::Away))),
            )
            .collect();
        let prof = |r: u32| AiProfile {
            reputation: r,
            experience: (r / 10).min(100) as u8,
            personality: AiPersonality::Pragmatist,
        };
        let (ph, pa) = (prof(rep[h.as_str()]), prof(rep[a.as_str()]));
        let mut rng = ChaCha12Rng::seed_from_u64(i as u64);
        let rep_ = if quick {
            let fixture = engine::QuickFixture {
                home: &hx,
                away: &ax,
                home_bench: &hb,
                away_bench: &ab,
                allows_extra_time: false,
            };
            engine::simulate_quick(&fixture, &config, &quick_model, &mut rng)
        } else {
            let mut state = LiveMatchState::new(hx, ax, config.clone(), hb, ab, false);
            loop {
                let r = state.step_minute(&mut rng);
                if r.is_finished {
                    break;
                }
                for (side, p) in [(Side::Home, &ph), (Side::Away, &pa)] {
                    for cmd in ai::ai_decide(&state, side, p, &mut rng) {
                        let _ = state.apply_command(cmd);
                    }
                }
            }
            state.into_report()
        };
        let (hs, as_) = (&rep_.home_stats, &rep_.away_stats);
        let mut subs = [0u32; 2];
        let mut inj = 0;
        for e in &rep_.events {
            match e.event_type {
                EventType::Substitution => {
                    let si = if e.side == Side::Home { 0 } else { 1 };
                    subs[si] += 1;
                    let on = e
                        .player_id
                        .as_ref()
                        .and_then(|id| roster.get(id))
                        .map(|r| pos(r.0.position))
                        .unwrap_or("?");
                    let off = e
                        .secondary_player_id
                        .as_ref()
                        .and_then(|id| roster.get(id))
                        .map(|r| pos(r.0.position))
                        .unwrap_or("?");
                    let _ = writeln!(s_csv, "{i},{:?},{},{off},{on}", e.side, e.minute);
                }
                EventType::Injury => inj += 1,
                _ => {}
            }
        }
        let _ = writeln!(
            m_csv,
            "{i},{feat},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.2},{},{},{},{}",
            rep_.home_goals,
            rep_.away_goals,
            hs.shots,
            as_.shots,
            hs.shots_on_target,
            as_.shots_on_target,
            hs.shots_blocked,
            as_.shots_blocked,
            hs.fouls,
            as_.fouls,
            hs.yellow_cards,
            as_.yellow_cards,
            hs.red_cards,
            as_.red_cards,
            hs.penalties,
            as_.penalties,
            hs.corners,
            as_.corners,
            hs.free_kicks,
            as_.free_kicks,
            hs.passes_completed,
            as_.passes_completed,
            hs.passes_intercepted,
            as_.passes_intercepted,
            hs.tackles,
            as_.tackles,
            hs.interceptions,
            as_.interceptions,
            rep_.home_possession,
            rep_.total_minutes,
            subs[0],
            subs[1],
            inj
        );
        for (id, st) in &rep_.player_stats {
            let Some((p, started, side)) = roster.get(id) else {
                continue;
            };
            if st.minutes_played == 0 {
                continue;
            }
            let _ = writeln!(
                p_csv,
                "{i},{:?},{id},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.2}",
                side,
                pos(p.position),
                *started as u8,
                p.ovr,
                p.shooting,
                p.passing,
                p.tackling,
                p.defending,
                p.dribbling,
                p.positioning,
                p.vision,
                p.pace,
                p.aggression,
                st.minutes_played,
                st.goals,
                st.assists,
                st.shots,
                st.shots_on_target,
                st.passes_attempted,
                st.passes_completed,
                st.tackles_won,
                st.interceptions,
                st.fouls_committed,
                st.yellow_cards,
                st.red_cards,
                st.rating
            );
        }
    }
    eprintln!("{n} {} matches in", if quick { "quick" } else { "live" });
    eprintln!("  took {:.2}s", t0.elapsed().as_secs_f64());
    std::fs::write(format!("{out}/matches.csv"), m_csv).unwrap();
    std::fs::write(format!("{out}/players.csv"), p_csv).unwrap();
    std::fs::write(format!("{out}/subs.csv"), s_csv).unwrap();
}
