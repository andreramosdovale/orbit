# 003. WebSocket `/ws` autenticado

- **Status:** Concluída
- **Fase:** 1 (ver `docs/roadmap.md`)

## Objetivo

Só quem tem um JWT válido (spec 002) consegue usar o canal de tempo real. O `sun` passa a saber **qual usuário** está em cada conexão, o que o chat e a presença (Fase 2) vão precisar. O cliente também recebe uma resposta clara quando o token é recusado, para poder mandar o usuário de volta ao login.

## Fora de escopo

- **Tela de login e conexão no `moon`**: próximo item do roadmap. Aqui o `moon` só recebe os tipos novos em `bindings.ts`.
- **Middleware de `Authorization: Bearer` para rotas REST**: nenhuma rota REST exige token ainda. Entra na primeira rota que precisar (Fase 2).
- **Registro de conexões ativas, broadcast e presença**: Fase 2.
- **Reconexão automática**: é do cliente, junto com a tela de login.
- **Revogação e encerramento da conexão quando o token expira** (ver Questões em aberto (originais), item 2).
- **Checar no banco se o usuário ainda existe**: não há exclusão de contas, e apagar o `orbit.db` também apaga o segredo, o que invalida os tokens antigos.

## Mudanças

### Protocolo (`shared` + `docs/protocol.md`)

O token vai na **primeira mensagem** da conexão, e não no handshake (ver Questões em aberto (originais), item 1). Variantes novas:

```rust
pub enum ClientMessage {
    /// Primeira mensagem da conexão.
    Auth { token: String },
    Ping,
}

pub enum ServerMessage {
    /// Token aceito. A conexão está autenticada.
    AuthOk { user: User },
    /// Autenticação recusada. O servidor fecha a conexão logo depois.
    AuthError,
    Pong,
}
```

No fio:

```json
{ "type": "auth", "token": "<jwt>" }
{ "type": "auth_ok", "user": { "id": 1, "username": "fulano" } }
{ "type": "auth_error" }
```

**Fluxo:**

1. O cliente abre o `/ws`. O upgrade não exige nada.
2. O cliente envia `auth` em até **10 segundos**.
3. Token válido: o servidor responde `auth_ok` e a conexão passa a aceitar o resto do protocolo.
4. Qualquer outro caso: o servidor envia `auth_error` e fecha com o código **1008** (*policy violation*).

**Casos antes de autenticar** (todos terminam em `auth_error` + 1008):

- token inválido, adulterado ou expirado (não diz qual dos motivos);
- `auth` não chega dentro do prazo;
- `ping` ou outro tipo válido que não seja `auth`;
- JSON malformado ou frame binário.

Esse último caso é uma **exceção** à regra "mensagem inválida é ignorada": ela só vale depois de autenticar. Antes disso, uma conexão que não fala o protocolo não tem por que continuar aberta.

**Depois de autenticar:**

- Um segundo `auth` é ignorado com log `warn`. Não troca o usuário nem derruba a conexão.
- O resto continua igual: mensagem inválida é ignorada e o limite é de 64 KiB.

**Mudança incompatível:** um cliente que só manda `ping` deixa de funcionar. O `moon` ainda não usa o `/ws`, então nada quebra na prática.

**Textos a corrigir:** `docs/protocol.md` e o comentário de `LoginResponse.token` em `shared/src/lib.rs` dizem que o token vai em `Authorization: Bearer`. Isso continua valendo para o REST futuro, mas não para o `/ws`.

### Dados (SQLite / migrations)

Nenhuma. O usuário sai das claims do token, sem consulta ao banco.

### `sun`

- `ws.rs`:
  - nova função `authenticate(&mut socket, &state) -> Option<User>`, que roda antes do loop de mensagens;
  - o prazo vale para a etapa inteira, não para cada mensagem: frames de controle (ping/pong) não renovam os 10 segundos;
  - `handle` só entra no loop com um `User` em mãos, então não existe conexão não autenticada dentro do loop;
  - o `match` de `ClientMessage` no loop trata `Auth` com `warn` e segue em frente.
- `AppState` ganha `ws_auth_timeout: Duration`. O `main.rs` usa a constante pública `sun::WS_AUTH_TIMEOUT` (10 s), e os testes usam um valor curto para não esperar 10 segundos.
- A conversão de `Claims` em `User` faz o parse de `sub` para `u32`. Se falhar, é tratada como token inválido, nunca como panic.
- Logs: `info!(user_id, username, "conexão WebSocket autenticada")` no sucesso e `debug!` com o motivo na recusa. **O token nunca é logado.**
- Sem dependências novas.

### `moon` (UI e core Rust)

- `ClientMessage` e `ServerMessage` já estão no `specta_builder`; só regenerar `bindings.ts`. Nenhuma tela nova.

