//! One side of a quick match: its ratings going in, and who is on the pitch
//! as the match unfolds.

use std::collections::HashSet;

use rand::{Rng, RngExt};

use crate::types::{PlayStyle, PlayerData, Position, TacticsConfig, TeamData};

/// A side's ratings as the full engine reads them: its forwards' finishing
/// and its keeper's shot-stopping (the two the full engine's shot resolution
/// uses), its composite attack, midfield and defence, and its condition.
#[derive(Debug, Clone, Copy)]
pub(super) struct SideRatings {
    pub finishing: f64,
    pub keeping: f64,
    pub attack: f64,
    pub midfield: f64,
    pub defence: f64,
    pub condition: f64,
}

/// A forward's finishing, as the full engine's shot resolution computes it.
pub(super) fn finishing(player: &PlayerData) -> f64 {
    (f64::from(player.shooting) + f64::from(player.composure) + f64::from(player.decisions)) / 3.0
}

fn keeping(player: &PlayerData) -> f64 {
    (f64::from(player.handling) + f64::from(player.reflexes) + f64::from(player.positioning)) / 3.0
}

impl SideRatings {
    pub(super) fn of(team: &TeamData) -> Self {
        let forwards = mean(
            team.players
                .iter()
                .filter(|p| p.position == Position::Forward)
                .map(finishing),
        );
        // No forwards: whoever is pushed up front finishes worse than the
        // side's outfielders usually do.
        let finishing = forwards.unwrap_or_else(|| {
            mean(
                team.players
                    .iter()
                    .filter(|p| p.position != Position::Goalkeeper)
                    .map(finishing),
            )
            .unwrap_or(40.0)
                - 5.0
        });
        let keeping = team
            .players
            .iter()
            .find(|p| p.position == Position::Goalkeeper)
            .map_or(40.0, keeping);
        Self {
            finishing,
            keeping,
            attack: team.attack_rating(),
            midfield: team.midfield_rating(),
            defence: team.defense_rating(),
            condition: mean(team.players.iter().map(|p| f64::from(p.condition))).unwrap_or(85.0),
        }
    }
}

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, count) = values.fold((0.0, 0u32), |(s, n), v| (s + v, n + 1));
    (count > 0).then(|| sum / f64::from(count))
}

/// Squad and pitch state for one side.
pub(super) struct SideState<'a> {
    pub style: PlayStyle,
    pub tactics: TacticsConfig,
    pub ratings: SideRatings,
    /// Starters first, then the bench; indices below refer to this list.
    pub squad: Vec<&'a PlayerData>,
    pub starters: usize,
    pub on_pitch: Vec<usize>,
    pub used_bench: HashSet<usize>,
    pub booked: HashSet<usize>,
    pub dismissed: HashSet<usize>,
    pub substitutions_left: u8,
}

impl<'a> SideState<'a> {
    pub(super) fn new(team: &'a TeamData, bench: &'a [PlayerData]) -> Self {
        let squad: Vec<&PlayerData> = team.players.iter().chain(bench.iter()).collect();
        let starters = team.players.len();
        Self {
            style: team.play_style,
            tactics: team.tactics.clone(),
            ratings: SideRatings::of(team),
            squad,
            starters,
            on_pitch: (0..starters).collect(),
            used_bench: HashSet::new(),
            booked: HashSet::new(),
            dismissed: HashSet::new(),
            substitutions_left: 5,
        }
    }

