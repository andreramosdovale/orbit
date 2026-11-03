# 001. Cadastro de usuários (auto-cadastro)

- **Status:** Em andamento (falta só a verificação manual)
- **Fase:** 1 (ver `docs/roadmap.md`)

## Objetivo

Um amigo que já está na rede do host (Tailscale) abre o `moon`, preenche um formulário mínimo (endereço do servidor, usuário e senha) e cria a própria conta no `sun`. O host não participa: não há convite nem cadastro feito pelo admin. Estar na rede do Tailscale do host é o que dá acesso (ADR 0003).

## Fora de escopo

- Login, JWT e `/ws` autenticado (próxima spec). Depois de cadastrar, o app só mostra "conta criada".
- Código de convite e qualquer papel de admin. **Isto substitui o item "Código de convite gerado pelo host" do roadmap** (ver ADR 0007, criado nesta spec).
- Nome de exibição, avatar, e-mail, recuperação ou troca de senha.
- Testes de front com Vitest (fica para a spec de login).
- Rate limiting de cadastro: a rede já é restrita aos amigos.

## Mudanças

### Protocolo (`shared` + `docs/protocol.md`)

Tipos novos em `shared/src/lib.rs`, com derive `specta::Type` (vão para `bindings.ts`):

```rust
/// Corpo de `POST /auth/register`.
pub struct RegisterRequest { pub username: String, pub password: String }

/// Usuário público (nunca contém senha ou hash).
pub struct User { pub id: u32, pub username: String }

/// Corpo de toda resposta de erro da API REST.
pub struct ApiErrorBody { pub code: ApiErrorCode, pub message: String }

#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode { InvalidRequest, InvalidUsername, InvalidPassword, UsernameTaken, Internal }
```

| Método | Rota | Corpo | Sucesso | Erros |
|---|---|---|---|---|
| `POST` | `/auth/register` | `RegisterRequest` | `201` + `User` | `400 invalid_request` / `400 invalid_username` / `400 invalid_password` / `409 username_taken` / `500 internal` |

Regras de validação (no `sun`, repetidas no form só como ajuda):

- **username:** 3 a 32 caracteres, só `a-z`, `0-9` e `_`. É normalizado para minúsculas antes de validar e salvar.
- **password:** 8 a 128 caracteres. Sem outras regras de complexidade.

`message` é um texto em português pronto para mostrar ao usuário. Em `500`, ela é genérica e a causa só vai para o log.

### Dados (SQLite / migrations)

- `sqlx` com SQLite, migrations em `sun/migrations/`, aplicadas automaticamente quando o `sun` sobe (`sqlx::migrate!`).
- Banco em `orbit.db` ao lado do executável. Pode ser trocado com a variável nova `SUN_DB` (caminho do arquivo).
- Migration `0001_users.sql`:

```sql
CREATE TABLE users (
    id            INTEGER PRIMARY KEY,
    username      TEXT    NOT NULL UNIQUE,
    password_hash TEXT    NOT NULL,
    created_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
```

### `sun`

- `AppState { db: SqlitePool }` passado via `Router::with_state`. `sun::app(state)` e `sun::serve(listener, state, shutdown)` passam a receber o estado, e os testes usam SQLite em memória.
- Módulo `auth/` com `routes()` e o handler `register`.
- `ApiError` (`thiserror` + `IntoResponse`) gerando `ApiErrorBody`. Violação do `UNIQUE` vira `409`, e o resto vira `500` com log.
- Hash com **argon2id** (parâmetros padrão do crate `argon2`) dentro de `tokio::task::spawn_blocking`, para não bloquear o runtime.
- **CORS** (`tower-http`) liberando só as origens do app: `http://tauri.localhost` (build), `tauri://localhost` e `http://localhost:1420` (dev). O webview faz `fetch` direto no `sun`, como na arquitetura.
- Consultas com `sqlx::query`/`query_as` + `.bind(...)` (parametrizadas, verificadas em tempo de execução). As macros `query!` exigiriam `DATABASE_URL` ou `cargo sqlx prepare` na CI. O `sun/CLAUDE.md` será ajustado para isso.
- Dependências novas: `sqlx` (`sqlite`, `runtime-tokio`, `migrate`), `argon2`, `thiserror`, `tower-http` (`cors`).

### `moon` (UI e core Rust)

