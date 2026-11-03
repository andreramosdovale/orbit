# sun: servidor

Servidor do Orbit que roda no PC do host. axum + tokio. Vai ganhar sqlx (SQLite) na Fase 1 e emissão de tokens LiveKit na Fase 3.

## Estrutura

```
src/
├── main.rs   # só lê config (env), inicia tracing e chama sun::app(). Sem lógica.
├── lib.rs    # app(): monta o Router com todas as rotas
└── ws.rs     # WebSocket /ws: recebe ClientMessage, responde ServerMessage
tests/
└── api.rs    # testes de integração
```

Conforme o servidor crescer, organize **por domínio** (`auth/`, `chat/`, `voice/`), cada um com seu `mod.rs` expondo as rotas (`fn routes() -> Router<AppState>`) e montado em `lib.rs`.

## Padrões

- **Testável sem rede:** toda rota HTTP deve ser testável via `sun::app().oneshot(request)` (`tower::ServiceExt`). Só o WebSocket precisa de porta real (`127.0.0.1:0`). Siga os exemplos em `tests/api.rs`.
- **Estado compartilhado** (pool do banco, config, conexões WS): uma struct `AppState` com `Clone` passada via `Router::with_state`. `app()` vai passar a receber o estado, para os testes poderem usar um SQLite em memória.
- **Mensagens do WebSocket:** sempre os tipos de `orbit_shared`. Nunca monte JSON à mão. Mensagem inválida gera `tracing::warn!` e é ignorada, sem derrubar a conexão.
- **Config via variáveis de ambiente**, com padrão sensato em `main.rs` (`SUN_ADDR`, padrão `0.0.0.0:7700`). Documente cada variável nova aqui.
- **Erros de handler:** quando houver mais de um, crie um `enum ApiError` com `impl IntoResponse`. Não retorne `500` com a mensagem interna.

## Variáveis de ambiente

| Variável | Padrão | Descrição |
|---|---|---|
| `SUN_ADDR` | `0.0.0.0:7700` | Endereço de escuta |
| `RUST_LOG` | `info` | Filtro do `tracing` (ex.: `sun=debug`) |

## Testes

```sh
cargo test -p sun
```

Se `target/debug/sun.exe` estiver travado (o servidor está rodando), use `CARGO_TARGET_DIR=target/verify cargo test -p sun`.
