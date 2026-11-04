# 006. Mensagens em tempo real

- **Status:** Concluída
- **Fase:** 2 (ver `docs/roadmap.md`)

## Objetivo

Duas pessoas conectadas conversam em um canal: o que uma envia aparece na hora para as outras, e fica guardado no servidor. Um canal criado também aparece na hora para todos, sem reabrir o app, o que resolve a limitação deixada pela spec 005.

## Fora de escopo

- **Histórico**: próximo item do roadmap. Nesta spec, o app só mostra as mensagens recebidas desde que conectou; as anteriores ficam no banco, mas só aparecem com o histórico (ver Questões em aberto (originais), item 2).
- **Presença (online/offline)**: item separado do roadmap.
- **Editar ou apagar mensagens, anexos, menções, reações e contador de não lidas**.
- **Rate limiting**: mesma justificativa das specs 001 e 002, a rede já é restrita aos amigos.
- **Reconexão automática**: quando a conexão cai, continua valendo o **Tentar de novo** da spec 004.

## Mudanças

### Protocolo (`shared` + `docs/protocol.md`)

```rust
/// Mensagem de texto de um canal.
pub struct ChatMessage {
    pub id: u32,
    pub channel_id: u32,
    pub author: User,
    pub content: String,
    /// Horário do servidor, em UTC (ISO 8601).
    pub created_at: String,
}

pub enum MessageRejectReason { InvalidContent, UnknownChannel, Internal }

pub enum ClientMessage {
    Auth { token: String },
    Ping,
    /// Envia uma mensagem. O autor sai da conexão autenticada.
    SendMessage { channel_id: u32, content: String },
}

pub enum ServerMessage {
    AuthOk { user: User },
    AuthError,
    Pong,
    /// Mensagem nova, enviada a todas as conexões, inclusive a de quem enviou.
    NewMessage { message: ChatMessage },
    /// Mensagem recusada, enviada só a quem tentou enviar.
    MessageRejected { reason: MessageRejectReason },
    /// Canal novo, enviado a todas as conexões.
    ChannelCreated { channel: TextChannel },
}
```

No fio:

```json
{ "type": "send_message", "channel_id": 1, "content": "oi" }
{ "type": "new_message", "message": { "id": 1, "channel_id": 1, "author": { "id": 1, "username": "fulano" }, "content": "oi", "created_at": "2026-11-03T12:00:00.000Z" } }
{ "type": "message_rejected", "reason": "invalid_content" }
{ "type": "channel_created", "channel": { "id": 2, "name": "jogos" } }
```

**Regras:**

- `content`: espaços nas pontas são removidos, e o resultado deve ter de **1 a 2000 caracteres**. Quebras de linha são permitidas.
- O autor vem sempre da conexão autenticada; o cliente não tem como informar outro.
- Todas as conexões autenticadas recebem as mensagens de todos os canais. Os canais não têm permissões, e o `moon` separa as mensagens por `channel_id`.
- Todos recebem as mensagens na mesma ordem.
- Antes de autenticar, `send_message` recebe `auth_error` + fechamento, como qualquer outra mensagem (ADR 0009).
- **Cliente lento:** quem fica muito para trás no fluxo de mensagens é desconectado com o código **1013** (*try again later*) e usa **Tentar de novo**. Isso evita que um cliente travado segure memória do servidor.

### Dados (SQLite / migrations)

Migration `0004_messages.sql`:

```sql
CREATE TABLE messages (
    id         INTEGER PRIMARY KEY,
    channel_id INTEGER NOT NULL REFERENCES channels (id),
    author_id  INTEGER NOT NULL REFERENCES users (id),
    content    TEXT    NOT NULL,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX messages_channel_id ON messages (channel_id, id);
```

O índice já atende o histórico paginado da próxima spec.

**Retenção:** o ADR 0004 deixou isso para a Fase 2. Proposto guardar para sempre (ver Questões em aberto (originais), item 1), registrado no ADR 0011.

### `sun`

- `AppState` ganha `hub: broadcast::Sender<ServerMessage>`, com capacidade para 256 mensagens, e um construtor `AppState::new(db, jwt)`, usado pelo `main.rs` e pelos testes.
- `chat/messages.rs`: `send_message(db, author, channel_id, content)`, com validação, checagem do canal, `INSERT` e o erro de domínio `MessageError`.
- `ws.rs`:
  - a conexão assina o `hub` **antes** de enviar `auth_ok`, para não perder nada que chegue logo depois;
  - o loop passa a esperar duas coisas ao mesmo tempo (`tokio::select!`): mensagens do cliente e do `hub`;
  - `send_message` grava no banco e publica no `hub`. Quem enviou recebe a mensagem pelo mesmo caminho dos outros, então a ordem é igual para todos;
  - quem atrasar além da capacidade do `hub` é desconectado com o código 1013;
  - cada envio ao cliente tem prazo de 10 segundos; estourou, a conexão fecha.
