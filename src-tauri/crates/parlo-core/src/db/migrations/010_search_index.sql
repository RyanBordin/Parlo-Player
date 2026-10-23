-- FTS5 indexes of the global search (spec 6.9) over the columns filled by
-- migration 9: they find the items where every word typed starts a word of
-- some field. Each index reads the text from its table (`content=`).
--
-- Order matters: the indexes are built from the tables first ('rebuild'),
-- and only then do the triggers keep them in step with every write. A
-- trigger that removes from an index text it never held corrupts it.
--
-- `IF NOT EXISTS`: development databases may already have them from an
-- earlier version of migration 9.

-- The text is already normalized, so the tokenizer only splits the words.
-- `prefix` speeds up the first letters typed.
CREATE VIRTUAL TABLE IF NOT EXISTS songs_fts USING fts5(
    search_name, search_text, content = 'songs', tokenize = 'unicode61 remove_diacritics 0', prefix = '1 2'
);
CREATE VIRTUAL TABLE IF NOT EXISTS albums_fts USING fts5(
    search_name, search_text, content = 'albums', tokenize = 'unicode61 remove_diacritics 0', prefix = '1 2'
);
CREATE VIRTUAL TABLE IF NOT EXISTS artists_fts USING fts5(
    search_name, content = 'artists', tokenize = 'unicode61 remove_diacritics 0', prefix = '1 2'
);

INSERT INTO songs_fts (songs_fts) VALUES ('rebuild');
INSERT INTO albums_fts (albums_fts) VALUES ('rebuild');
INSERT INTO artists_fts (artists_fts) VALUES ('rebuild');

CREATE TRIGGER IF NOT EXISTS songs_fts_insert AFTER INSERT ON songs BEGIN
    INSERT INTO songs_fts (rowid, search_name, search_text) VALUES (new.rowid, new.search_name, new.search_text);
END;
CREATE TRIGGER IF NOT EXISTS songs_fts_delete AFTER DELETE ON songs BEGIN
    INSERT INTO songs_fts (songs_fts, rowid, search_name, search_text)
    VALUES ('delete', old.rowid, old.search_name, old.search_text);
END;
-- A sync writes every column again: only a new text touches the index.
CREATE TRIGGER IF NOT EXISTS songs_fts_update AFTER UPDATE OF search_name, search_text ON songs
WHEN old.search_name IS NOT new.search_name OR old.search_text IS NOT new.search_text BEGIN
    INSERT INTO songs_fts (songs_fts, rowid, search_name, search_text)
    VALUES ('delete', old.rowid, old.search_name, old.search_text);
    INSERT INTO songs_fts (rowid, search_name, search_text) VALUES (new.rowid, new.search_name, new.search_text);
END;

CREATE TRIGGER IF NOT EXISTS albums_fts_insert AFTER INSERT ON albums BEGIN
    INSERT INTO albums_fts (rowid, search_name, search_text) VALUES (new.rowid, new.search_name, new.search_text);
END;
CREATE TRIGGER IF NOT EXISTS albums_fts_delete AFTER DELETE ON albums BEGIN
    INSERT INTO albums_fts (albums_fts, rowid, search_name, search_text)
    VALUES ('delete', old.rowid, old.search_name, old.search_text);
END;
CREATE TRIGGER IF NOT EXISTS albums_fts_update AFTER UPDATE OF search_name, search_text ON albums
WHEN old.search_name IS NOT new.search_name OR old.search_text IS NOT new.search_text BEGIN
    INSERT INTO albums_fts (albums_fts, rowid, search_name, search_text)
    VALUES ('delete', old.rowid, old.search_name, old.search_text);
    INSERT INTO albums_fts (rowid, search_name, search_text) VALUES (new.rowid, new.search_name, new.search_text);
END;

CREATE TRIGGER IF NOT EXISTS artists_fts_insert AFTER INSERT ON artists BEGIN
    INSERT INTO artists_fts (rowid, search_name) VALUES (new.rowid, new.search_name);
END;
CREATE TRIGGER IF NOT EXISTS artists_fts_delete AFTER DELETE ON artists BEGIN
    INSERT INTO artists_fts (artists_fts, rowid, search_name) VALUES ('delete', old.rowid, old.search_name);
END;
CREATE TRIGGER IF NOT EXISTS artists_fts_update AFTER UPDATE OF search_name ON artists
WHEN old.search_name IS NOT new.search_name BEGIN
    INSERT INTO artists_fts (artists_fts, rowid, search_name) VALUES ('delete', old.rowid, old.search_name);
    INSERT INTO artists_fts (rowid, search_name) VALUES (new.rowid, new.search_name);
END;