- `RegisterRequest`, `User` e `ApiErrorBody` registrados no `specta_builder`, que os exporta para `bindings.ts`.
- Tela única `Register`, sem estilização além do básico:
  - campo **Servidor** (ex.: `http://100.x.y.z:7700`), salvo em `localStorage` para não redigitar;
  - campos **Usuário**, **Senha** e **Confirmar senha** (a confirmação só é checada no front);
  - botão **Criar conta**, desabilitado enquanto envia;
  - mensagem de sucesso ("Conta `fulano` criada") ou o `message` do erro do servidor. Falha de rede mostra "Não foi possível conectar ao servidor".
- `src/features/auth/api.ts` com `register(server, req): Promise<User>`, tipado pelos `bindings.ts`. Não monta JSON de resposta à mão.

## Critérios de aceite

- [x] `shared`: testes de formato do fio para `RegisterRequest`, `User` e `ApiErrorBody` passam.
- [x] `sun/tests/auth.rs::register_creates_user`: `201` com `{ id, username }` e o usuário existe no banco.
- [x] `sun/tests/auth.rs::register_normalizes_username`: `"Fulano"` é salvo e retornado como `"fulano"`.
- [x] `sun/tests/auth.rs::register_duplicate_returns_409`: segundo cadastro de `fulano` (também como `FULANO`) dá `409 username_taken`.
- [x] `sun/tests/auth.rs::register_invalid_username_returns_400`: curto, longo e com caractere inválido dão `400 invalid_username`.
- [x] `sun/tests/auth.rs::register_invalid_password_returns_400`: senha com 7 e com 129 caracteres dá `400 invalid_password`.
- [x] `sun/tests/auth.rs::register_stores_argon2_hash`: o `password_hash` salvo começa com `$argon2id$`, valida com a senha original e não contém a senha.
- [x] `sun/tests/auth.rs::register_cors_allows_app_origin`: preflight de `http://tauri.localhost` recebe `access-control-allow-origin`, e uma origem qualquer não.
- [x] `moon/src/bindings.ts` contém `RegisterRequest`, `User` e `ApiErrorBody`.
- [ ] Manual: com `cargo run -p sun` e `npm run tauri dev`, criar uma conta pelo form mostra sucesso, e repetir o mesmo usuário mostra "nome de usuário já existe".
- [x] `/verify` verde.

## Plano de implementação

Passos pequenos, cada um terminando com `/verify` verde.

1. **shared:** tipos novos + testes de formato + `docs/protocol.md`.
2. **sun, banco:** `sqlx`, migration `0001_users.sql`, `AppState`, `SUN_DB`, `app(state)`/`serve(...)` e helper de teste com SQLite em memória (os testes atuais continuam passando).
3. **sun, cadastro:** módulo `auth/`, validação, `ApiError`, argon2 em `spawn_blocking` e testes de `sun/tests/auth.rs`.
4. **sun, CORS:** `tower-http` + teste de preflight.
5. **moon:** tipos no `specta_builder`, `src/api.ts` e tela `Register`.
6. **docs:** ADR 0007 (auto-cadastro substitui convite), roadmap, `docs/architecture.md`, `sun/CLAUDE.md` (`SUN_DB`, consultas `sqlx`), `moon/CLAUDE.md` e esta spec como `Concluída`.

## Decisões

Questões resolvidas com o usuário em 2026-11-03:

1. **Convite:** removido do roadmap. Acesso controlado pelo compartilhamento do Tailscale ([ADR 0007](../decisions/0007-auto-cadastro-sem-convite.md)).
2. **Endereço do servidor:** campo "Servidor" no form, salvo em `localStorage`.
3. **SQL:** `sqlx::query` em runtime com `.bind(...)`, sem macros `query!`.

Extras feitos durante a implementação: código `invalid_request` para JSON malformado (teste `register_malformed_body_returns_400`), teste `register_response_never_contains_password` e limite de 16 KiB no corpo HTTP.

## Questões em aberto (originais)

1. **Convite:** confirmar que o convite sai do roadmap de vez (qualquer um na rede do Tailscale pode se cadastrar). Se o host compartilhar a máquina com alguém por engano, essa pessoa também consegue criar conta.
2. **Endereço do servidor no form:** proposto um campo "Servidor" salvo em `localStorage`. A alternativa seria fixar por config de build, mas cada amigo usa o IP Tailscale do host.
3. **`query!` vs `query`:** proposto `query` em tempo de execução para não exigir `sqlx-cli`/`.sqlx` na CI. A troca é perder a checagem do SQL na compilação, compensada pelos testes.
