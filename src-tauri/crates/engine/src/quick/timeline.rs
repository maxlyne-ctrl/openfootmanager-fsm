//! Turning a match's counts into a timeline of events.
//!
//! The counts (how many goals, shots, fouls…) are decided up front from the
//! sides' ratings. Each is then given a minute, the list is sorted, and the
//! match is walked once in order so every event is credited to someone who
//! was actually on the pitch at that minute — a substituted or dismissed
//! player scores nothing afterwards.

use rand::{Rng, RngExt};

use super::model::{PositionRates, QuickModel};
use super::sample::{binomial, chance, poisson};
use super::side::SideState;
use crate::event::{EventDetail, EventType, GoalContext, MatchEvent};
use crate::types::{PlayerData, Position, Side, Zone};

/// The per-side counts the timeline is built from.
#[derive(Debug, Clone, Default)]
pub(super) struct SidePlan {
    pub open_goals: u32,
    pub penalties_scored: u32,
    pub penalties_missed: u32,
    pub saved: u32,
    pub off_target: u32,
    pub blocked: u32,
    pub fouls: u32,
    pub corners: u32,
    pub injuries: u32,
    pub substitution_minutes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Happening {
    // Order within a minute: changes of personnel first, so a player who
    // comes on in the 70th can score in the 70th.
    Substitution,
    Foul,
    Injury,
    Corner,
    ShotOffTarget,
    ShotBlocked,
    ShotSaved,
    PenaltyMiss,
    PenaltyGoal,
    Goal,
}

#[derive(Debug, Clone, Copy)]
struct Slot {
    minute: u8,
    side: Side,
    happening: Happening,
}

/// Walks the sorted slots, crediting each to a player on the pitch.
pub(super) struct Timeline<'m, 'a> {
    model: &'m QuickModel,
    pub home: SideState<'a>,
    pub away: SideState<'a>,
    pub events: Vec<MatchEvent>,
    home_score: u8,
    away_score: u8,
}

impl<'m, 'a> Timeline<'m, 'a> {
    pub(super) fn new(model: &'m QuickModel, home: SideState<'a>, away: SideState<'a>) -> Self {
        Self {
            model,
            home,
            away,
            events: Vec::with_capacity(160),
            home_score: 0,
            away_score: 0,
        }
    }

    pub(super) fn score(&self) -> (u8, u8) {
        (self.home_score, self.away_score)
    }

    /// Play `plans` out between `first` and `last` minutes inclusive.
    pub(super) fn play<R: Rng>(&mut self, plans: [&SidePlan; 2], first: u8, last: u8, rng: &mut R) {
        let mut slots = Vec::with_capacity(96);
        for (side, plan) in [(Side::Home, plans[0]), (Side::Away, plans[1])] {
            schedule(&mut slots, side, plan, first, last, rng);
        }
        slots.sort_by_key(|slot| (slot.minute, slot.happening));
        for slot in slots {
            self.resolve(slot, rng);
        }
    }

    fn side(&self, side: Side) -> &SideState<'a> {
        match side {
            Side::Home => &self.home,
            Side::Away => &self.away,
        }
    }

