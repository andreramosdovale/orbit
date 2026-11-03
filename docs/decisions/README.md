# Decisões de arquitetura (ADRs)

Registro curto de cada decisão técnica relevante e do porquê. Use `/adr` para criar uma nova. ADRs nunca são apagados: quando uma decisão muda, o ADR antigo vira `Substituída por NNNN`.

| # | Decisão | Status |
|---|---|---|
| [0001](0001-stack-rust-tauri-react.md) | Stack: Rust no servidor e no app, Tauri 2 + React no front | Aceita |
| [0002](0002-livekit-como-sfu.md) | LiveKit como SFU para voz e vídeo | Aceita |
| [0003](0003-rede-via-tailscale.md) | Rede privada via Tailscale | Aceita |
| [0004](0004-sqlite-persistencia-minima.md) | SQLite com persistência mínima | Aceita |
| [0005](0005-tipos-gerados-com-specta.md) | Tipos TypeScript gerados do Rust com specta | Aceita |
| [0006](0006-transmissao-de-tela-hibrida.md) | Transmissão de tela: webview na v1, captura nativa em Rust na v2 | Aceita |
| [0007](0007-auto-cadastro-sem-convite.md) | Auto-cadastro sem convite: acesso controlado pelo Tailscale | Aceita |
