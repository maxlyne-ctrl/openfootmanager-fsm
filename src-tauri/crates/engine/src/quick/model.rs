//! The numbers the quick resolver plays by.
//!
//! Every coefficient here was fitted to the minute-by-minute engine, not
//! invented. The rates (goals, shots on target, shots, possession) come from
//! ~23,000 league fixtures played by the full engine inside real careers —
//! four worlds, eight seasons each, through the game's own day loop — with
//! each side described as it stood the evening before (Poisson GLMs for the
//! counts, OLS for possession; `season-harness/examples/calib_season.rs`).
//! The per-player division of labour comes from 30,000 day-one fixtures
//! (`calib_collect.rs`), which log every player's line. The quick resolver
//! is a stand-in for the full engine, so its job is to agree with it: when
//! the full engine's balance changes, re-run both and refresh these values.
//!
//! The ratings are the ones the full engine itself plays by — a forward's
//! finishing is his shooting, composure and decisions, a keeper's is his
//! handling, reflexes and positioning, exactly as the full engine's shot
//! resolution reads them — plus its own composite attack, midfield and
//! defence ratings. Fitting on those, rather than on OVR, is what lets the
//! quick resolver tell two clubs of equal OVR apart the way the full engine
//! does. Ratings enter centred on [`RatingReference`], so each intercept reads
//! as "a side of reference quality, Balanced, on default tactics, against
//! the same, away from home".

use serde::{Deserialize, Serialize};

use crate::types::{
    DefensiveLine, PlayStyle, PressingIntensity, TacticsConfig, TacticsPitchWidth, Tempo,
};

/// Ratings the coefficients are centred on: roughly the mean AI side in the
/// shipped world, in the scale of [`super::side::SideRatings`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingReference {
    pub finishing: f64,
    pub keeping: f64,
    pub attack: f64,
    pub midfield: f64,
    pub defence: f64,
    pub condition: f64,
}

/// A log-linear rate (goals, shots on target, shots) for one side, from its
/// own ratings and tactics and the opposition's.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateModel {
    pub intercept: f64,
    pub home: f64,
    pub own_finishing: f64,
    pub own_attack: f64,
    pub own_midfield: f64,
    pub own_condition: f64,
    pub opp_keeping: f64,
    pub opp_defence: f64,
    pub opp_midfield: f64,
    pub opp_condition: f64,
    /// Effect of the side's own play style, relative to Balanced.
    pub own_style: StyleEffects,
    /// Effect of the opposition's play style on this side's rate.
    pub opp_style: StyleEffects,
    pub own_tactics: TacticEffects,
    pub opp_tactics: TacticEffects,
}

/// Effects of the tactical dials AI clubs actually vary, each relative to the
/// dial's default setting. The other dials (build-up, marking, shape,
/// counter-press, break speed) sit at their defaults for AI clubs, so the
/// calibration has nothing to measure them by.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TacticEffects {
    pub pressing_passive: f64,
    pub pressing_aggressive: f64,
    pub line_very_low: f64,
    pub line_low: f64,
    pub line_high: f64,
    pub tempo_patient: f64,
    pub width_narrow: f64,
    pub width_wide: f64,
}

impl TacticEffects {
    pub fn of(&self, tactics: &TacticsConfig) -> f64 {
        let pressing = match tactics.pressing_intensity {
            PressingIntensity::Passive => self.pressing_passive,
            PressingIntensity::Medium => 0.0,
            PressingIntensity::Aggressive => self.pressing_aggressive,
        };
        let line = match tactics.defensive_line {
            DefensiveLine::VeryLow => self.line_very_low,
            DefensiveLine::Low => self.line_low,
            DefensiveLine::Medium => 0.0,
            DefensiveLine::High => self.line_high,
        };
        let tempo = match tactics.tempo {
            Tempo::Patient => self.tempo_patient,
            Tempo::Direct => 0.0,
        };
        let width = match tactics.width {
            TacticsPitchWidth::Narrow => self.width_narrow,
            TacticsPitchWidth::Normal => 0.0,
            TacticsPitchWidth::Wide => self.width_wide,
        };
        pressing + line + tempo + width
    }
}

