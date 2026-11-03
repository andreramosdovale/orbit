# Orbit

Um "Discord" self-hosted para jogar com os amigos: chat, salas de voz e transmissão de tela.

```
orbit/
├── sun/       # servidor (Rust: axum + SQLite) — roda no PC do host
├── moon/      # app desktop (Tauri 2 + Vite/React) — o que os amigos instalam
└── shared/    # crate com o protocolo/tipos usados por sun e moon
```

## Arquitetura

- **Rede:** os amigos acessam o PC do host pelo [Tailscale](https://tailscale.com) (compartilhando só a máquina do servidor).
- **sun:** API REST + WebSocket (chat, presença, auth via convite + JWT), persistência mínima em SQLite e emissão de tokens do LiveKit.
- **Voz/vídeo:** [LiveKit server](https://github.com/livekit/livekit) (SFU) roda ao lado do `sun` no PC do host.
- **moon:** UI e voz no webview (LiveKit JS); a transmissão de tela é capturada nativamente em Rust
  (Windows Graphics Capture + WASAPI process loopback) e publicada como uma track separada.

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
