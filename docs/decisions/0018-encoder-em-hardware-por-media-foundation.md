# 0018. Encoder em hardware por Media Foundation, publicado sem recodificar

- **Status:** Aceita
- **Data:** 2026-11-04

## Contexto

O SDK do LiveKit não oferece encoder em hardware no Windows ([ADR 0017](0017-sdk-nativo-do-livekit.md)), e o VP8 em software usa o processador de quem transmite, muitas vezes durante um jogo. O SDK aceita H.264 já codificado com `VideoEncoderBackend::PreEncoded`, que a prova técnica da [spec 014](../specs/014-encoding-em-hardware.md) confirmou funcionar até o webview.

## Decisão

- Codificar em H.264 Constrained Baseline com o encoder da placa, por Media Foundation, e publicar sem recodificar.
- Abrir o encoder e codificar um quadro de teste antes de publicar. Se nenhum funcionar, publicar em VP8 no processador.
- Quadros passam pela memória do sistema, e os objetos COM ficam em uma thread própria.

## Alternativas consideradas

- **Crate de NVENC:** só cobriria a NVIDIA, com mais uma dependência nativa. Media Foundation é parte do Windows e abre o caminho para outras placas.
- **`livekit-capture`:** declara ser "Developer Preview" e traria um crate a mais só para o laço que já temos. Usamos a API de `libwebrtc` por baixo dele.
- **Textura direto da captura para a placa:** evita uma cópia, mas a captura e o encoder podem estar em placas diferentes. Fica para quando o desempenho pedir.

## Consequências

- `capture/encoder.rs` concentra `unsafe` e FFI do Windows, com uma API segura.
- Quem assiste decodifica H.264 ou VP8, conforme quem transmite, sem mudança no front.
- Um encoder que falhe depois de iniciar encerra a transmissão, com um aviso, em vez de trocar de codec no ar.
