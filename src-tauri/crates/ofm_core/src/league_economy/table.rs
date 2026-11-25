//! The real-world table and the reputation fallback.

use std::sync::OnceLock;

use serde::Deserialize;

use super::{EconomySource, LeagueEconomy, RevenueSplit};

const EMBEDDED_TABLE: &str = include_str!("../../data/league_economy.json");

/// Wages as a share of revenue where the table is silent: European top divisions pay 47% of
/// revenue in player wages and the rest of the wage bill takes it near 60 (UEFA ECFIL 2025);
/// lower divisions chase promotion and pay more.
const FALLBACK_TOP_FLIGHT_WAGE_RATIO: u32 = 62;
const FALLBACK_LOWER_DIVISION_WAGE_RATIO: u32 = 75;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct EconomyTable {
    reputation_knots: Vec<(u32, i64)>,
    leagues: Vec<CountryRow>,
}

#[derive(Debug, Deserialize)]
struct CountryRow {
    country: String,
    divisions: Vec<DivisionRow>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DivisionRow {
    revenue: i64,
    wage_ratio: u32,
}

pub(super) fn parse_table(json: &str) -> Result<EconomyTable, serde_json::Error> {
    serde_json::from_str(json)
}

/// The embedded table, parsed once. A table that fails to parse is a build defect the tests
/// catch; at run time it degrades to no rows and no knots rather than stopping the game.
fn table() -> &'static EconomyTable {
    static TABLE: OnceLock<EconomyTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        parse_table(EMBEDDED_TABLE).unwrap_or_else(|error| {
            log::error!("league economy table does not parse: {error}");
            EconomyTable {
                reputation_knots: Vec::new(),
                leagues: Vec::new(),
            }
        })
    })
}

pub(super) fn league_economy(country: &str, division: u32, mean_reputation: u32) -> LeagueEconomy {
    let rows = table()
        .leagues
        .iter()
        .find(|row| row.country == country)
        .map(|row| row.divisions.as_slice())
        .unwrap_or_default();
    if let Some(row) = usize::try_from(division)
        .ok()
        .and_then(|index| rows.get(index))
    {
        return economy(row.revenue, row.wage_ratio, EconomySource::Table);
    }
    let mut revenue = revenue_by_reputation(&table().reputation_knots, mean_reputation);
    // A division the table stops short of is still below the last one it lists.
    if let Some(lowest_listed) = rows.last() {
        revenue = revenue.min(lowest_listed.revenue);
    }
    let wage_ratio = if division == 0 {
        FALLBACK_TOP_FLIGHT_WAGE_RATIO
    } else {
        FALLBACK_LOWER_DIVISION_WAGE_RATIO
    };
    economy(revenue, wage_ratio, EconomySource::Reputation)
}

fn economy(revenue: i64, wage_ratio_percent: u32, source: EconomySource) -> LeagueEconomy {
    LeagueEconomy {
        revenue_per_club: revenue,
        split: split_for(revenue),
        wage_ratio_percent,
        source,
    }
}

/// Revenue for a league whose clubs average `reputation`, interpolated on a log scale between
/// the knots, so money multiplies as fame adds. Outside the knots the nearest one holds.
fn revenue_by_reputation(knots: &[(u32, i64)], reputation: u32) -> i64 {
    let (Some(first), Some(last)) = (knots.first(), knots.last()) else {
        return 0;
    };
    if reputation <= first.0 {
        return first.1;
    }
    if reputation >= last.0 {
        return last.1;
    }
    knots
        .windows(2)
        .find(|pair| reputation <= pair[1].0)
        .map_or(last.1, |pair| {
            let ((low_rep, low_rev), (high_rep, high_rev)) = (pair[0], pair[1]);
            let along = f64::from(reputation - low_rep) / f64::from(high_rep - low_rep);
            let log_revenue =
                (low_rev as f64).ln() * (1.0 - along) + (high_rev as f64).ln() * along;
            log_revenue.exp().round() as i64
        })
}

/// How revenue divides at a league of this size. The top flights earn half from television
/// (Premier League: GBP 3.4bn of 6.8bn); mid-sized leagues lean on the gate (Scotland 36%,
/// UEFA ECFIL 2025); small leagues live on it.
fn split_for(revenue_per_club: i64) -> RevenueSplit {
    let (broadcast, commercial) = match revenue_per_club {
        r if r >= 100_000_000 => (50, 33),
        r if r >= 20_000_000 => (35, 35),
        r if r >= 2_000_000 => (20, 35),
        _ => (5, 35),
    };
    RevenueSplit {
        broadcast_percent: broadcast,
        commercial_percent: commercial,
        matchday_percent: 100 - broadcast - commercial,
    }
}

#[cfg(test)]
mod table_tests {
    use super::*;

    /// Given the table that ships in the build
    /// When it is parsed
    /// Then it parses, its knots rise in both reputation and revenue, and every listed division
    ///      is poorer than the one above it.
    #[test]
    fn the_shipped_table_parses_and_every_ladder_falls() {
        let table = parse_table(EMBEDDED_TABLE).expect("the shipped table parses");

        assert!(
            table
                .reputation_knots
                .windows(2)
                .all(|pair| pair[0].0 < pair[1].0 && pair[0].1 < pair[1].1)
        );
        for row in &table.leagues {
            assert!(
                row.divisions
                    .windows(2)
                    .all(|pair| pair[0].revenue > pair[1].revenue),
                "{}",
                row.country
            );
        }
    }
}
