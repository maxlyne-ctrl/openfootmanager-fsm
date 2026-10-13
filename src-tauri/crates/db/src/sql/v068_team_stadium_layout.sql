-- The ground as built, stand by stand: JSON of domain::stadium::StadiumLayout.
-- NULL for rows written before grounds were modelled; the game generates a
-- layout for those on load.
ALTER TABLE teams ADD COLUMN stadium_layout_json TEXT;
