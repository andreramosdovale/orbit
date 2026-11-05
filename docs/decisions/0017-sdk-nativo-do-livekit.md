# 0017. Transmissão nativa com o SDK Rust do LiveKit, em VP8

- **Status:** Aceita, alterada pela [0018](0018-encoder-em-hardware-por-media-foundation.md)
- **Data:** 2026-11-04

## Contexto

O [ADR 0006](0006-transmissao-de-tela-hibrida.md) previu a captura no core Rust, publicada pelo SDK Rust do LiveKit, com encoding em hardware. A prova técnica da [spec 013](../specs/013-transmissao-nativa.md) mostrou que o SDK não oferece encoder em hardware no Windows: o código de NVENC e VAAPI só é compilado no Linux. Em software, o VP8 entregou 748 de 759 quadros em 1080p, e o OpenH264 só 185.

## Decisão

- Vídeo por `windows-capture`, áudio por `wasapi` (process loopback) e publicação por `livekit`, em VP8 no software, com a identidade `<id>#screen`.
- O encoding em hardware fica para uma spec própria.
- A captura de tela pelo webview ([spec 012](../specs/012-transmissao-de-tela.md)) é removida: o token de voz volta a só permitir o microfone.

## Alternativas consideradas

- **Encoder próprio com Media Foundation agora:** exige o modo de quadros já codificados do SDK, ainda em preview, e atrasaria o áudio por aplicativo, que é o principal ganho.
- **OpenH264:** descarta quadros demais nesse uso.
- **Manter o webview como reserva:** dois caminhos para manter, e um token com mais permissões.

## Consequências

- O app cresce de 4,7 MB para perto de 23 MB.
- `.cargo/config.toml` liga `+crt-static` no workspace, porque o WebRTC pré-compilado exige.
- A primeira compilação baixa cerca de 500 MB, sem conferência de hash pelo SDK. Um `CARGO_TARGET_DIR` com caminho muito longo faz o compilador C++ não achar os cabeçalhos.
- O VP8 em software usa o processador de quem transmite, o que pode pesar em jogos exigentes.
