# 002. Login com usuário + senha (JWT)

- **Status:** Concluída
- **Fase:** 1 (ver `docs/roadmap.md`)

## Objetivo

Um usuário já cadastrado (spec 001) envia usuário e senha para o `sun` e recebe um **JWT** que identifica quem ele é. Esse token é o que as próximas specs vão exigir: `/ws` autenticado e, depois, todas as rotas de chat e voz.

## Fora de escopo

Cada item abaixo é um item separado do roadmap ou ainda não está nele:

- **`/ws` autenticado**: próximo item do roadmap. Esta spec só emite e valida o token; nenhuma rota passa a exigir token ainda.
- **Tela de login no `moon`**: item separado do roadmap. Aqui o `moon` só recebe os tipos novos em `bindings.ts`.
- **Testes de front com Vitest**: item separado do roadmap.
- Refresh token, logout com revogação e lista de tokens revogados.
- Troca ou recuperação de senha.
- Rate limiting de tentativas de login (mesma justificativa da spec 001: a rede já é restrita aos amigos).

## Mudanças

### Protocolo (`shared` + `docs/protocol.md`)

Tipos novos em `shared/src/lib.rs`, com derive `specta::Type`:

```rust
/// Corpo de `POST /auth/login`.
pub struct LoginRequest { pub username: String, pub password: String }

/// Resposta de login: o token e o usuário logado.
pub struct LoginResponse { pub token: String, pub user: User }
```

Código novo em `ApiErrorCode`: `InvalidCredentials` (`"invalid_credentials"`).

| Método | Rota | Corpo | Sucesso | Erros |
|---|---|---|---|---|
| `POST` | `/auth/login` | `LoginRequest` | `200` + `LoginResponse` | `400 invalid_request` / `401 invalid_credentials` / `500 internal` |

Regras:

- O `username` é convertido para minúsculas antes da busca (`FULANO` loga como `fulano`).
- **Usuário inexistente e senha errada dão a mesma resposta**: `401 invalid_credentials`, com a mensagem "Usuário ou senha incorretos.". Isso evita que alguém descubra quais usuários existem.
- O login não aplica as regras de formato do cadastro: um usuário ou senha fora das regras só não vai bater com nenhuma conta e dá `401`.
- O token é enviado pelo cliente, nas próximas specs, no cabeçalho `Authorization: Bearer <token>`.

**Formato do JWT** (HS256):

| Claim | Valor |
|---|---|
| `sub` | id do usuário, como string (`"1"`) |
| `username` | nome normalizado |
| `iat` | emissão (Unix, segundos) |
| `exp` | `iat` + **30 dias** |

### Dados (SQLite / migrations)

Migration `0002_secrets.sql`, para o segredo do JWT:

```sql
CREATE TABLE secrets (
    name  TEXT PRIMARY KEY,
    value BLOB NOT NULL
);
```

- Quando o `sun` sobe, ele lê o segredo `jwt` da tabela. Se não existir, gera **32 bytes aleatórios** (gerador do sistema operacional) e grava.
- Assim o host não precisa configurar nada, e os tokens continuam válidos depois de reiniciar o servidor.
- Apagar o `orbit.db` invalida todos os tokens, o que é esperado, porque as contas também somem.

### `sun`

- `auth/jwt.rs`:
  - `load_or_create_secret(db)`;
  - `issue(user, now) -> String`;
  - `verify(token, now) -> Result<Claims, JwtError>`.
  
  O `now` é parâmetro para poder testar a expiração sem esperar 30 dias. O `verify` ainda não tem rota que use, mas fica pronto para o `/ws` autenticado.
- `AppState` ganha o campo `jwt` com as chaves de assinatura. O `main.rs` carrega o segredo depois de abrir o banco, e o helper de teste faz o mesmo no banco em memória.
- `auth/login.rs`:
  - busca o usuário pelo nome normalizado;
  - verifica a senha com argon2 dentro de `spawn_blocking`;
  - quando o usuário não existe, calcula um hash argon2 da senha recebida e descarta, para a resposta levar o mesmo tempo nos dois casos.
