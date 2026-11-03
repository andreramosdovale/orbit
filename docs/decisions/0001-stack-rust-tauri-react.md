# 0001. Stack: Rust no servidor e no app, Tauri 2 + React no front

- **Status:** Aceita
- **Data:** 2026-11-03

## Contexto

O Orbit precisa de captura de tela e áudio de alta qualidade no Windows, incluindo áudio de um único aplicativo, como o Discord faz. Também queremos um app leve e um servidor fácil de rodar no PC do host.

## Decisão

- **Servidor (`sun`):** Rust com axum + tokio.
- **App (`moon`):** Tauri 2. A UI fica em Vite + React + TypeScript no webview (WebView2) e o core em Rust.
- **Monorepo** com um Cargo workspace e um crate `shared` para o protocolo.

## Alternativas consideradas

- **Electron:** `desktopCapturer` pronto, mas pesado e sem acesso fácil a áudio por processo.
- **Rust nativo puro (iced/Slint):** controle total, mas renderizar vídeo remoto e construir a UI seria muito mais trabalho.
- **Node/TypeScript no servidor:** viável, mas perderíamos o compartilhamento de tipos via crate `shared`.

## Consequências

- Captura nativa em Rust acessa direto as APIs do Windows (Windows Graphics Capture, WASAPI).
- Dois ecossistemas no app (Rust + TS). Os tipos ficam unificados pelo [ADR 0005](0005-tipos-gerados-com-specta.md).
- Só Windows por enquanto.