    fn side_mut(&mut self, side: Side) -> &mut SideState<'a> {
        match side {
            Side::Home => &mut self.home,
            Side::Away => &mut self.away,
        }
    }

    fn rates(&self, position: Position) -> &'m PositionRates {
        let table = &self.model.positions;
        match position {
            Position::Goalkeeper => &table.goalkeeper,
            Position::Defender => &table.defender,
            Position::Midfielder => &table.midfielder,
            Position::Forward => &table.forward,
        }
    }

    /// An on-pitch player by `weight`, or any outfielder on the pitch if the
    /// weights rule everyone out (a side with no forwards still has a shooter).
    fn pick_or_any<R: Rng>(
        &self,
        side: Side,
        exclude: Option<usize>,
        rng: &mut R,
        weight: impl Fn(&PlayerData) -> f64,
    ) -> Option<usize> {
        let state = self.side(side);
        state.pick(rng, exclude, weight).or_else(|| {
            state.pick(rng, exclude, |p| {
                if p.position == Position::Goalkeeper {
                    0.0
                } else {
                    1.0
                }
            })
        })
    }

    fn id(&self, side: Side, index: usize) -> String {
        self.side(side).player(index).id.clone()
    }

    fn push(&mut self, event: MatchEvent) {
        self.events.push(event);
    }

    fn resolve<R: Rng>(&mut self, slot: Slot, rng: &mut R) {
        let Slot {
            minute,
            side,
            happening,
        } = slot;
        match happening {
            Happening::Substitution => self.substitution(minute, side, rng),
            Happening::Foul => self.foul(minute, side, rng),
            Happening::Injury => self.injury(minute, side, rng),
            Happening::Corner => self.corner(minute, side),
            Happening::ShotOffTarget => self.shot(minute, side, EventType::ShotOffTarget, rng),
            Happening::ShotBlocked => self.shot(minute, side, EventType::ShotBlocked, rng),
            Happening::ShotSaved => self.shot(minute, side, EventType::ShotSaved, rng),
            Happening::PenaltyMiss => self.penalty(minute, side, false),
            Happening::PenaltyGoal => self.penalty(minute, side, true),
            Happening::Goal => self.goal(minute, side, rng),
        }
    }

    fn substitution<R: Rng>(&mut self, minute: u8, side: Side, rng: &mut R) {
        if let Some((on, off)) = self.side_mut(side).substitute(rng) {
            let event = MatchEvent::new(minute, EventType::Substitution, side, Zone::Midfield)
                .with_player(&self.id(side, on))
                .with_secondary(&self.id(side, off));
            self.push(event);
        }
    }

    fn shooting_weight(&self, player: &PlayerData, base: f64) -> f64 {
        let slope = self.model.events.shooting_skill_slope;
        base * (slope * (super::side::finishing(player) - 70.0)).exp()
    }

    fn shot<R: Rng>(&mut self, minute: u8, side: Side, kind: EventType, rng: &mut R) {
        let Some(shooter) = self.pick_or_any(side, None, rng, |p| {
            self.shooting_weight(p, self.rates(p.position).shot_weight)
        }) else {
            return;
        };
        let event = MatchEvent::new(minute, kind, side, Zone::attacking_box(side))
            .with_player(&self.id(side, shooter));
        self.push(event);
    }

    fn goal_context(&self, side: Side) -> GoalContext {
        let (own, opp) = match side {
            Side::Home => (self.home_score, self.away_score),
            Side::Away => (self.away_score, self.home_score),
        };
        if own == 0 && opp == 0 {
            GoalContext::Opener
        } else if own + 1 == opp {
            GoalContext::Equaliser
        } else if own + 1 > opp {
            GoalContext::Extends
        } else {
            GoalContext::Consolation
        }
    }

    fn add_goal(&mut self, side: Side) {
        match side {
            Side::Home => self.home_score += 1,
            Side::Away => self.away_score += 1,
        }
    }

    fn goal<R: Rng>(&mut self, minute: u8, side: Side, rng: &mut R) {
        let Some(scorer) = self.pick_or_any(side, None, rng, |p| {
            self.shooting_weight(p, self.rates(p.position).goal_weight)
        }) else {
            return;
        };
        let events = &self.model.events;
        let (corner_share, free_kick_share) =
            (events.corner_goal_share, events.free_kick_goal_share);
        let roll = rng.random_range(0.0..1.0f64);
        if roll < corner_share {
            self.push(MatchEvent::new(
                minute,
                EventType::Corner,
                side,
                Zone::attacking_box(side),
            ));
        } else if roll < corner_share + free_kick_share {
            // A free kick in the attacking third is what marks the goal as a
            // set piece in the report.
            self.push(MatchEvent::new(
                minute,
                EventType::FreeKick,
                side,
                Zone::attacking_third(side),
            ));
        }
        let assister = chance(rng, self.model.events.assisted_share)
            .then(|| {
                self.side(side).pick(rng, Some(scorer), |p| {
                    let vision = (0.01 * (f64::from(p.vision) - 70.0)).exp();
                    self.rates(p.position).assist_weight * vision
                })
            })
            .flatten();
        let mut event = MatchEvent::new(minute, EventType::Goal, side, Zone::attacking_box(side))
            .with_player(&self.id(side, scorer))
            .with_detail(EventDetail::Goal {
                context: self.goal_context(side),
            });
        if let Some(assister) = assister {
            event = event.with_secondary(&self.id(side, assister));
        }
        self.push(event);
        self.add_goal(side);
    }

    fn penalty(&mut self, minute: u8, side: Side, scored: bool) {
        // The taker is the best finisher on the pitch, as a manager would choose.
        let state = self.side(side);
        let taker = state
            .on_pitch
            .iter()
            .copied()
            .filter(|&i| state.player(i).position != Position::Goalkeeper)
            .max_by_key(|&i| state.player(i).shooting);
        let Some(taker) = taker else { return };
        self.push(MatchEvent::new(
            minute,
            EventType::PenaltyAwarded,
            side,
            Zone::attacking_box(side),
        ));
        let kind = if scored {
            EventType::PenaltyGoal
        } else {
            EventType::PenaltyMiss
        };
        let mut event = MatchEvent::new(minute, kind, side, Zone::attacking_box(side))
            .with_player(&self.id(side, taker));
        if scored {
            event = event.with_detail(EventDetail::Goal {
                context: self.goal_context(side),
            });
        }
        self.push(event);
        if scored {
            self.add_goal(side);
        }
    }

    fn foul<R: Rng>(&mut self, minute: u8, side: Side, rng: &mut R) {
        let slope = self.model.events.aggression_slope;
        let Some(fouler) = self.pick_or_any(side, None, rng, |p| {
            self.rates(p.position).fouls * (slope * (f64::from(p.aggression) - 60.0)).exp()
        }) else {
            return;
        };
        let fouler_id = self.id(side, fouler);
        self.push(
            MatchEvent::new(minute, EventType::Foul, side, Zone::Midfield).with_player(&fouler_id),
        );
        if chance(rng, self.model.events.free_kicks_per_foul) {
            self.push(MatchEvent::new(
                minute,
                EventType::FreeKick,
                side.opposite(),
                Zone::Midfield,
            ));
        }
        let rates = &self.model.events;
        let (yellow, red) = (rates.yellow_per_foul, rates.straight_red_per_foul);
        if chance(rng, red) {
            self.push(
                MatchEvent::new(minute, EventType::RedCard, side, Zone::Midfield)
                    .with_player(&fouler_id),
            );
            self.side_mut(side).dismiss(fouler);
        } else if chance(rng, yellow) {
            if self.side(side).booked.contains(&fouler) {
                self.push(
                    MatchEvent::new(minute, EventType::SecondYellow, side, Zone::Midfield)
                        .with_player(&fouler_id),
                );
                self.side_mut(side).dismiss(fouler);
            } else {
                self.push(
                    MatchEvent::new(minute, EventType::YellowCard, side, Zone::Midfield)
                        .with_player(&fouler_id),
                );
                self.side_mut(side).booked.insert(fouler);
            }
        }
    }

    fn injury<R: Rng>(&mut self, minute: u8, side: Side, rng: &mut R) {
        if let Some(hurt) = self.pick_or_any(side, None, rng, |p| {
            if p.position == Position::Goalkeeper {
                0.2
            } else {
                1.0
            }
        }) {
            let event = MatchEvent::new(minute, EventType::Injury, side, Zone::Midfield)
                .with_player(&self.id(side, hurt));
            self.push(event);
        }
    }

    fn corner(&mut self, minute: u8, side: Side) {
        self.push(MatchEvent::new(
            minute,
            EventType::Corner,
            side,
            Zone::attacking_box(side),
        ));
        // Cleared: a corner that leads nowhere must not claim a later goal.
        self.push(MatchEvent::new(
            minute,
            EventType::Clearance,
            side.opposite(),
            Zone::attacking_box(side),
        ));
    }
}

