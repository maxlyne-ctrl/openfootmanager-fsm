-- Which economy the career's money follows: 'legacy' or 'leagues'.
-- NULL for saves written before the league economy; they read as 'legacy'.
ALTER TABLE game_meta ADD COLUMN economy TEXT;
