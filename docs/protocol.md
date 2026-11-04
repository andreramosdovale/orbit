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
| `GET` | `/ws` | JWT na 1ª mensagem | upgrade WebSocket | Canal de tempo real ([spec 003](specs/003-ws-autenticado.md)) |
| `POST` | `/auth/register` | não | `201` + `User` | Auto-cadastro ([spec 001](specs/001-cadastro-de-usuarios.md)) |
| `POST` | `/auth/login` | não | `200` + `LoginResponse` | Login, devolve um JWT ([spec 002](specs/002-login-jwt.md)) |
| `GET` | `/channels` | `Bearer` | `200` + `TextChannel[]` | Lista os canais de texto ([spec 005](specs/005-canais-de-texto.md)) |
| `POST` | `/channels` | `Bearer` | `201` + `TextChannel` | Cria um canal de texto ([spec 005](specs/005-canais-de-texto.md)) |

Corpos HTTP acima de **16 KiB** (`sun::MAX_BODY_SIZE`) são recusados. O CORS só libera as origens do `moon`: as do build (`sun::ALLOWED_ORIGINS`) e `http://localhost` nas portas de dev (`sun::DEV_PORTS`, 1420 a 1439).

### `POST /auth/register`

Corpo `RegisterRequest`:

```json
{ "username": "fulano", "password": "senha-boa-123" }
```

- `username`: 3 a 32 caracteres `a-z`, `0-9` ou `_`. É convertido para minúsculas, então `Fulano` e `fulano` são o mesmo usuário.
- `password`: 8 a 128 caracteres. Guardada só como hash argon2id.

Sucesso: `201` com `User`, ou seja, `{ "id": 1, "username": "fulano" }`.

### `POST /auth/login`

Corpo `LoginRequest`:

```json
{ "username": "fulano", "password": "senha-boa-123" }
```

- `username` é convertido para minúsculas antes da busca. As regras de formato do cadastro não são aplicadas: um valor fora delas só não encontra conta.
- Usuário inexistente e senha errada dão a **mesma** resposta: `401 invalid_credentials`.

Sucesso: `200` com `LoginResponse`:

```json
{ "token": "<jwt>", "user": { "id": 1, "username": "fulano" } }
```

### Autenticação (JWT)

O token é um JWT HS256 ([ADR 0008](decisions/0008-jwt-com-segredo-no-sqlite.md)). No `/ws`, ele vai na mensagem `auth` ([ADR 0009](decisions/0009-autenticacao-do-ws-na-primeira-mensagem.md)), porque o WebSocket do webview não permite definir cabeçalhos. Nas rotas REST marcadas com `Bearer`, ele vai em `Authorization: Bearer <token>`; token ausente, malformado, inválido ou expirado dão a mesma resposta: `401 unauthorized`.

| Claim | Valor |
|---|---|
| `sub` | id do usuário, como string (`"1"`) |
| `username` | nome normalizado |
| `iat` | emissão (Unix, segundos) |
| `exp` | `iat` + 30 dias |

### `GET /channels`

Sucesso: `200` com os canais na ordem de criação. Um servidor novo já vem com `geral`.

```json
[{ "id": 1, "name": "geral" }]
```

### `POST /channels`

Corpo `CreateChannelRequest`:

```json
{ "name": "música" }
```

- `name`: espaços nas pontas são removidos e o nome é convertido para minúsculas, então `Geral` e `geral` são o mesmo canal. Depois disso, deve ter de 1 a 32 caracteres: letras (com acento), números, `-` ou `_`.
- Qualquer usuário logado pode criar canais.

Sucesso: `201` com `TextChannel`, ou seja, `{ "id": 2, "name": "música" }`.

### Erros da API REST

Toda resposta de erro tem corpo `ApiErrorBody`: `{ "code": "<ApiErrorCode>", "message": "<texto para o usuário>" }`.

