# sun: servidor

Servidor do Orbit que roda no PC do host. axum + tokio + sqlx (SQLite). Vai ganhar emissão de tokens LiveKit na Fase 3.

## Estrutura

```
src/
├── main.rs          # só lê e valida config (env), abre o banco, carrega o segredo do JWT, inicia tracing e chama sun::serve(). Sem lógica.
├── lib.rs           # AppState; app(state): Router + CORS + limite de corpo; serve(): ConnectInfo e shutdown gracioso
├── db.rs            # connect(path) / connect_in_memory(): pool SQLite + migrations
├── error.rs         # ApiError → ApiErrorBody (IntoResponse)
├── auth/
│   ├── mod.rs       # routes(): POST /auth/register e /auth/login (handlers finos)
│   ├── register.rs  # validação, hash argon2id, INSERT. Erro de domínio: RegisterError
│   ├── login.rs     # confere senha (argon2) e emite o token. Erro de domínio: LoginError
│   └── jwt.rs       # JwtKeys: segredo no SQLite (tabela secrets), issue/verify (ADR 0008)
└── ws.rs            # WebSocket /ws: recebe ClientMessage, responde ServerMessage
migrations/          # SQL versionado, aplicado quando o sun sobe (sqlx::migrate!)
tests/
├── api.rs           # /health e WebSocket
├── auth.rs          # cadastro, login e persistência do segredo do JWT
└── common/mod.rs    # helpers: state() com SQLite em memória, post_json, TestServer, send/recv
```

Conforme o servidor crescer, organize **por domínio** (`auth/`, `chat/`, `voice/`), cada um com seu `mod.rs` expondo as rotas (`fn routes() -> Router<AppState>`) e montado em `lib.rs`.

## Padrões

- **Testável sem rede:** toda rota HTTP deve ser testável via `sun::app(state).oneshot(request)` (`tower::ServiceExt`), com `common::state()` (SQLite em memória). Só o WebSocket precisa de porta real (`127.0.0.1:0`). Siga os exemplos em `tests/auth.rs` e `tests/api.rs`.
- **Estado compartilhado** (pool do banco, config, conexões WS): campos da struct `AppState` (`Clone`), passada via `Router::with_state`.
- **Banco:** mudança de schema = arquivo novo em `migrations/` (`NNNN_descricao.sql`). Nunca edite uma migration já commitada.
- **Mensagens do WebSocket:** sempre os tipos de `orbit_shared`. Nunca monte JSON à mão. Mensagem inválida gera `tracing::warn!` e é ignorada, sem derrubar a conexão.
- **Config via variáveis de ambiente**, com padrão sensato em `main.rs` (`SUN_ADDR`, padrão `0.0.0.0:7700`). Documente cada variável nova aqui.

## Tratamento de erros

O `sun` é um servidor de longa duração: um panic na inicialização derruba o servidor de todo mundo, e um panic dentro de uma task some sem deixar rastro útil. **Todo erro deve ser tratado ou propagado, nunca ignorado nem transformado em panic.**

### Proibido em `src/`

- `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!`.
- Indexar dados vindos de fora (`vec[i]`, `&s[a..b]`, `map["chave"]`). Use `.get(..)` e trate o `None`.
- Conversões numéricas com `as` que podem truncar. Use `TryFrom`/`try_into()` com `?`.
- Descartar um `Result` em silêncio (`let _ = ...`, `.ok();` solto). Se ignorar for o certo, logue ou explique em comentário o porquê.

`unwrap_or`, `unwrap_or_else` e `unwrap_or_default` são permitidos **quando o valor padrão é a resposta correta** (ex.: variável de ambiente ausente). Não use para esconder um erro real: valor de config presente mas inválido deve falhar com mensagem clara, não cair no padrão.

Se uma invariante realmente não pode falhar, prefira reestruturar o código para o compilador provar isso (tipos, `let-else`, `match` exaustivo). Se não der, `expect("invariante: <por que nunca falha>")` com comentário, e isso deve ser exceção apontada na revisão.

### Como tratar, por camada

