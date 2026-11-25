-- Whether the career has national teams and international competitions.
-- NULL for saves written before it could be switched off; they read as on.
ALTER TABLE game_meta ADD COLUMN international_football INTEGER;