- `POST /channels` publica `channel_created` no `hub`.
- Não ter ninguém conectado não é erro ao publicar: a mensagem já está no banco.
- O conteúdo das mensagens não vai para o log. Sem dependências novas.

### `moon` (UI e core Rust)

- `ChatMessage` e `MessageRejectReason` entram em `bindings.ts`.
- `features/auth/session.ts`: `Session` ganha `send(message)` e `subscribe(handler)`. O que chegar antes da primeira assinatura fica guardado e é entregue a ela, para não perder o intervalo entre o `auth_ok` e a tela montar.
- `App.tsx`: passa a `Session` para o `ChatLayout`.
- `features/chat/ChatLayout.tsx`:
  - guarda as mensagens por canal, incluindo as dos canais não selecionados;
  - `channel_created` adiciona o canal à lista sem trocar a seleção, ignorando um canal já listado, porque quem criou recebe o canal pelo REST e pelo WebSocket.
- `features/chat/MessageList.tsx`: autor, horário local (`HH:MM`) e texto, preservando as quebras de linha. Rola até o fim quando chega mensagem.
- `features/chat/Composer.tsx`:
  - **Enter** envia e **Shift+Enter** quebra a linha;
  - não envia texto vazio nem acima de 2000 caracteres;
  - limpa o campo ao enviar. A mensagem só aparece quando volta do servidor, sem exibição otimista;
  - `message_rejected` mostra o erro junto do campo.
- O aviso "As mensagens chegam em breve." sai da tela.

### Fora do plano original: várias instâncias em dev

Pedido do usuário durante a implementação, para abrir mais de uma janela do app e testar a conversa com duas contas:

- `npm run dev:app` (`moon/scripts/dev.mjs`) abre o app na primeira porta livre entre 1420 e 1439, com o `devUrl` do Tauri acompanhando a porta e um `target/dev-<porta>` próprio da segunda porta em diante.
- `npm run tauri dev` continua igual, na porta 1420.
- O CORS do `sun` passa a liberar `http://localhost` nessa faixa de portas (`sun::DEV_PORTS`).

## Critérios de aceite

- [x] `shared`: os testes de formato do fio para `ChatMessage`, `MessageRejectReason`, `SendMessage`, `NewMessage`, `MessageRejected` e `ChannelCreated` passam.
- [x] `sun/tests/messages.rs::message_reaches_all_connections`: com dois clientes, o que um envia chega igual aos dois, com o autor certo.
- [x] `sun/tests/messages.rs::message_is_persisted`: a mensagem enviada está no banco, com canal, autor e conteúdo.
- [x] `sun/tests/messages.rs::messages_keep_order`: várias mensagens de um cliente chegam ao outro na ordem, com ids crescentes.
- [x] `sun/tests/messages.rs::content_is_trimmed`: `"  oi  "` é salvo e entregue como `"oi"`.
- [x] `sun/tests/messages.rs::invalid_content_is_rejected`: vazio, só espaços e 2001 caracteres recebem `message_rejected` com `invalid_content`; o outro cliente não recebe nada e nada é gravado. 2000 caracteres são aceitos.
- [x] `sun/tests/messages.rs::unknown_channel_is_rejected`: um canal inexistente recebe `message_rejected` com `unknown_channel`, e a conexão continua.
- [x] `sun/tests/messages.rs::send_before_auth_is_rejected`: `send_message` como primeira mensagem recebe `auth_error` + 1008, e nada é gravado.
- [x] `sun/tests/messages.rs::channel_created_reaches_connections`: um `POST /channels` faz as conexões abertas receberem `channel_created`.
- [x] `sun/tests/messages.rs::lagging_client_is_disconnected`: quem atrasa além da capacidade do `hub` é desconectado com o código 1013.
- [x] `moon/src/bindings.ts` contém os tipos e as mensagens novos.
- [x] `moon/src/features/auth/session.test.ts`:
  - `send` envia o JSON pelo socket;
  - `subscribe` entrega as mensagens recebidas, e cancelar a assinatura interrompe a entrega;
  - o que chega antes da primeira assinatura é entregue a ela.
