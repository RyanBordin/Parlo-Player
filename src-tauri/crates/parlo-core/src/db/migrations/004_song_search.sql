-- Quick search and sorting of the Songs page run in SQLite (spec 6.3, 7).
-- The values are normalized by the app (lower case, without accents), so
-- "Beyoncé" matches "beyonce" and sorts with the other "B"s. The migration
-- fills them for the songs already cached; each sync writes them after.
ALTER TABLE songs ADD COLUMN search_text TEXT NOT NULL DEFAULT '';
ALTER TABLE songs ADD COLUMN sort_title  TEXT NOT NULL DEFAULT '';
ALTER TABLE songs ADD COLUMN sort_artist TEXT;
ALTER TABLE songs ADD COLUMN sort_album  TEXT;