/// One value per play style; `balanced` is the reference and normally zero.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleEffects {
    pub balanced: f64,
    pub attacking: f64,
    pub defensive: f64,
    pub possession: f64,
    pub counter: f64,
    pub high_press: f64,
}

impl StyleEffects {
    pub fn of(&self, style: PlayStyle) -> f64 {
        match style {
            PlayStyle::Balanced => self.balanced,
            PlayStyle::Attacking => self.attacking,
            PlayStyle::Defensive => self.defensive,
            PlayStyle::Possession => self.possession,
            PlayStyle::Counter => self.counter,
            PlayStyle::HighPress => self.high_press,
        }
    }
}

/// Home possession share in percentage points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PossessionModel {
    /// Home share with equal sides on default tactics, both Balanced;
    /// carries the home advantage.
    pub home_intercept: f64,
    /// Percentage points per point of home-minus-away midfield rating.
    pub per_midfield_point: f64,
    pub home_style: StyleEffects,
    pub away_style: StyleEffects,
    pub home_tactics: TacticEffects,
    pub away_tactics: TacticEffects,
    /// Match-to-match spread the ratings do not explain (standard deviation,
    /// percentage points).
    pub noise: f64,
    pub min_share: f64,
    pub max_share: f64,
}

/// Per-90-minute rates for one outfield line or the goalkeeper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionRates {
    /// Share of the side's goals scored by this line (weights, not normalised).
    pub goal_weight: f64,
    pub assist_weight: f64,
    pub shot_weight: f64,
    pub passes_attempted: f64,
    pub passes_completed: f64,
    pub tackles: f64,
    pub interceptions: f64,
    pub fouls: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionTable {
    pub goalkeeper: PositionRates,
    pub defender: PositionRates,
    pub midfielder: PositionRates,
    pub forward: PositionRates,
}

/// Everything else, as per-side averages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRates {
    /// Penalties awarded per side per match.
    pub penalties_awarded: f64,
    pub penalty_conversion: f64,
    /// Share of goals with an assist credited.
    pub assisted_share: f64,
    /// Share of open-play goals that come straight from a corner / free kick.
    pub corner_goal_share: f64,
    pub free_kick_goal_share: f64,
    /// Share of off-target-or-blocked attempts that were blocked.
    pub blocked_share: f64,
    pub corners: f64,
    /// Free kicks awarded per foul suffered.
    pub free_kicks_per_foul: f64,
    pub yellow_per_foul: f64,
    pub straight_red_per_foul: f64,
    pub injuries: f64,
    /// Probability a side makes 0, 1, 2, 3, 4 or 5 substitutions.
    pub substitution_counts: [f64; 6],
    /// Earliest and latest minute of a normal-time substitution; the bulk
    /// falls after the hour.
    pub substitution_window: (u8, u8),
    /// Goal rate in extra time relative to normal time, per minute.
    pub extra_time_goal_factor: f64,
    pub shootout_conversion: f64,
    /// Per point of shooting above 70, on the log scale of a forward's share.
    pub shooting_skill_slope: f64,
    /// Per point of aggression above 60, on the log scale of a player's fouls.
    pub aggression_slope: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickModel {
    pub reference: RatingReference,
    pub goals: RateModel,
    pub shots_on_target: RateModel,
    pub shots: RateModel,
    pub possession: PossessionModel,
    pub positions: PositionTable,
    pub events: EventRates,
}

impl Default for QuickModel {
    fn default() -> Self {
        Self {
            reference: RatingReference {
                finishing: 79.0,
                keeping: 94.0,
                attack: 80.0,
                midfield: 82.0,
                defence: 84.0,
                condition: 92.0,
            },
            goals: fitted_goals(),
            shots_on_target: fitted_shots_on_target(),
            shots: fitted_shots(),
            possession: fitted_possession(),
            positions: fitted_positions(),
            events: fitted_events(),
        }
    }
}