| Status | `code` | Quando |
|---|---|---|
| `400` | `invalid_request` | JSON malformado ou campo faltando |
| `400` | `invalid_username` | Usuário fora das regras |
| `400` | `invalid_password` | Senha fora das regras |
| `400` | `invalid_channel_name` | Nome de canal fora das regras |
| `401` | `invalid_credentials` | Login com usuário ou senha incorretos |
| `401` | `unauthorized` | Token ausente, inválido ou expirado em rota protegida |
| `409` | `username_taken` | Usuário já existe |
| `409` | `channel_name_taken` | Já existe um canal com esse nome |
| `500` | `internal` | Erro interno (detalhes só no log do `sun`) |

## WebSocket (`/ws`)

### Autenticação

O upgrade não exige nada, mas o cliente precisa enviar `auth` como primeira mensagem, em até **10 segundos** (`sun::WS_AUTH_TIMEOUT`).

- **Token válido:** o servidor responde `auth_ok` e a conexão passa a aceitar o resto do protocolo.
- **Qualquer outro caso:** o servidor responde `auth_error` e fecha com o código **1008**, sem dizer o motivo. Isso inclui token inválido ou expirado, prazo estourado, outro tipo de mensagem, JSON malformado e frame binário.

O token só é checado aqui: uma conexão aberta não cai quando ele expira.

### Cliente → servidor (`ClientMessage`)

| `type` | Campos | Resposta |
|---|---|---|
| `auth` | `token` (JWT) | `auth_ok` ou `auth_error` + fechamento |
| `ping` | — | `pong` |
| `send_message` | `channel_id`, `content` | `new_message` para todos, ou `message_rejected` só para quem enviou |

### Servidor → cliente (`ServerMessage`)

| `type` | Campos | Quando |
|---|---|---|
| `auth_ok` | `user` (`User`) | Token aceito |
| `auth_error` | — | Autenticação recusada; a conexão fecha em seguida |
| `pong` | — | Resposta a `ping` |
| `new_message` | `message` (`ChatMessage`) | Mensagem nova, para todas as conexões, inclusive a de quem enviou |
| `message_rejected` | `reason` (`MessageRejectReason`) | Mensagem recusada, só para quem tentou enviar |
| `channel_created` | `channel` (`TextChannel`) | Canal novo, para todas as conexões |

Depois de autenticar, mensagens inválidas são ignoradas (com log `warn` no servidor) e a conexão continua aberta. Um segundo `auth` também é ignorado: não troca o usuário nem recebe resposta.

### Mensagens de texto

```json
{ "type": "send_message", "channel_id": 1, "content": "oi" }
{ "type": "new_message", "message": { "id": 1, "channel_id": 1, "author": { "id": 1, "username": "fulano" }, "content": "oi", "created_at": "2026-11-03T12:00:00.000Z" } }
{ "type": "message_rejected", "reason": "invalid_content" }
```

- `content`: espaços nas pontas são removidos, e o resultado deve ter de 1 a 2000 caracteres. Quebras de linha são permitidas.
- O autor vem sempre da conexão autenticada; o cliente não tem como informar outro.
- `reason`: `invalid_content`, `unknown_channel` ou `internal`.
- Todas as conexões autenticadas recebem as mensagens de todos os canais, na mesma ordem, e o `moon` as separa por `channel_id`. As mensagens ficam guardadas sem limite de tempo ([ADR 0011](decisions/0011-retencao-de-mensagens.md)).
- Quem fica mais de 256 mensagens atrasado (`sun::HUB_CAPACITY`) é desconectado com o código **1013**. Um envio que não termina em 10 segundos também encerra a conexão.

Mensagens (e frames) acima de **64 KiB** (`sun::MAX_MESSAGE_SIZE`) fazem o servidor encerrar a conexão, antes ou depois de autenticar.

## Comandos Tauri (front → core Rust do `moon`)

Gerados em `moon/src/bindings.ts` como `commands.*`.

| Comando | Retorno | Descrição |
|---|---|---|
| `appVersion()` | `string` | Versão do app |
