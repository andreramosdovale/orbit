# 0002. LiveKit como SFU para voz e vídeo

- **Status:** Aceita
- **Data:** 2026-11-03

## Contexto

Com P2P puro (mesh), quem transmite envia uma cópia para cada espectador, e isso não escala além de 2–3 pessoas. Precisamos de um SFU (Selective Forwarding Unit).

## Decisão

Usar o **LiveKit server** (open source, binário único) rodando no PC do host, ao lado do `sun`. O `sun` só emite tokens (`livekit-api`). O `moon` usa o SDK JS no webview e o SDK Rust (`livekit`) para a tela nativa.

## Alternativas consideradas

- **Mesh P2P:** não escala com transmissão de tela.
- **mediasoup (binding Rust) / str0m:** SFU dentro do processo Rust, mas é muito mais código para manter.
- **Janus:** em C, com configuração mais complexa e SDKs menos convenientes.

## Consequências

- Um processo extra (escrito em Go) para o host rodar. Precisa de script/guia na Fase 3.
- Ganhamos simulcast, controle de banda e SDKs maduros.
- O upload do host continua sendo o gargalo.
