-- Global search of the Topbar (spec 6.9), run in SQLite (spec 7).
--
-- Each item keeps its name and its other searchable fields normalized by the
-- app (lower case, without accents). The migration fills them for what is
-- already cached; each sync writes them after. The indexes over them come in
-- migration 10, once the columns are filled.
ALTER TABLE songs   ADD COLUMN search_name TEXT NOT NULL DEFAULT '';
ALTER TABLE albums  ADD COLUMN search_name TEXT NOT NULL DEFAULT '';
ALTER TABLE albums  ADD COLUMN search_text TEXT NOT NULL DEFAULT '';
ALTER TABLE artists ADD COLUMN search_name TEXT NOT NULL DEFAULT '';

-- The last searches (spec 6.9), newest first; `key` is the search
-- normalized, so "Beyoncé" and "beyonce" are one entry.
CREATE TABLE recent_searches (
    key         TEXT PRIMARY KEY NOT NULL,
    query       TEXT NOT NULL,                -- as typed
    searched_at INTEGER NOT NULL              -- Unix milliseconds
) WITHOUT ROWID;
