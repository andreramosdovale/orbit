# 0004. SQLite com persistência mínima

- **Status:** Aceita
- **Data:** 2026-11-03

## Contexto

O servidor roda no PC do host. Queremos zero configuração e o mínimo de dados guardados.

## Decisão

Um único arquivo **SQLite** (`orbit.db`) acessado via `sqlx`, com migrations versionadas em `sun/migrations/`. Persistimos apenas usuários (hash argon2), canais e mensagens de texto. Voz e vídeo nunca são gravados.

## Alternativas consideradas

- **Postgres:** exigiria instalar e manter um serviço no PC do host.
- **Só memória:** perderia contas e histórico a cada reinício.

## Consequências

- Backup = copiar um arquivo.
- Escrita concorrente limitada, o que é irrelevante para ~10 usuários.
- Política de retenção de mensagens fica para decidir na Fase 2.
