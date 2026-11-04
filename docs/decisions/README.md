# Decisões de arquitetura (ADRs)

Registro curto de cada decisão técnica relevante e do porquê. Use `/adr` para criar uma nova. ADRs nunca são apagados: quando uma decisão muda, o ADR antigo vira `Substituída por NNNN`.

| # | Decisão | Status |
|---|---|---|
| [0001](0001-stack-rust-tauri-react.md) | Stack: Rust no servidor e no app, Tauri 2 + React no front | Aceita |
| [0002](0002-livekit-como-sfu.md) | LiveKit como SFU para voz e vídeo | Aceita, alterada pela 0014 |
| [0003](0003-rede-via-tailscale.md) | Rede privada via Tailscale | Aceita |
| [0004](0004-sqlite-persistencia-minima.md) | SQLite com persistência mínima | Aceita |
| [0005](0005-tipos-gerados-com-specta.md) | Tipos TypeScript gerados do Rust com specta | Aceita |
| [0006](0006-transmissao-de-tela-hibrida.md) | Transmissão de tela: webview na v1, captura nativa em Rust na v2 | Aceita |
| [0007](0007-auto-cadastro-sem-convite.md) | Auto-cadastro sem convite: acesso controlado pelo Tailscale | Aceita |
| [0008](0008-jwt-com-segredo-no-sqlite.md) | JWT HS256 com segredo guardado no SQLite, válido por 30 dias | Aceita |
| [0009](0009-autenticacao-do-ws-na-primeira-mensagem.md) | Autenticação do WebSocket pela primeira mensagem | Aceita |
| [0010](0010-token-no-localstorage.md) | Token de sessão guardado no `localStorage` | Aceita |
| [0011](0011-retencao-de-mensagens.md) | Mensagens de texto guardadas sem limite de tempo | Aceita |
| [0012](0012-tanstack-query-no-front.md) | TanStack Query para as chamadas de chat e o cache em memória do `moon` | Aceita, alterada pela 0013 |
| [0013](0013-reconexao-automatica.md) | Reconexão automática mantendo a tela e o cache | Aceita |
| [0014](0014-livekit-no-host.md) | LiveKit no host: scripts, chaves e tokens sem SDK | Aceita |
| [0015](0015-permissao-do-microfone.md) | O core do `moon` permite o microfone, sem diálogo | Aceita |
| [0016](0016-design-system-proprio.md) | Design system próprio, em CSS puro | Aceita |