## Critérios de aceite

- [x] `shared`: os testes de formato do fio para `ClientMessage::Auth`, `ServerMessage::AuthOk` e `ServerMessage::AuthError` passam.
- [x] `sun/tests/ws_auth.rs::login_token_authenticates`: o token de um `POST /auth/login` real recebe `auth_ok` com o mesmo `user` do login, e o `ping` seguinte recebe `pong`.
- [x] `sun/tests/ws_auth.rs::invalid_token_is_rejected`: token inválido recebe `auth_error` e fechamento 1008.
- [x] `sun/tests/ws_auth.rs::expired_token_is_rejected`: token emitido com `iat` no passado, já vencido, recebe `auth_error` e fechamento 1008.
- [x] `sun/tests/ws_auth.rs::ping_before_auth_is_rejected`: `ping` como primeira mensagem recebe `auth_error` e fechamento 1008, sem `pong`.
- [x] `sun/tests/ws_auth.rs::malformed_message_before_auth_is_rejected`: JSON inválido como primeira mensagem recebe `auth_error` e fechamento 1008.
- [x] `sun/tests/ws_auth.rs::binary_message_before_auth_is_rejected`: frame binário como primeira mensagem recebe `auth_error` e fechamento 1008.
- [x] `sun/tests/ws_auth.rs::auth_timeout_closes_connection`: sem enviar nada, a conexão recebe `auth_error` e fechamento 1008 quando o prazo acaba.
- [x] `sun/tests/ws_auth.rs::second_auth_is_ignored`: depois de autenticar, um `auth` com token inválido não recebe resposta nem derruba a conexão, e o `ping` seguinte recebe `pong`.
- [x] `sun/tests/api.rs`: os testes de WebSocket que já existem (`ws_ping_gets_pong`, `ws_invalid_message_keeps_connection`, `ws_oversized_message_closes_connection`) passam usando uma conexão autenticada.
- [x] `moon/src/bindings.ts` contém `auth`, `auth_ok` e `auth_error`.
- [x] `/verify` verde.

## Plano de implementação

Passos pequenos, cada um terminando com `/verify` verde.

1. **ADR 0009:** autenticação do WebSocket pela primeira mensagem, com as alternativas descartadas.
2. **shared:** `Auth`, `AuthOk`, `AuthError` + testes de formato + `docs/protocol.md` (incluindo a correção sobre o `Bearer`) + regeneração do `bindings.ts`. O `match` do `ws.rs` ganha só o braço de `Auth` para continuar compilando.
3. **sun:** `authenticate`, `AppState.ws_auth_timeout`, `main.rs`, helpers de teste (`TestServer` expõe o estado e ganha `connect_ws_authed`), `sun/tests/ws_auth.rs` e migração dos testes de `sun/tests/api.rs`.
4. **docs:** `docs/architecture.md`, `sun/CLAUDE.md` (estrutura e a exceção da regra de mensagem inválida), roadmap (marcar "`/ws` autenticado"), `docs/specs/README.md`, `docs/decisions/README.md` e esta spec como `Concluída`.

## Decisões

Questões resolvidas com o usuário em 2026-11-03: as três propostas abaixo foram aceitas e registradas no [ADR 0009](../decisions/0009-autenticacao-do-ws-na-primeira-mensagem.md).

## Questões em aberto (originais)

1. **Como o token chega ao `/ws`:** proposto **na primeira mensagem**. A API `WebSocket` do webview não permite definir o cabeçalho `Authorization`, então o que a spec 002 previa não funciona aqui.
   - **Query string (`/ws?token=...`):** mais simples, e dá para recusar com `401` antes do upgrade. Mas o token fica na URL, e o front não consegue ler o status de um handshake recusado, ou seja, não distingue "token inválido" de "servidor fora do ar".
   - **Subprotocolo (`Sec-WebSocket-Protocol`):** funciona, mas é um uso improvisado desse cabeçalho e tem o mesmo problema de o front não ler a recusa.
   - **`tauri-plugin-websocket` (conexão pelo core Rust):** permite cabeçalhos, mas adiciona uma dependência e tira o WebSocket do webview só por causa disso.
   - **Custo da proposta:** a conexão fica aberta sem autenticação por até 10 segundos, o que é aceitável numa rede restrita aos amigos (ADR 0003).
2. **Token que expira com a conexão aberta:** proposto **não derrubar**. O token só é checado ao conectar, e a próxima reconexão já exige um válido. Com validade de 30 dias e o PC do host sendo reiniciado de vez em quando, uma conexão durar até o token vencer é raro, e derrubar exigiria um timer em cada conexão.
3. **Prazo para autenticar:** proposto **10 segundos**: folgado para o cliente mandar uma mensagem logo depois de conectar, mesmo com rede ruim, e curto para não acumular conexões ociosas.
