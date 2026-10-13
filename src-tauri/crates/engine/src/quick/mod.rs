//! The quick resolver: a whole match from a handful of draws.
//!
//! The minute-by-minute engine plays every minute — zones, passes, duels,
//! touchline decisions — which is what a match the player watches needs, and
//! far more than one nobody watches does. This resolver plays the classic
//! management-game way: the two sides' ratings set how many goals, shots,
//! fouls and corners each can expect; those counts are drawn; each is given a
//! minute and credited to a player who was on the pitch then.
//!
//! What comes out is the same [`MatchReport`] the full engine produces —
//! scorers, assists, cards, substitutions, minutes, team stats and ratings —
//! so every consumer downstream (tables, season stats, morale, stamina, news)
//! treats a quick match exactly like a full one. The numbers it plays by are
//! fitted to the full engine; see [`model`].

pub mod model;
mod sample;
mod side;
mod timeline;
mod workload;

use rand::{Rng, RngExt};

use crate::event::{EventType, MatchEvent};
use crate::report::MatchReport;
use crate::types::{MatchConfig, PlayerData, Side, TeamData, Zone};
pub use model::QuickModel;
use model::{RateModel, RatingReference};
use sample::{categorical, chance, normal, poisson};
use side::{SideRatings, SideState};
use timeline::{SidePlan, Timeline, plan_attempts};

/// The two sides of a fixture, as the full engine would receive them.
#[derive(Debug, Clone, Copy)]
pub struct QuickFixture<'a> {
    pub home: &'a TeamData,
    pub away: &'a TeamData,
    pub home_bench: &'a [PlayerData],
    pub away_bench: &'a [PlayerData],
    /// A knockout tie level at full time goes to extra time and penalties.
    pub allows_extra_time: bool,
}

/// What a side can expect from a match, before any dice are rolled.
#[derive(Debug, Clone, Copy)]
struct Expectation {
    goals: f64,
    on_target: f64,
    shots: f64,
}

/// Resolve a whole match at once.
pub fn simulate_quick<R: Rng>(
    fixture: &QuickFixture<'_>,
    config: &MatchConfig,
    model: &QuickModel,
    rng: &mut R,
) -> MatchReport {
    // As the full engine: a side with nobody on its books cannot play.
    if fixture.home.players.is_empty() || fixture.away.players.is_empty() {
        return MatchReport::from_events(Vec::new(), 0, 0, 0);
    }
    let home = SideState::new(fixture.home, fixture.home_bench);
    let away = SideState::new(fixture.away, fixture.away_bench);
    let expected = [
        expectation(model, &home, &away, true),
        expectation(model, &away, &home, false),
    ];
    let home_share = possession(model, &home, &away, rng);

    let first_half_end = 45 + rng.random_range(0..=config.stoppage_time_max);
    let full_time = first_half_end + 45 + rng.random_range(0..=config.stoppage_time_max);
    let plans = [
        normal_time_plan(model, &home, expected[0], full_time, rng),
        normal_time_plan(model, &away, expected[1], full_time, rng),
    ];

    let mut timeline = Timeline::new(model, home, away);
    timeline.play([&plans[0], &plans[1]], 1, full_time, rng);
    let mut last_minute = full_time;
    let mut shootout = None;
    if fixture.allows_extra_time && timeline.score().0 == timeline.score().1 {
        last_minute = play_extra_time(&mut timeline, model, expected, full_time, rng);
        if timeline.score().0 == timeline.score().1 {
            shootout = Some(play_shootout(&mut timeline, model, last_minute, rng));
        }
    }

    let mut events = std::mem::take(&mut timeline.events);
    events.extend([
        MatchEvent::new(0, EventType::KickOff, Side::Home, Zone::Midfield),
        MatchEvent::new(
            first_half_end,
            EventType::HalfTime,
            Side::Home,
            Zone::Midfield,
        ),
        MatchEvent::new(
            first_half_end + 1,
            EventType::SecondHalfStart,
            Side::Away,
            Zone::Midfield,
        ),
        MatchEvent::new(last_minute, EventType::FullTime, Side::Home, Zone::Midfield),
    ]);
    events.sort_by_key(|event| (event.minute, structural_rank(&event.event_type)));

    let appeared: Vec<String> = timeline
        .home
        .appeared_ids()
        .into_iter()
        .chain(timeline.away.appeared_ids())
        .collect();
    let home_ticks = (home_share * 10.0).round() as u32;
    let mut report = MatchReport::from_events_with_players(
        events,
        home_ticks,
        1000 - home_ticks,
        last_minute,
        appeared,
    );
    workload::add_workload(&mut report, &timeline.home, true, model, rng);
    workload::add_workload(&mut report, &timeline.away, false, model, rng);
    if let Some((home_scored, away_scored)) = shootout {
        report.home_penalties = Some(home_scored);
        report.away_penalties = Some(away_scored);
    }
    report.assign_ratings(&timeline.home.everyone(), &timeline.away.everyone());
    report
}