fn schedule<R: Rng>(
    slots: &mut Vec<Slot>,
    side: Side,
    plan: &SidePlan,
    first: u8,
    last: u8,
    rng: &mut R,
) {
    let mut add = |count: u32, happening: Happening, rng: &mut R| {
        for _ in 0..count {
            slots.push(Slot {
                minute: rng.random_range(first..=last),
                side,
                happening,
            });
        }
    };
    add(plan.open_goals, Happening::Goal, rng);
    add(plan.penalties_scored, Happening::PenaltyGoal, rng);
    add(plan.penalties_missed, Happening::PenaltyMiss, rng);
    add(plan.saved, Happening::ShotSaved, rng);
    add(plan.off_target, Happening::ShotOffTarget, rng);
    add(plan.blocked, Happening::ShotBlocked, rng);
    add(plan.fouls, Happening::Foul, rng);
    add(plan.corners, Happening::Corner, rng);
    add(plan.injuries, Happening::Injury, rng);
    for &minute in &plan.substitution_minutes {
        slots.push(Slot {
            minute: minute.clamp(first, last),
            side,
            happening: Happening::Substitution,
        });
    }
}

/// Split a side's expected shooting into its parts, all drawn here so the
/// totals agree with each other (a goal is always also a shot on target).
pub(super) fn plan_attempts<R: Rng>(
    plan: &mut SidePlan,
    model: &QuickModel,
    goals: f64,
    on_target: f64,
    shots: f64,
    rng: &mut R,
) {
    let events = &model.events;
    let awarded = poisson(rng, events.penalties_awarded);
    plan.penalties_scored = binomial(rng, awarded, events.penalty_conversion);
    plan.penalties_missed = awarded - plan.penalties_scored;
    let penalty_goals = events.penalties_awarded * events.penalty_conversion;
    plan.open_goals = poisson(rng, (goals - penalty_goals).max(0.05));
    plan.saved = poisson(rng, (on_target - goals).max(0.2));
    let wide_or_blocked = poisson(rng, (shots - on_target).max(0.3));
    plan.blocked = binomial(rng, wide_or_blocked, events.blocked_share);
    plan.off_target = wide_or_blocked - plan.blocked;
    plan.corners = poisson(
        rng,
        (events.corners - events.corner_goal_share * goals).max(0.5),
    );
}
