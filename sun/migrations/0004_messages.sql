-- Mensagens de texto. Guardadas sem limite de tempo (ADR 0011).
CREATE TABLE messages (
    id         INTEGER PRIMARY KEY,
    channel_id INTEGER NOT NULL REFERENCES channels (id),
    author_id  INTEGER NOT NULL REFERENCES users (id),
    content    TEXT    NOT NULL,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Também atende o histórico paginado por canal.
CREATE INDEX messages_channel_id ON messages (channel_id, id);
