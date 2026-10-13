//! Prints generated grounds as JSON, for the Stand Builder's gallery and for
//! eyeballing the generator: one ground per row of `SAMPLES`, with its figures.
//!
//! cargo run -p ofm_core --example stadium_gallery > grounds.json

use ofm_core::seed::rng_from_u64;
use ofm_core::stadium::{StadiumBrief, generate_stadium_layout, stadium_figures};

/// Label, target capacity, reputation, year founded, seed.
const SAMPLES: &[(&str, u32, u32, u32, u64)] = &[
    ("Step 6 village club", 900, 60, 1946, 11),
    ("Old non-league town club", 2_200, 150, 1884, 4),
    ("League Two, old ground", 6_500, 300, 1889, 2),
    ("League One, two stands rebuilt", 11_000, 420, 1902, 9),
    ("Championship, modernised", 21_000, 560, 1895, 5),
    ("Championship, new build", 30_000, 620, 1978, 13),
    ("Premier League bowl", 42_000, 780, 1880, 3),
    ("Big-city bowl", 62_000, 880, 1892, 17),
    ("National stadium", 90_000, 950, 1923, 21),
];

#[derive(serde::Serialize)]
struct Sample {
    label: &'static str,
    target: u32,
    figures: ofm_core::stadium::StadiumFigures,
    layout: domain::stadium::StadiumLayout,
}

fn main() -> Result<(), serde_json::Error> {
    let samples: Vec<Sample> = SAMPLES
        .iter()
        .map(|&(label, target, reputation, founded_year, seed)| {
            let brief = StadiumBrief {
                target_capacity: target,
                reputation,
                founded_year,
            };
            let layout = generate_stadium_layout(&brief, &mut rng_from_u64(seed));
            Sample {
                label,
                target,
                figures: stadium_figures(&layout),
                layout,
            }
        })
        .collect();
    println!("{}", serde_json::to_string(&samples)?);
    Ok(())
}