- **`main.rs` e inicialização:** `anyhow::Result<()>` com `?`. Toda falha de I/O ou parse ganha contexto do que estava sendo feito: `.context("abrindo o banco")` / `.with_context(|| format!("não foi possível abrir {addr}"))`.
- **Lógica de domínio (`auth/`, `chat/`...):** erros tipados com `thiserror` (`enum ChatError`), um por domínio. Nada de `anyhow` em API pública de módulo: quem chama precisa saber quais casos tratar.
- **Handlers HTTP:** retornam `Result<T, ApiError>`. `ApiError` implementa `IntoResponse` e `From<ChatError>` etc., mapeando:
  - entrada inválida → `400`, não autenticado → `401`, sem permissão → `403`, não encontrado → `404`, conflito → `409`;
  - erro interno → `500` com mensagem **genérica**. A causa real vai para `tracing::error!(error = ?err, ...)`, nunca para o corpo da resposta.
- **WebSocket:** mensagem inválida do cliente → `tracing::warn!` e continua. Erro de envio/recebimento no socket → `tracing::debug!` e encerra o loop da conexão. Nunca derrube o servidor por causa de um cliente.
- **Tasks (`tokio::spawn`):** a função da task retorna `Result` e o erro é logado dentro dela; um `JoinHandle` descartado engole o erro.

### Em testes

O clippy permite `unwrap`/`expect` em testes (`clippy.toml`), mas falhas precisam ser legíveis:

- Prefira funções de teste `async fn x() -> anyhow::Result<()>` usando `?`.
- Quando `?` não couber, use `expect("o que deveria ter acontecido")` em vez de `unwrap()`.
- Setup repetido (subir servidor, conectar WS) vai para helpers em `tests/common/mod.rs`, escritos uma vez, com contexto nos erros.

## Boas práticas de async e servidor

- **Não bloqueie o runtime:** nada de I/O síncrono (`std::fs`, `std::thread::sleep`) ou CPU pesado em `async fn`. Use `tokio::fs`, `tokio::time` ou `tokio::task::spawn_blocking`.
- **Locks:** não segure `std::sync::Mutex` através de um `.await`. Para estado compartilhado entre conexões, prefira canais (`tokio::sync::broadcast`/`mpsc`) a locks; se precisar de lock, mantenha a seção crítica curta.
- **Timeouts e limites:** todo I/O externo (LiveKit, rede) com `tokio::time::timeout`. Defina tamanho máximo de mensagem WS e de corpo HTTP; não confie no cliente.
- **Encerramento gracioso:** `axum::serve(...).with_graceful_shutdown(...)` ouvindo `Ctrl+C`, para fechar conexões e o banco sem corromper dados.
- **Logs estruturados:** campos em vez de interpolação (`tracing::info!(user_id = %id, "entrou na sala")`). Use `#[tracing::instrument(skip(...))]` em handlers relevantes. **Nunca logue segredos**: tokens, JWT, convites, senhas.
- **Validação na borda:** valide e normalize entrada no handler (tamanho, formato) e converta para tipos do domínio (newtypes como `UserId`, `RoomName`) antes de passar adiante. Dentro do domínio, os dados já são válidos.
- **SQL:** `sqlx::query`/`query_as`/`query_scalar` com `.bind(...)` (checadas em runtime e cobertas por testes; as macros `query!` exigiriam `sqlx-cli` na CI). Nunca `format!` com SQL.

## Variáveis de ambiente

| Variável | Padrão | Descrição |
|---|---|---|
| `SUN_ADDR` | `0.0.0.0:7700` | Endereço de escuta (`ip:porta`; inválido faz o servidor não subir) |
| `SUN_DB` | `orbit.db` ao lado do executável | Caminho do arquivo SQLite (criado se não existir) |
| `RUST_LOG` | `info,sun=debug` em dev, `info` em release | Filtro do `tracing` (ex.: `sun=debug`; inválido faz o servidor não subir) |

## Testes

```sh
cargo test -p sun
```

Se `target/debug/sun.exe` estiver travado (o servidor está rodando), use `CARGO_TARGET_DIR=target/verify cargo test -p sun`.
