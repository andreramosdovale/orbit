# Roadmap

Cada fase vira uma ou mais specs em [`specs/`](specs/README.md) antes de começar. Uma fase só termina quando todos os critérios de aceite das specs estão verdes no `/verify` e na CI.

## Fase 0: Fundação ✅

- [x] Workspace Cargo (`sun`, `moon`, `shared`) + Tauri 2 com Vite/React
- [x] `CLAUDE.md` (raiz e por pacote), `docs/`, ADRs iniciais
- [x] Tipos TS gerados do Rust (`tauri-specta`) + checagem na CI
- [x] Lint/format: rustfmt, clippy (`-D warnings`), Biome
- [x] Testes: protocolo (`shared`), HTTP e WebSocket (`sun`)
- [x] Harness: `.claude/settings.json`, hook de formatação, skills `/verify`, `/spec`, `/adr`
- [x] CI no GitHub Actions (Windows)

## Fase 1: Contas e autenticação ✅

- [x] SQLite com migrations (`sqlx`), arquivo `orbit.db` ao lado do executável
- [x] Auto-cadastro com usuário + senha (argon2), sem convite ([spec 001](specs/001-cadastro-de-usuarios.md), [ADR 0007](decisions/0007-auto-cadastro-sem-convite.md))
- [x] Login com usuário + senha, JWT ([spec 002](specs/002-login-jwt.md), [ADR 0008](decisions/0008-jwt-com-segredo-no-sqlite.md))
- [x] `/ws` autenticado ([spec 003](specs/003-ws-autenticado.md), [ADR 0009](decisions/0009-autenticacao-do-ws-na-primeira-mensagem.md))
- [x] Tela de login no `moon` com endereço do servidor configurável ([spec 004](specs/004-tela-de-login.md), [ADR 0010](decisions/0010-token-no-localstorage.md))
- [x] Testes de front com Vitest ([spec 004](specs/004-tela-de-login.md))

## Fase 2: Chat de texto ✅

- [x] Canais de texto (criar/listar) ([spec 005](specs/005-canais-de-texto.md))
- [x] Mensagens em tempo real via WebSocket, persistidas ([spec 006](specs/006-mensagens-em-tempo-real.md), [ADR 0011](decisions/0011-retencao-de-mensagens.md))
- [x] Histórico paginado ([spec 007](specs/007-historico-paginado.md), [ADR 0012](decisions/0012-tanstack-query-no-front.md))
- [x] Presença (online/offline) ([spec 008](specs/008-presenca.md))
- [x] Reconexão automática ([spec 009](specs/009-reconexao-automatica.md), [ADR 0013](decisions/0013-reconexao-automatica.md))

## Fase 3: Voz

- [x] LiveKit server rodando no host ([guia](host-setup.md), [ADR 0014](decisions/0014-livekit-no-host.md))
- [x] `sun` emite tokens LiveKit por canal de voz ([spec 010](specs/010-salas-de-voz.md))
- [ ] Entrar/sair de sala, mute, indicador de quem está falando no `moon` ([spec 010](specs/010-salas-de-voz.md), falta o teste com microfones reais)

## Fase 4: Transmissão de tela v1

Substituída pela Fase 5: a captura pelo webview foi removida antes do teste com a tela real ([ADR 0017](decisions/0017-sdk-nativo-do-livekit.md)).

- [ ] Transmissão via `getDisplayMedia` no webview, sem áudio ([spec 012](specs/012-transmissao-de-tela.md), falta o teste com a tela real)
- [ ] Assistir transmissões de outros ([spec 012](specs/012-transmissao-de-tela.md), falta o teste em outro PC)

## Fase 5: Transmissão de tela v2 (nativa)

- [ ] Captura de janela via Windows Graphics Capture (`windows-capture`) ([spec 013](specs/013-transmissao-nativa.md), falta o teste com um jogo real)
- [ ] Áudio do processo via WASAPI process loopback ([spec 013](specs/013-transmissao-nativa.md), falta o teste com um jogo real)
- [ ] Encoding em hardware ([spec 014](specs/014-encoding-em-hardware.md), [ADR 0018](decisions/0018-encoder-em-hardware-por-media-foundation.md), falta o teste com um jogo real)
- [ ] Publicação via LiveKit Rust SDK como `<usuario>#screen` ([spec 013](specs/013-transmissao-nativa.md), falta o teste com um jogo real)

## Fase 6: Distribuição

- [ ] Instalador (NSIS) e auto-update (`tauri-plugin-updater`)
- [ ] Guia para os amigos: instalar Tailscale + Orbit e conectar
