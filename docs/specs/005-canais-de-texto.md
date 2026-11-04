# 005. Canais de texto (criar e listar)

- **Status:** Concluída
- **Fase:** 2 (ver `docs/roadmap.md`)

## Objetivo

Depois de entrar, o usuário vê a lista de canais de texto do servidor, seleciona um e pode criar outro. Um servidor novo já vem com o canal `geral`. Esta spec também cria a autenticação das rotas REST, que as próximas specs vão reutilizar.

## Fora de escopo

- **Mensagens e histórico**: próximos itens do roadmap. Selecionar um canal só mostra o nome dele e um aviso.
- **Atualização em tempo real da lista** (ver Questões em aberto (originais), item 4).
- **Renomear ou apagar canais**: o roadmap só pede criar e listar.
- **Permissões por canal, canais privados e papel de admin** (ADR 0007).
- **Presença**: item separado do roadmap.

## Mudanças

### Protocolo (`shared` + `docs/protocol.md`)

Tipos novos em `shared/src/lib.rs`:

```rust
/// Canal de texto.
pub struct TextChannel { pub id: u32, pub name: String }

/// Corpo de `POST /channels`.
pub struct CreateChannelRequest { pub name: String }
```

O tipo se chama `TextChannel`, e não `Channel` como na versão aprovada desta spec, porque o `tauri-specta` reserva `Channel` para o próprio runtime e recusa gerar os bindings. O JSON não muda.

Códigos novos em `ApiErrorCode`: `Unauthorized` (`"unauthorized"`), `InvalidChannelName` (`"invalid_channel_name"`) e `ChannelNameTaken` (`"channel_name_taken"`).

| Método | Rota | Auth | Corpo | Sucesso | Erros |
|---|---|---|---|---|---|
| `GET` | `/channels` | `Bearer` | — | `200` + `TextChannel[]`, na ordem de criação | `401 unauthorized` / `500 internal` |
| `POST` | `/channels` | `Bearer` | `CreateChannelRequest` | `201` + `TextChannel` | `400 invalid_request` / `400 invalid_channel_name` / `401 unauthorized` / `409 channel_name_taken` / `500 internal` |

**Regras do nome do canal** (ver Questões em aberto (originais), item 2):

- Espaços nas pontas são removidos e o nome é convertido para minúsculas, então `Geral` e `geral` são o mesmo canal.
- De 1 a 32 caracteres.
- Só letras (com acento), números, `-` e `_`. Espaço no meio é recusado.

**Autenticação REST:** token ausente, malformado, inválido ou expirado dão a mesma resposta: `401 unauthorized`.

### Dados (SQLite / migrations)

Migration `0003_channels.sql`:

```sql
CREATE TABLE channels (
    id         INTEGER PRIMARY KEY,
    name       TEXT    NOT NULL UNIQUE,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

INSERT INTO channels (name) VALUES ('geral');
```

### `sun`

- `auth/extractor.rs`: extrator `AuthUser(User)`, que lê `Authorization: Bearer <token>` e valida com `JwtKeys`. Uma rota fica protegida ao recebê-lo como parâmetro.
- `chat/mod.rs`: `routes()`, com handlers finos.
- `chat/channels.rs`: `list_channels` e `create_channel`, com validação, normalização e o erro de domínio `ChannelError`.
- `ApiError` ganha `Unauthorized`, `InvalidChannelName` e `ChannelNameTaken`.
- **CORS:** `allow_headers` passa a incluir `Authorization`. Sem isso, o webview bloqueia toda chamada autenticada, embora os testes sem navegador continuem passando.
- O token nunca é logado. Sem dependências novas.

### `moon` (UI e core Rust)

- `TextChannel` e `CreateChannelRequest` registrados no `specta_builder`.
- `ApiError` e o código de requisição saem de `features/auth/api.ts` para `src/api/http.ts`, com suporte a token, para `auth` e `chat` usarem o mesmo código.
- `features/chat/api.ts`: `listChannels(server, token)` e `createChannel(server, token, req)`.
- `features/chat/ChatLayout.tsx` substitui `Connected.tsx` depois de entrar:
  - barra lateral com a lista de canais, o usuário conectado, **Sair** e o formulário **Novo canal**;
  - o primeiro canal já vem selecionado, e clicar em outro troca a seleção;
  - a área principal mostra `# nome-do-canal` e o aviso "As mensagens chegam em breve.";
  - criar um canal adiciona ele à lista e o seleciona; o `message` do erro aparece junto do formulário;
  - falha ao carregar a lista mostra o erro e **Tentar de novo**.
- **Sessão expirada:** um `401` em qualquer chamada faz o mesmo que `auth_error`: apaga o token e volta ao login com "Sua sessão expirou. Entre novamente."

## Critérios de aceite