/// Kick-off and the second-half restart open their minute; half time and
/// full time close theirs.
fn structural_rank(kind: &EventType) -> u8 {
    match kind {
        EventType::KickOff | EventType::SecondHalfStart => 0,
        EventType::HalfTime | EventType::FullTime => 2,
        _ => 1,
    }
}

fn rate(
    model: &RateModel,
    reference: &RatingReference,
    own: &SideState<'_>,
    opp: &SideState<'_>,
    at_home: bool,
) -> f64 {
    let (o, p): (&SideRatings, &SideRatings) = (&own.ratings, &opp.ratings);
    let linear = model.intercept
        + if at_home { model.home } else { 0.0 }
        + model.own_finishing * (o.finishing - reference.finishing)
        + model.own_attack * (o.attack - reference.attack)
        + model.own_midfield * (o.midfield - reference.midfield)
        + model.own_condition * (o.condition - reference.condition)
        + model.opp_keeping * (p.keeping - reference.keeping)
        + model.opp_defence * (p.defence - reference.defence)
        + model.opp_midfield * (p.midfield - reference.midfield)
        + model.opp_condition * (p.condition - reference.condition)
        + model.own_style.of(own.style)
        + model.opp_style.of(opp.style)
        + model.own_tactics.of(&own.tactics)
        + model.opp_tactics.of(&opp.tactics);
    // Clamped so a pathological squad cannot ask for a thousand goals.
    linear.clamp(-4.0, 4.0).exp()
}

fn expectation(
    model: &QuickModel,
    own: &SideState<'_>,
    opp: &SideState<'_>,
    at_home: bool,
) -> Expectation {
    let reference = &model.reference;
    let goals = rate(&model.goals, reference, own, opp, at_home);
    let on_target = rate(&model.shots_on_target, reference, own, opp, at_home).max(goals);
    let shots = rate(&model.shots, reference, own, opp, at_home).max(on_target);
    Expectation {
        goals,
        on_target,
        shots,
    }
}

fn possession<R: Rng>(
    model: &QuickModel,
    home: &SideState<'_>,
    away: &SideState<'_>,
    rng: &mut R,
) -> f64 {
    let p = &model.possession;
    let share = normal(rng, 0.0, p.noise)
        + p.home_intercept
        + p.per_midfield_point * (home.ratings.midfield - away.ratings.midfield)
        + p.home_style.of(home.style)
        + p.away_style.of(away.style)
        + p.home_tactics.of(&home.tactics)
        + p.away_tactics.of(&away.tactics);
    share.clamp(p.min_share, p.max_share)
}

fn normal_time_plan<R: Rng>(
    model: &QuickModel,
    side: &SideState<'_>,
    expected: Expectation,
    full_time: u8,
    rng: &mut R,
) -> SidePlan {
    let mut plan = SidePlan::default();
    plan_attempts(
        &mut plan,
        model,
        expected.goals,
        expected.on_target,
        expected.shots,
        rng,
    );
    let slope = model.events.aggression_slope;
    let expected_fouls: f64 = side.squad[..side.starters]
        .iter()
        .map(|p| {
            let rates = match p.position {
                crate::types::Position::Goalkeeper => &model.positions.goalkeeper,
                crate::types::Position::Defender => &model.positions.defender,
                crate::types::Position::Midfielder => &model.positions.midfielder,
                crate::types::Position::Forward => &model.positions.forward,
            };
            rates.fouls * (slope * (f64::from(p.aggression) - 60.0)).exp()
        })
        .sum();
    plan.fouls = poisson(rng, expected_fouls);
    plan.injuries = poisson(rng, model.events.injuries);
    let count = categorical(rng, &model.events.substitution_counts);
    let (earliest, latest) = model.events.substitution_window;
    plan.substitution_minutes = (0..count)
        .map(|_| {
            // Square root of a uniform leans late, as managers do: the median
            // change comes around the 78th minute.
            let lean = rng.random_range(0.0..1.0f64).sqrt();
            let minute = f64::from(earliest) + f64::from(latest - earliest) * lean;
            (minute.round() as u8).min(full_time)
        })
        .collect();
    plan
}