    pub(super) fn player(&self, index: usize) -> &'a PlayerData {
        self.squad[index]
    }

    /// Everyone who could have played: the ratings pass needs the whole list.
    pub(super) fn everyone(&self) -> Vec<&'a PlayerData> {
        self.squad.clone()
    }

    /// Ids of everyone who started or came on, for minutes played.
    pub(super) fn appeared_ids(&self) -> Vec<String> {
        (0..self.starters)
            .chain(self.used_bench.iter().copied())
            .map(|i| self.squad[i].id.clone())
            .collect()
    }

    /// Pick an on-pitch player, weighted by `weight`. `None` if nobody on the
    /// pitch has any weight.
    pub(super) fn pick<R: Rng>(
        &self,
        rng: &mut R,
        exclude: Option<usize>,
        weight: impl Fn(&PlayerData) -> f64,
    ) -> Option<usize> {
        let candidates: Vec<(usize, f64)> = self
            .on_pitch
            .iter()
            .copied()
            .filter(|&i| Some(i) != exclude)
            .map(|i| (i, weight(self.squad[i]).max(0.0)))
            .filter(|&(_, w)| w > 0.0)
            .collect();
        let total: f64 = candidates.iter().map(|(_, w)| w).sum();
        if total <= 0.0 {
            return None;
        }
        let mut roll = rng.random_range(0.0..total);
        for &(index, w) in &candidates {
            if roll < w {
                return Some(index);
            }
            roll -= w;
        }
        candidates.last().map(|&(index, _)| index)
    }

    /// Send a player off: he takes no further part.
    pub(super) fn dismiss(&mut self, index: usize) {
        self.dismissed.insert(index);
        self.on_pitch.retain(|&i| i != index);
    }

    /// Bring a bench player on for someone on the pitch: like for like where
    /// the bench allows, never the goalkeeper. Returns `(on, off)`.
    pub(super) fn substitute<R: Rng>(&mut self, rng: &mut R) -> Option<(usize, usize)> {
        if self.substitutions_left == 0 {
            return None;
        }
        let outfield: Vec<usize> = self
            .on_pitch
            .iter()
            .copied()
            .filter(|&i| self.squad[i].position != Position::Goalkeeper)
            .collect();
        if outfield.is_empty() {
            return None;
        }
        let off = outfield[rng.random_range(0..outfield.len())];
        let wanted = self.squad[off].position;
        let available: Vec<usize> = (self.starters..self.squad.len())
            .filter(|i| !self.used_bench.contains(i))
            .filter(|&i| self.squad[i].position != Position::Goalkeeper)
            .collect();
        let like_for_like: Vec<usize> = available
            .iter()
            .copied()
            .filter(|&i| self.squad[i].position == wanted)
            .collect();
        let pool = if like_for_like.is_empty() {
            &available
        } else {
            &like_for_like
        };
        let on = *pool.get(rng.random_range(0..pool.len().max(1)))?;
        self.used_bench.insert(on);
        self.on_pitch.retain(|&i| i != off);
        self.on_pitch.push(on);
        self.substitutions_left -= 1;
        Some((on, off))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TacticsConfig;
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
            pace: 60,
            stamina: 60,
            strength: 60,
            agility: 60,
            passing: 60,
            shooting: 60,
            tackling: 60,
            dribbling: 60,
            defending: 60,
            positioning: 60,
            vision: 60,
            decisions: 60,
            composure: 60,
            aggression: 60,
            teamwork: 60,
            leadership: 60,
            handling: 60,
            reflexes: 60,
            aerial: 60,
            traits: vec![],
            role: Default::default(),
        }
    }

    fn team(players: Vec<PlayerData>) -> TeamData {
        TeamData {
            id: "t".into(),
            name: "T".into(),
            formation: "4-4-2".into(),
            play_style: PlayStyle::Balanced,
            players,
            tactics: TacticsConfig::default(),
        }
    }

    /// Given a side with no forwards, when it is rated, then its finishing
    /// falls back to its outfielders' less five rather than to zero.
    #[test]
    fn a_side_without_forwards_still_finishes_badly_not_never() {
        let t = team(vec![
            player("gk", Position::Goalkeeper, 70),
            player("d", Position::Defender, 70),
            player("m", Position::Midfielder, 70),
        ]);
        let ratings = SideRatings::of(&t);
        assert!(
            (ratings.finishing - 55.0).abs() < 1e-9,
            "{}",
            ratings.finishing
        );
        assert!((ratings.keeping - 60.0).abs() < 1e-9, "{}", ratings.keeping);
    }

    /// Given a bench with a defender and a forward, when a defender is taken
    /// off, then the defender comes on and the forward stays on the bench.
    #[test]
    fn substitutions_are_like_for_like_when_the_bench_allows() {
        let t = team(vec![
            player("gk", Position::Goalkeeper, 70),
            player("d1", Position::Defender, 70),
        ]);
        let bench = vec![
            player("f2", Position::Forward, 70),
            player("d2", Position::Defender, 70),
        ];
        let mut side = SideState::new(&t, &bench);
        let mut rng = StdRng::seed_from_u64(1);
        let (on, off) = side.substitute(&mut rng).expect("a substitution");
        assert_eq!(side.player(off).id, "d1");
        assert_eq!(side.player(on).id, "d2");
        assert!(side.on_pitch.contains(&on) && !side.on_pitch.contains(&off));
    }

    /// Given only a goalkeeper on the pitch, when a substitution is asked for,
    /// then none is made: the keeper is never taken off.
    #[test]
    fn the_goalkeeper_is_never_substituted() {
        let t = team(vec![player("gk", Position::Goalkeeper, 70)]);
        let bench = vec![player("gk2", Position::Goalkeeper, 70)];
        let mut side = SideState::new(&t, &bench);
        let mut rng = StdRng::seed_from_u64(1);
        assert!(side.substitute(&mut rng).is_none());
    }
}
