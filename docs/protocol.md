# Protocolo

Contrato entre `sun` (servidor) e `moon` (app). Os tipos vivem em [`shared/src/lib.rs`](../shared/src/lib.rs). **Este arquivo e aquele mudam juntos.**

## Convenções

- JSON em tudo. Campos em `snake_case`.
- Enums de mensagem usam tag interna `type` em `snake_case`: `{ "type": "ping" }`.
- Toda mudança no formato do fio precisa de um teste em `shared` que fixe o JSON esperado (ver `client_message_wire_format`).
- Mudanças que quebram compatibilidade são aceitáveis enquanto o projeto for `0.x`. Host e amigos atualizam juntos.

## HTTP (`sun`, porta 7700)

| Método | Rota | Auth | Resposta | Descrição |
|---|---|---|---|---|
| `GET` | `/health` | não | `200 ok` (texto) | Verificação de vida |
| `GET` | `/ws` | não (Fase 1: JWT) | upgrade WebSocket | Canal de tempo real |

## WebSocket (`/ws`)

### Cliente → servidor (`ClientMessage`)

| `type` | Campos | Resposta |
|---|---|---|
| `ping` | — | `pong` |

### Servidor → cliente (`ServerMessage`)

| `type` | Campos | Quando |
|---|---|---|
| `pong` | — | Resposta a `ping` |

Mensagens inválidas são ignoradas (com log `warn` no servidor) e a conexão continua aberta.

## Comandos Tauri (front → core Rust do `moon`)

Gerados em `moon/src/bindings.ts` como `commands.*`.

| Comando | Retorno | Descrição |
|---|---|---|
| `appVersion()` | `string` | Versão do app |