/// Thirty more minutes at a slightly lower scoring rate. Returns the last
/// minute played.
fn play_extra_time<R: Rng>(
    timeline: &mut Timeline<'_, '_>,
    model: &QuickModel,
    expected: [Expectation; 2],
    full_time: u8,
    rng: &mut R,
) -> u8 {
    let scale = 30.0 / 90.0;
    let factor = model.events.extra_time_goal_factor;
    let plan_for = |e: Expectation, rng: &mut R| SidePlan {
        open_goals: poisson(rng, e.goals * scale * factor),
        saved: poisson(rng, (e.on_target - e.goals).max(0.0) * scale),
        off_target: poisson(rng, (e.shots - e.on_target).max(0.0) * scale),
        fouls: poisson(rng, 10.0 * scale),
        substitution_minutes: if chance(rng, 0.5) {
            vec![full_time + 15]
        } else {
            Vec::new()
        },
        ..SidePlan::default()
    };
    let plans = [plan_for(expected[0], rng), plan_for(expected[1], rng)];
    let end = full_time + 30 + rng.random_range(0..=2u8);
    timeline.play([&plans[0], &plans[1]], full_time + 1, end, rng);
    end
}

/// Takers for a shootout: outfielders on the pitch, best finishers first,
/// the keeper only if nobody else is left.
fn shootout_order(side: &SideState<'_>) -> Vec<String> {
    let mut takers: Vec<&PlayerData> = side
        .on_pitch
        .iter()
        .map(|&i| side.player(i))
        .filter(|p| p.position != crate::types::Position::Goalkeeper)
        .collect();
    if takers.is_empty() {
        takers = side.on_pitch.iter().map(|&i| side.player(i)).collect();
    }
    takers.sort_by_key(|p| std::cmp::Reverse(p.shooting));
    let mut ids: Vec<String> = takers.into_iter().map(|p| p.id.clone()).collect();
    if ids.is_empty() {
        ids.push(String::new());
    }
    ids
}

/// Can the shootout stop after `taken` kicks each (home kicking first)?
fn shootout_decided(home: (u8, u8), away: (u8, u8)) -> bool {
    let ((home_taken, home_scored), (away_taken, away_scored)) = (home, away);
    if home_taken <= 5 && away_taken <= 5 {
        let home_left = 5 - home_taken;
        let away_left = 5 - away_taken;
        home_scored + home_left < away_scored
            || away_scored + away_left < home_scored
            || (home_taken == 5 && away_taken == 5 && home_scored != away_scored)
    } else {
        home_taken == away_taken && home_scored != away_scored
    }
}

/// Five each, then sudden death, stopping as soon as it is decided. Returns
/// each side's successful kicks.
fn play_shootout<R: Rng>(
    timeline: &mut Timeline<'_, '_>,
    model: &QuickModel,
    minute: u8,
    rng: &mut R,
) -> (u8, u8) {
    let home_order = shootout_order(&timeline.home);
    let away_order = shootout_order(&timeline.away);
    let conversion = model.events.shootout_conversion;
    let mut home = (0u8, 0u8);
    let mut away = (0u8, 0u8);
    // A tie this long has never happened; capped so the loop is total, and
    // settled for the home side so the bracket always gets a winner.
    for kick in 0..60usize {
        let side = if kick % 2 == 0 {
            Side::Home
        } else {
            Side::Away
        };
        let (order, tally) = match side {
            Side::Home => (&home_order, &mut home),
            Side::Away => (&away_order, &mut away),
        };
        let taker = &order[(kick / 2) % order.len()];
        let hit = chance(rng, conversion);
        let kind = if hit {
            EventType::ShootoutGoal
        } else {
            EventType::ShootoutMiss
        };
        timeline.events.push(
            MatchEvent::new(minute, kind, side, Zone::attacking_box(side)).with_player(taker),
        );
        tally.0 += 1;
        if hit {
            tally.1 += 1;
        }
        if shootout_decided(home, away) {
            return (home.1, away.1);
        }
    }
    (home.1 + 1, away.1)
}

#[cfg(test)]
mod tests;
