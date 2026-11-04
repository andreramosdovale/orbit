-- Salas de voz (spec 010). Uma tabela separada evita que uma sala receba mensagens
-- de texto ou apareça na lista de canais de texto.
CREATE TABLE voice_channels (
    id         INTEGER PRIMARY KEY,
    name       TEXT    NOT NULL UNIQUE,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

INSERT INTO voice_channels (name) VALUES ('geral');
