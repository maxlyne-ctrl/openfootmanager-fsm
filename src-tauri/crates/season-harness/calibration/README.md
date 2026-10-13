# Quick resolver calibration

The quick match resolver (`crates/engine/src/quick/`) stands in for the full
minute-by-minute engine on fixtures nobody watches. Its coefficients
(`quick/model.rs`) are fitted to the full engine, so re-run this whenever the
full engine's balance changes.

Requires Python 3 with `pandas`, `numpy` and `statsmodels`. Run from
`src-tauri/`; paths below put the data in `calib/`.

## 1. Collect in-season data from both engines

```bash
cargo build --release -p season-harness --examples
mkdir -p calib/season
for s in 42 43 44 45; do
  ./target/release/examples/calib_season 8 calib/season/full_$s.csv full $s &
done; wait
```

Each run plays 8 seasons of a three-nation world and logs every AI league
fixture in the active scope: both sides as they stood the evening before, and
the result.

## 2. Fit

```bash
(cd calib && python3 ../crates/season-harness/calibration/fit_quick_model.py)
```

Writes `calib/model_v2.json`: Poisson GLMs for goals, shots on target and
shots, OLS for possession. Copy the values into `quick/model.rs`
(`fitted_goals`, `fitted_shots_on_target`, `fitted_shots`,
`fitted_possession`, and the `RatingReference`).

## 3. Check

```bash
for s in 42 43 44 45; do
  ./target/release/examples/calib_season 8 calib/season/quick_$s.csv quick $s &
done; wait
(cd calib && python3 ../crates/season-harness/calibration/compare_seasons.py)
```

The two engines should agree on goals per match, draw and home-win rates,
shot volumes, fouls, cards, possession spread, and the spread of points in a
league (points-per-game standard deviation, champion and bottom totals).

Per-player division of labour (who shoots, scores, assists, passes) comes
from `calib_collect` (day-one fixtures, every player's line) and
`compare_fixtures.py`.

## Results at the time of writing (13 Oct 2026)

| | Full engine | Quick resolver |
|---|---|---|
| Goals per match | 3.49 | 3.49 |
| Draws | 21.3% | 21.9% |
| Home wins | 43.8% | 43.3% |
| Shots / on target per side | 13.65 / 6.89 | 13.69 / 6.89 |
| Fouls / yellows per side | 9.49 / 1.13 | 9.44 / 1.11 |
| Possession s.d. | 7.66 | 7.64 |
| Points-per-game s.d. (league) | 0.41 | 0.40 |
| Champion points (Div 1 / Div 2) | 84.3 / 86.2 | 82.4 / 80.9 |
