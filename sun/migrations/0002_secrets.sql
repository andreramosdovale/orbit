-- Segredos gerados pelo próprio `sun` (ex.: chave do JWT, ADR 0008).
CREATE TABLE secrets (
    name  TEXT PRIMARY KEY,
    value BLOB NOT NULL
);
