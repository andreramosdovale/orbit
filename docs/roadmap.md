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

## Fase 1: Contas e autenticação

- [x] SQLite com migrations (`sqlx`), arquivo `orbit.db` ao lado do executável
- [x] Auto-cadastro com usuário + senha (argon2), sem convite ([spec 001](specs/001-cadastro-de-usuarios.md), [ADR 0007](decisions/0007-auto-cadastro-sem-convite.md))
- [x] Login com usuário + senha, JWT ([spec 002](specs/002-login-jwt.md), [ADR 0008](decisions/0008-jwt-com-segredo-no-sqlite.md))
- [ ] `/ws` autenticado
- [ ] Tela de login no `moon` com endereço do servidor configurável
- [ ] Testes de front com Vitest

## Fase 2: Chat de texto

- [ ] Canais de texto (criar/listar)
- [ ] Mensagens em tempo real via WebSocket, persistidas
- [ ] Histórico paginado
- [ ] Presença (online/offline)

## Fase 3: Voz

- [ ] LiveKit server rodando no host (script/guia de setup)
- [ ] `sun` emite tokens LiveKit por canal de voz
- [ ] Entrar/sair de sala, mute, indicador de quem está falando no `moon`

## Fase 4: Transmissão de tela v1

- [ ] Transmissão via `getDisplayMedia` no webview (resultado rápido, sem áudio por app)
- [ ] Assistir transmissões de outros

## Fase 5: Transmissão de tela v2 (nativa)

- [ ] Captura de janela via Windows Graphics Capture (`windows-capture`)
- [ ] Áudio do processo via WASAPI process loopback
- [ ] Encoding em hardware
- [ ] Publicação via LiveKit Rust SDK como `<usuario>#screen`

## Fase 6: Distribuição

- [ ] Instalador (NSIS) e auto-update (`tauri-plugin-updater`)
- [ ] Guia para os amigos: instalar Tailscale + Orbit e conectar
