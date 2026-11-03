---
name: verify
description: Roda todas as verificações do Orbit (fmt, clippy, testes, bindings, lint e build do front) e reporta o que falhou. Use antes de dizer que uma tarefa terminou e antes de qualquer commit.
---

# /verify

Rode na raiz do repositório, nesta ordem, e **pare no primeiro passo que falhar** para corrigir:

1. `cargo fmt --all --check`
2. `cd moon && npm run build`: o Tauri precisa de `moon/dist` para compilar o `moon`.
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`: também regenera `moon/src/bindings.ts`.
5. `git status --short moon/src/bindings.ts`: se mudou, os tipos do front mudaram. Rode `npm run build` de novo para checar se o front ainda compila, e inclua o arquivo no commit.
6. `cd moon && npm run check`: Biome (lint + formatação).

## Problemas conhecidos

- **`failed to remove file ...\target\debug\sun.exe` / `Acesso negado`**: o `sun` ou o `moon` está rodando e travando o executável. Rode com outro target dir: `CARGO_TARGET_DIR=target/verify cargo test --workspace`. Não mate os processos do usuário.
- **`STATUS_ENTRYPOINT_NOT_FOUND` em testes do `moon`**: os testes que linkam o Tauri precisam ficar em `moon/src-tauri/tests/` (testes de integração). O `build.rs` só embute o manifest do Windows nesses testes.

## Saída

Termine com uma linha por passo (✅/❌). Se algo falhou e você não corrigiu, diga isso explicitamente.
