-- Canais de texto. `name` já chega normalizado (minúsculas) pelo `sun`.
CREATE TABLE channels (
    id         INTEGER PRIMARY KEY,
    name       TEXT    NOT NULL UNIQUE,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Um servidor novo nunca abre vazio.
INSERT INTO channels (name) VALUES ('geral');
