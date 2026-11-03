# Orbit

Um "Discord" self-hosted para jogar com os amigos: chat, salas de voz e transmissão de tela.

```
orbit/
├── sun/       # servidor (Rust: axum + SQLite) — roda no PC do host
├── moon/      # app desktop (Tauri 2 + Vite/React) — o que os amigos instalam
└── shared/    # crate com o protocolo/tipos usados por sun e moon
```

## Documentação

- [`docs/architecture.md`](docs/architecture.md): como as peças se encaixam
- [`docs/roadmap.md`](docs/roadmap.md): fases e progresso
- [`docs/decisions/`](docs/decisions/README.md): por que cada escolha foi feita
- [`CLAUDE.md`](CLAUDE.md): guia para desenvolver com o Claude Code (vale para humanos também)

## Pré-requisitos

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- Node.js 22+
- [Pré-requisitos do Tauri para Windows](https://tauri.app/start/prerequisites/) (MSVC Build Tools + WebView2)

## Rodando

```sh
# servidor
cargo run -p sun

# app (dentro de moon/)
npm install
npm run tauri dev
```

## Desenvolvendo com o Claude Code

O repositório já vem configurado para o Claude Code:

- `CLAUDE.md` na raiz e em cada pacote com contexto e convenções
- hook que formata automaticamente (rustfmt/Biome) cada arquivo editado
- skills: `/verify` (todas as checagens), `/spec` (nova funcionalidade), `/adr` (nova decisão)