- [x] `shared`: os testes de formato do fio para `TextChannel`, `CreateChannelRequest` e os três códigos novos de `ApiErrorCode` passam.
- [x] `sun/tests/channels.rs::list_requires_token`: sem token, com token inválido e com token expirado, dá `401 unauthorized`.
- [x] `sun/tests/channels.rs::create_requires_token`: sem token dá `401 unauthorized` e nenhum canal é criado.
- [x] `sun/tests/channels.rs::new_server_has_general_channel`: em um banco novo, a lista contém só `geral`.
- [x] `sun/tests/channels.rs::create_returns_channel_and_lists_it`: criar dá `201`, e o canal aparece depois de `geral`, na ordem de criação.
- [x] `sun/tests/channels.rs::create_normalizes_name`: ` Música ` é salvo como `música`.
- [x] `sun/tests/channels.rs::create_duplicate_returns_409`: repetir um nome, também com outra capitalização, dá `409 channel_name_taken`.
- [x] `sun/tests/channels.rs::create_invalid_name_returns_400`: vazio, só espaços, 33 caracteres, espaço no meio e símbolo dão `400 invalid_channel_name`.
- [x] `sun/tests/channels.rs::create_malformed_body_returns_400`: JSON inválido dá `400 invalid_request`.
- [x] `sun/tests/channels.rs::cors_allows_authorization_header`: o preflight de uma origem do `moon` libera `Authorization`.
- [x] `sun/tests/channels.rs::channels_persist`: um canal criado continua lá ao reabrir o mesmo arquivo de banco, e `geral` não é duplicado.
- [x] `moon/src/bindings.ts` contém `TextChannel`, `CreateChannelRequest` e os códigos novos.
- [x] `moon/src/features/chat/api.test.ts`: as chamadas enviam `Authorization: Bearer <token>`, e um `401` rejeita com `ApiError` de código `unauthorized`.
- [x] `moon/src/features/chat/ChatLayout.test.tsx`:
  - lista os canais e seleciona o primeiro;
  - clicar em outro canal troca a seleção;
  - criar um canal adiciona e seleciona;
  - o erro de nome repetido aparece junto do formulário;
  - falha ao carregar mostra **Tentar de novo**, que carrega de novo ao ser clicado.
- [x] `moon/src/App.test.tsx`: um `401` ao carregar os canais apaga o token e volta ao login com o aviso de sessão expirada.
- [x] **Verificação manual** na janela real com um `sun` real: entrar, ver `geral`, criar um canal, trocar de canal, fechar e reabrir o app (o canal continua lá). É o que confirma o CORS no webview de verdade.
- [x] `/verify` verde.

## Plano de implementação

Passos pequenos, cada um terminando com `/verify` verde.

1. **shared:** tipos, códigos e testes de formato + `docs/protocol.md` + `specta_builder` e regeneração do `bindings.ts`.
2. **sun, autenticação REST:** `AuthUser`, `ApiError::Unauthorized` e CORS, com testes.
3. **sun, canais:** migration, `chat/`, rotas e `sun/tests/channels.rs`.
4. **moon, HTTP:** `src/api/http.ts` e `features/chat/api.ts`, com testes, sem mudar o comportamento do login e do cadastro.
5. **moon, telas:** `ChatLayout.tsx`, sessão expirada em `App.tsx` e CSS, com testes.
6. **Verificação manual** com o `sun` real.
7. **docs:** `docs/architecture.md`, `sun/CLAUDE.md`, `moon/CLAUDE.md`, roadmap (marcar "Canais de texto"), `docs/specs/README.md` e esta spec como `Concluída`.

## Verificação manual

Feita em 2026-11-03 na janela real do app (WebView2), dirigida por script via depuração remota, contra um `sun` real com banco temporário. Passaram: `geral` listado e selecionado, criar ` Música ` (vira `música` e fica selecionado), nome repetido mostra o erro, trocar de canal e fechar e reabrir o app com o canal ainda lá. As chamadas com `Authorization` passaram pelo CORS do webview de verdade, a partir da origem `http://localhost:1420`. A origem do build de release (`tauri.localhost`) só é coberta pelo teste de preflight.

## Decisões

Questões resolvidas com o usuário em 2026-11-03: as quatro propostas abaixo foram aceitas.

## Questões em aberto (originais)

1. **Quem pode criar canais:** proposto **qualquer usuário logado**, porque não existe papel de admin (ADR 0007) e o grupo é de amigos. A troca é que qualquer um pode encher a lista, e como apagar está fora de escopo, um canal indesejado fica lá até existir essa função.
2. **Regras do nome:** proposto **aceitar acentos**, para nomes como `música` funcionarem em um grupo brasileiro, e recusar espaços, como no Discord. A alternativa de só ASCII é mais simples, mas forçaria `musica`.
3. **Canal inicial:** proposto **criar `geral` na migration**, para um servidor novo nunca abrir vazio.
4. **Outros usuários verem um canal novo:** proposto **só ao reabrir o app, por enquanto**. A atualização ao vivo depende do broadcast no WebSocket, que a próxima spec (mensagens em tempo real) vai criar; o evento `channel_created` entra lá.
