# Orbit

"Discord" self-hosted para um grupo pequeno de amigos: chat de texto, salas de voz e transmissão de tela. O servidor roda no PC do host e os amigos conectam via Tailscale com um app desktop Windows.

Antes de mexer em algo não trivial, leia [`docs/architecture.md`](docs/architecture.md). As decisões já tomadas estão em [`docs/decisions/`](docs/decisions/README.md): **não as rediscuta nem as contorne sem um ADR novo**.

## Mapa do repositório

| Pasta | O que é | Instruções |
|---|---|---|
| `sun/` | Servidor (Rust, axum, tokio) | [`sun/CLAUDE.md`](sun/CLAUDE.md) |
| `moon/` | App desktop (Tauri 2 + Vite + React + TS) | [`moon/CLAUDE.md`](moon/CLAUDE.md) |
| `shared/` | Protocolo: tipos usados por `sun` e `moon` | [`shared/CLAUDE.md`](shared/CLAUDE.md) |
| `docs/` | Arquitetura, protocolo, roadmap, ADRs, specs | [`docs/README.md`](docs/README.md) |
| `.claude/` | Settings, hook de formatação, skills | — |

Os três crates Rust formam um Cargo workspace. A raiz guarda as versões compartilhadas (`[workspace.dependencies]`), os lints e o profile de release.

## Comandos

```sh
cargo run -p sun                         # servidor em 0.0.0.0:7700 (SUN_ADDR para mudar)
cd moon && npm run tauri dev             # app em modo dev (também regenera bindings.ts)

cargo test --workspace                   # todos os testes Rust
cargo clippy --workspace --all-targets -- -D warnings
cd moon && npm run check                 # Biome (lint + format, modo CI)
cd moon && npm run build                 # tsc + vite build (gera moon/dist)
```

**Use `/verify` para rodar tudo na ordem certa.** Os passos e os problemas conhecidos estão em `.claude/skills/verify/SKILL.md`.

## Fluxo de trabalho

1. **Funcionalidade nova → `/spec` primeiro.** A spec define critérios de aceite verificáveis. Espere aprovação do usuário antes de implementar.
2. **Implemente em passos pequenos**, cada um terminando com `/verify` verde.
3. **Teste junto com o código.** Lógica do `sun` → `sun/tests/`; formato do protocolo → teste em `shared` que fixe o JSON.
4. **A doc acompanha o código no mesmo commit:** `docs/protocol.md`, `docs/architecture.md`, `docs/roadmap.md` (marque o item), os `CLAUDE.md` afetados.
5. **Decisão técnica relevante → `/adr`.**

## Mudou o protocolo? Checklist

1. Altere os tipos em `shared/src/lib.rs` e o teste de formato do fio.
2. Atualize `docs/protocol.md`.
3. Ajuste o `sun` e os testes de integração dele.
4. Rode `cargo test -p moon` para regenerar `moon/src/bindings.ts` e ajuste o front.
5. Rode `/verify`.

## Convenções

- **Idioma:** identificadores, nomes de arquivo e crates em inglês. Docs, comentários, mensagens de log/erro e descrições de commit em português.
- **Commits:** Conventional Commits (`feat(sun): ...`, `fix(moon): ...`, `docs: ...`, `chore: ...`). Escopo = pacote.
- **Erros em Rust:** nada de `unwrap()`/`expect()` fora de testes (o clippy avisa). Use `anyhow` em binários e `thiserror` em bibliotecas quando houver erros tipados.
- **Logs:** `tracing` (`info!`, `warn!` etc.). Nada de `println!` no `sun`.
- **Formatação é automática:** um hook roda `rustfmt`/Biome após cada edição. Não formate à mão e não brigue com o formatador.
- **Dependências:** dependências Rust usadas por mais de um crate vão em `[workspace.dependencies]`. Pergunte antes de adicionar uma dependência nova relevante.

## Não faça

- Não edite `moon/src/bindings.ts`: ele é gerado.
- Não mexa nas versões do `specta`/`tauri-specta`/`specta-typescript` isoladamente: elas são fixadas juntas (ADR 0005).
- Não use `--no-verify` nem desligue lints para passar na CI. Corrija a causa.
- Não mate processos `sun.exe`/`moon.exe` do usuário. Se o `target/` estiver travado, use `CARGO_TARGET_DIR=target/verify`.
- Não adicione funcionalidades fora da spec em andamento. Anote em "Questões em aberto".

## Ambiente

- Windows 11. O host e os clientes também são Windows. A CI roda em `windows-latest`.
- Rust stable (`rust-toolchain.toml`) e Node 22.
- Fim de linha LF em tudo (`.gitattributes`).