- [x] `moon/src/features/chat/ChatLayout.test.tsx`:
  - Enter envia `send_message` com o canal selecionado;
  - `message_rejected` mostra o erro;
  - a mensagem do canal selecionado aparece com autor e texto;
  - a mensagem de outro canal só aparece ao selecionar esse canal;
  - `channel_created` adiciona o canal sem trocar a seleção;
  - criar um canal recebendo o REST e o `channel_created` não duplica a entrada.
- [x] `moon/src/features/chat/Composer.test.tsx`:
  - Enter envia e limpa o campo;
  - Shift+Enter não envia;
  - texto vazio e acima de 2000 caracteres não são enviados.
- [x] **Verificação manual** com um `sun` real e **duas janelas** do app, com contas diferentes: uma envia e a outra recebe, nos dois sentidos; uma cria um canal e ele aparece na outra.
- [x] `sun/tests/auth.rs::register_cors_allows_app_origin`: as portas 1420 e 1439 de `localhost` são liberadas; 1419, 1440 e `localhost:1420.evil.example` não.
- [x] **Verificação manual** de `npm run dev:app`: com a porta 1420 ocupada, o app abre na seguinte, e o login funciona nessa origem.
- [x] `/verify` verde.

## Plano de implementação

Passos pequenos, cada um terminando com `/verify` verde.

1. **ADR 0011:** retenção de mensagens.
2. **shared:** tipos, mensagens e testes de formato + `docs/protocol.md` + `specta_builder` e regeneração do `bindings.ts`.
3. **sun, mensagens:** migration, `chat/messages.rs` e `AppState::new`, com testes de unidade.
4. **sun, tempo real:** `hub`, `select!` em `ws.rs`, `channel_created` e `sun/tests/messages.rs`.
5. **moon, sessão:** `send` e `subscribe` em `session.ts`, com testes.
6. **moon, telas:** `MessageList.tsx`, `Composer.tsx`, `ChatLayout.tsx` e CSS, com testes.
7. **Verificação manual** com duas janelas.
8. **docs:** `docs/architecture.md`, `sun/CLAUDE.md`, `moon/CLAUDE.md`, roadmap (marcar "Mensagens em tempo real"), `docs/specs/README.md` e esta spec como `Concluída`.

## Verificação manual

Feita em 2026-11-03 em duas janelas reais do app (WebView2), dirigidas por script via depuração remota, contra um `sun` real com banco temporário e com contas diferentes. A primeira janela usou a origem `localhost:1420` e a segunda, `localhost:1421`. Passaram:

- uma janela envia e a outra recebe, nos dois sentidos e na mesma ordem;
- Shift+Enter quebra a linha, e HTML aparece como texto;
- um canal criado aparece na outra janela sem trocar a seleção dela;
- a mensagem de outro canal só aparece ao selecionar esse canal;
- com 40 mensagens, só a lista rola e fica no fim;
- o conteúdo das mensagens não aparece no log do `sun`.

`npm run dev:app` foi rodado de verdade com a porta 1420 ocupada: escolheu a 1421, compilou em cerca de 1 minuto, em uma pasta `target/dev-1421` de 3,1 GB, e abriu o app.

Não cobre o build de release nem a conexão por outro PC via Tailscale, que ficam para a Fase 6.

## Decisões

Questões resolvidas com o usuário em 2026-11-03: as quatro propostas abaixo foram aceitas, e a de retenção foi registrada no [ADR 0011](../decisions/0011-retencao-de-mensagens.md).

## Questões em aberto (originais)

1. **Retenção:** proposto **guardar para sempre, sem limpeza automática**. Texto de um grupo de 10 pessoas ocupa pouco: mesmo 1.000 mensagens por dia dão por volta de 100 MB por ano. Apagar depois de um prazo reduziria o que fica no PC do host, mas o grupo perderia conversas antigas, e sempre dá para adicionar um limite depois.
2. **Histórico em uma spec separada:** proposto **manter separado**, como no roadmap, para um PR menor. A consequência é que, até o próximo item, reabrir o app mostra os canais vazios, embora as mensagens estejam guardadas. Juntar as duas evitaria esse intervalo, mas dobraria o tamanho da entrega.
3. **Limite de tamanho:** proposto **2000 caracteres**, como no Discord. Os 64 KiB do WebSocket comportam isso com folga.
4. **Sem exibição otimista:** proposto **a mensagem só aparecer quando volta do servidor**. É mais simples e garante a mesma ordem para todos. Na rede local ou no Tailscale a demora é quase imperceptível, mas em uma conexão ruim o texto some do campo um instante antes de aparecer na conversa.
