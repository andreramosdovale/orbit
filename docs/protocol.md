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
| `POST` | `/auth/register` | não | `201` + `User` | Auto-cadastro ([spec 001](specs/001-cadastro-de-usuarios.md)) |

Corpos HTTP acima de **16 KiB** (`sun::MAX_BODY_SIZE`) são recusados. O CORS só libera as origens do `moon` (`sun::ALLOWED_ORIGINS`).

### `POST /auth/register`

Corpo `RegisterRequest`:

```json
{ "username": "fulano", "password": "senha-boa-123" }
```

- `username`: 3 a 32 caracteres `a-z`, `0-9` ou `_`. É convertido para minúsculas, então `Fulano` e `fulano` são o mesmo usuário.
- `password`: 8 a 128 caracteres. Guardada só como hash argon2id.

Sucesso: `201` com `User`, ou seja, `{ "id": 1, "username": "fulano" }`.

### Erros da API REST

Toda resposta de erro tem corpo `ApiErrorBody`: `{ "code": "<ApiErrorCode>", "message": "<texto para o usuário>" }`.

| Status | `code` | Quando |
|---|---|---|
| `400` | `invalid_request` | JSON malformado ou campo faltando |
| `400` | `invalid_username` | Usuário fora das regras |
| `400` | `invalid_password` | Senha fora das regras |
| `409` | `username_taken` | Usuário já existe |
| `500` | `internal` | Erro interno (detalhes só no log do `sun`) |

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

Mensagens (e frames) acima de **64 KiB** (`sun::MAX_MESSAGE_SIZE`) fazem o servidor encerrar a conexão.

## Comandos Tauri (front → core Rust do `moon`)

Gerados em `moon/src/bindings.ts` como `commands.*`.

| Comando | Retorno | Descrição |
|---|---|---|
| `appVersion()` | `string` | Versão do app |