fn fitted_goals() -> RateModel {
    RateModel {
        intercept: 0.2396,
        home: 0.1377,
        own_finishing: 0.0364,
        own_attack: 0.0156,
        own_midfield: -0.0018,
        own_condition: 0.015,
        opp_keeping: -0.0277,
        opp_defence: -0.0047,
        opp_midfield: -0.0041,
        opp_condition: -0.0099,
        own_style: StyleEffects {
            balanced: 0.0,
            attacking: 0.2069,
            defensive: -0.0644,
            possession: 0.0983,
            counter: 0.0829,
            high_press: 0.1946,
        },
        opp_style: StyleEffects {
            balanced: 0.0,
            attacking: 0.2169,
            defensive: -0.0744,
            possession: 0.1186,
            counter: 0.0267,
            high_press: 0.2033,
        },
        own_tactics: TacticEffects {
            pressing_passive: 0.0185,
            pressing_aggressive: -0.0911,
            line_very_low: 0.0567,
            line_low: -0.0382,
            line_high: 0.0648,
            tempo_patient: 0.0339,
            width_narrow: 0.0,
            width_wide: -0.095,
        },
        opp_tactics: TacticEffects {
            pressing_passive: -0.0477,
            pressing_aggressive: -0.0316,
            line_very_low: -0.0481,
            line_low: 0.0004,
            line_high: 0.0132,
            tempo_patient: 0.0442,
            width_narrow: 0.0,
            width_wide: -0.0021,
        },
    }
}

fn fitted_shots_on_target() -> RateModel {
    RateModel {
        intercept: 1.754,
        home: 0.1452,
        own_finishing: 0.0123,
        own_attack: 0.0089,
        own_midfield: 0.0022,
        own_condition: 0.0069,
        opp_keeping: 0.0033,
        opp_defence: -0.0048,
        opp_midfield: -0.0053,
        opp_condition: -0.0031,
        own_style: StyleEffects {
            balanced: 0.0,
            attacking: 0.171,
            defensive: -0.0507,
            possession: 0.0852,
            counter: 0.0217,
            high_press: 0.1653,
        },
        opp_style: StyleEffects {
            balanced: 0.0,
            attacking: 0.0708,
            defensive: -0.0194,
            possession: 0.0076,
            counter: 0.0313,
            high_press: 0.0623,
        },
        own_tactics: TacticEffects {
            pressing_passive: -0.029,
            pressing_aggressive: -0.0481,
            line_very_low: -0.0573,
            line_low: 0.0282,
            line_high: 0.0154,
            tempo_patient: 0.0345,
            width_narrow: 0.0,
            width_wide: -0.0516,
        },
        opp_tactics: TacticEffects {
            pressing_passive: 0.012,
            pressing_aggressive: -0.0092,
            line_very_low: 0.0059,
            line_low: 0.0061,
            line_high: -0.0337,
            tempo_patient: -0.0117,
            width_narrow: 0.0,
            width_wide: 0.0333,
        },
    }
}

fn fitted_shots() -> RateModel {
    RateModel {
        intercept: 2.4504,
        home: 0.1445,
        own_finishing: 0.0013,
        own_attack: 0.0055,
        own_midfield: 0.0039,
        own_condition: 0.0035,
        opp_keeping: 0.0032,
        opp_defence: -0.0042,
        opp_midfield: -0.0057,
        opp_condition: -0.0033,
        own_style: StyleEffects {
            balanced: 0.0,
            attacking: 0.1215,
            defensive: -0.0362,
            possession: 0.0654,
            counter: 0.0219,
            high_press: 0.1319,
        },
        opp_style: StyleEffects {
            balanced: 0.0,
            attacking: 0.0743,
            defensive: -0.0187,
            possession: 0.0136,
            counter: 0.0419,
            high_press: 0.062,
        },
        own_tactics: TacticEffects {
            pressing_passive: -0.0142,
            pressing_aggressive: -0.0333,
            line_very_low: -0.0239,
            line_low: 0.0097,
            line_high: 0.0215,
            tempo_patient: 0.0292,
            width_narrow: 0.0,
            width_wide: -0.0338,
        },
        opp_tactics: TacticEffects {
            pressing_passive: 0.0232,
            pressing_aggressive: -0.0011,
            line_very_low: 0.0401,
            line_low: -0.0169,
            line_high: -0.0314,
            tempo_patient: -0.0051,
            width_narrow: 0.0,
            width_wide: 0.0312,
        },
    }
}

