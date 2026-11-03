# 0006. Transmissão de tela: webview na v1, captura nativa em Rust na v2

- **Status:** Aceita
- **Data:** 2026-11-03

## Contexto

`getDisplayMedia` no WebView2 funciona, mas oferece pouco controle: seletor padrão, áudio do sistema instável e nada de áudio por aplicativo. A captura nativa resolve isso, mas é bem mais trabalho.

## Decisão

- **v1 (Fase 4):** `getDisplayMedia` no webview, publicado pelo LiveKit JS. Valida o fluxo de ponta a ponta rápido.
- **v2 (Fase 5):** captura no core Rust. Vídeo via Windows Graphics Capture (`windows-capture`), áudio via WASAPI *process loopback* (Windows 10 2004+), encoding em hardware, publicado pelo SDK Rust do LiveKit como um participante separado `<usuario>#screen`.

## Alternativas consideradas

- **Só webview:** sem áudio por aplicativo, que é o diferencial pedido.
- **Toda a mídia (voz + tela) no Rust:** exigiria renderizar vídeo remoto fora do webview.

## Consequências

- Quem assiste não muda nada entre a v1 e a v2: tudo chega como track LiveKit no webview.
- Um usuário ocupa duas identidades no LiveKit quando transmite. A UI deve agrupá-las.
- A v2 vai exigir `unsafe` e FFI do Windows, que deve ficar isolado num módulo próprio.