- `ApiError::InvalidCredentials` → `401`.
- Logs `debug!` do fluxo, iguais aos do cadastro. Nunca logar a senha nem o token.
- Dependências novas (preciso da sua aprovação):
  - **`jsonwebtoken`**, com o backend de criptografia em Rust puro (`rust_crypto`). O outro backend (`aws_lc_rs`) precisa de CMake/NASM para compilar no Windows.
  - **`getrandom`**, para gerar o segredo.

### `moon` (UI e core Rust)

- `LoginRequest` e `LoginResponse` registrados no `specta_builder` para aparecerem em `bindings.ts`. Nenhuma tela nova.

## Critérios de aceite

- [x] `shared`: os testes de formato do fio para `LoginRequest`, `LoginResponse` e `ApiErrorCode::InvalidCredentials` passam.
- [x] `sun/tests/auth.rs::login_returns_token_and_user`: depois do cadastro, o login dá `200` com `user` igual ao cadastrado e `token` não vazio.
- [x] `sun/tests/auth.rs::login_is_case_insensitive`: o cadastro de `fulano` loga com `FULANO`.
- [x] `sun/tests/auth.rs::login_wrong_password_returns_401`: senha errada dá `401 invalid_credentials`.
- [x] `sun/tests/auth.rs::login_unknown_user_returns_401`: usuário inexistente dá `401` com o **mesmo corpo** da senha errada.
- [x] `sun/tests/auth.rs::login_malformed_body_returns_400`: JSON inválido dá `400 invalid_request`.
- [x] `sun/tests/auth.rs::login_response_never_contains_password`: o corpo da resposta não contém a senha nem o hash.
- [x] `sun/tests/auth.rs::jwt_secret_persists`: abrir o mesmo arquivo de banco duas vezes dá o mesmo segredo, e um token emitido na primeira abertura é válido na segunda.
- [x] Testes de unidade em `sun/src/auth/jwt.rs`:
  - o token emitido é validado e traz `sub`, `username` e `exp = iat + 30 dias`;
  - um token expirado é rejeitado;
  - um token assinado com outro segredo é rejeitado;
  - um token adulterado é rejeitado.
- [x] `moon/src/bindings.ts` contém `LoginRequest` e `LoginResponse`.
- [x] `/verify` verde.

## Plano de implementação

Passos pequenos, cada um terminando com `/verify` verde.

1. **ADR 0008:** JWT HS256 com segredo gerado pelo `sun` e guardado no SQLite, válido por 30 dias, sem refresh.
2. **shared:** `LoginRequest`, `LoginResponse`, `InvalidCredentials` + testes de formato + `docs/protocol.md`.
3. **sun, segredo e JWT:** migration `0002_secrets.sql`, `auth/jwt.rs` com testes de unidade, `AppState.jwt`, `main.rs` e helper de teste.
4. **sun, login:** `auth/login.rs`, rota `/auth/login`, `ApiError::InvalidCredentials` e os testes de `sun/tests/auth.rs`.
5. **moon:** tipos no `specta_builder` e regeneração do `bindings.ts`.
6. **docs:** `docs/architecture.md`, `sun/CLAUDE.md`, roadmap (marcar "Login com usuário + senha, JWT"), `docs/specs/README.md` e esta spec como `Concluída`.

## Decisões

Questões resolvidas com o usuário em 2026-11-03: as três propostas abaixo foram aceitas e registradas no [ADR 0008](../decisions/0008-jwt-com-segredo-no-sqlite.md).

## Questões em aberto (originais)

1. **Validade do token:** proposto **30 dias, sem refresh**. Num grupo de amigos, pedir login toda semana incomoda, e o risco de um token vazado dentro da rede do Tailscale é baixo. A troca é que não dá para "deslogar à força" alguém antes de expirar.
2. **Onde fica o segredo:** proposto gerar e guardar no SQLite. A alternativa é uma variável `SUN_JWT_SECRET`, mas aí o host teria mais uma coisa para configurar e, se esquecesse, todos os tokens morreriam a cada reinício.
3. **Dependências novas:** `jsonwebtoken` (com `rust_crypto`) e `getrandom`. Podem entrar?