fn fitted_possession() -> PossessionModel {
    PossessionModel {
        home_intercept: 51.6765,
        per_midfield_point: 0.27,
        home_style: StyleEffects {
            balanced: 0.0,
            attacking: 1.8572,
            defensive: -0.9728,
            possession: 3.4785,
            counter: -0.3858,
            high_press: 3.506,
        },
        away_style: StyleEffects {
            balanced: 0.0,
            attacking: -1.2063,
            defensive: 1.0248,
            possession: -3.2245,
            counter: 0.8245,
            high_press: -2.7289,
        },
        home_tactics: TacticEffects {
            pressing_passive: -1.3586,
            pressing_aggressive: 0.3002,
            line_very_low: -0.7243,
            line_low: -0.6343,
            line_high: 0.3868,
            tempo_patient: 2.5057,
            width_narrow: 0.0,
            width_wide: 0.0988,
        },
        away_tactics: TacticEffects {
            pressing_passive: 1.8493,
            pressing_aggressive: -0.6011,
            line_very_low: 1.9395,
            line_low: -0.0902,
            line_high: -0.9299,
            tempo_patient: -2.1997,
            width_narrow: 0.0,
            width_wide: -0.0384,
        },
        noise: 5.108,
        min_share: 25.0,
        max_share: 75.0,
    }
}

/// The full engine's division of labour: forwards take nearly every shot and
/// goal, midfielders provide nearly every assist and most passes. Mirrored
/// here so match ratings — whose per-position baselines were measured on the
/// full engine — mean the same thing whichever resolver played the match.
fn fitted_positions() -> PositionTable {
    PositionTable {
        goalkeeper: PositionRates {
            goal_weight: 0.0,
            assist_weight: 0.0,
            shot_weight: 0.0,
            passes_attempted: 0.0,
            passes_completed: 0.0,
            tackles: 0.0,
            interceptions: 0.0,
            fouls: 0.0,
        },
        defender: PositionRates {
            goal_weight: 0.016,
            assist_weight: 0.002,
            shot_weight: 0.102,
            passes_attempted: 1.129,
            passes_completed: 0.610,
            tackles: 2.392,
            interceptions: 0.010,
            fouls: 1.091,
        },
        midfielder: PositionRates {
            goal_weight: 0.0,
            assist_weight: 0.500,
            shot_weight: 0.0,
            passes_attempted: 5.156,
            passes_completed: 5.155,
            tackles: 2.910,
            interceptions: 2.461,
            fouls: 1.178,
        },
        forward: PositionRates {
            goal_weight: 0.949,
            assist_weight: 0.023,
            shot_weight: 5.502,
            passes_attempted: 0.303,
            passes_completed: 0.276,
            tackles: 0.249,
            interceptions: 0.116,
            fouls: 0.117,
        },
    }
}

fn fitted_events() -> EventRates {
    EventRates {
        penalties_awarded: 0.232,
        penalty_conversion: 0.78,
        assisted_share: 0.92,
        corner_goal_share: 0.165,
        free_kick_goal_share: 0.08,
        blocked_share: 0.40,
        corners: 5.32,
        free_kicks_per_foul: 0.95,
        yellow_per_foul: 0.119,
        straight_red_per_foul: 0.005,
        injuries: 0.30,
        substitution_counts: [0.164, 0.457, 0.184, 0.137, 0.050, 0.008],
        substitution_window: (55, 88),
        extra_time_goal_factor: 0.85,
        shootout_conversion: 0.76,
        shooting_skill_slope: 0.012,
        aggression_slope: 0.012,
    }
}
