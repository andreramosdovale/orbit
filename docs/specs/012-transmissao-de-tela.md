# 012. Transmissão de tela (v1, pelo webview)

- **Status:** Substituída pela [spec 013](013-transmissao-nativa.md), que removeu a captura pelo webview antes do teste com a tela real
- **Fase:** 4 (ver `docs/roadmap.md`)

## Objetivo

Quem está em uma sala de voz clica em **Transmitir tela**, escolhe uma tela ou janela no seletor do sistema e começa a transmitir. As outras pessoas da sala veem quem está transmitindo e escolhem assistir, sem sair do chat nem da conversa. É a primeira versão, pelo webview, como define o [ADR 0006](../decisions/0006-transmissao-de-tela-hibrida.md).

## Fora de escopo

- **Áudio da transmissão:** nem do sistema nem por aplicativo. O áudio por aplicativo é a Fase 5.
- **Captura nativa, encoding em hardware e identidade `<usuario>#screen`:** Fase 5. Na v1, a tela sai pela mesma identidade da voz.
- **Seletor próprio:** usa o seletor padrão do WebView2.
- **Escolher qualidade na interface:** a qualidade é fixa nesta versão.
- **Transmitir sem estar em uma sala de voz** e **ver uma transmissão sem entrar na sala**.
- **Assistir a duas transmissões ao mesmo tempo:** várias pessoas podem transmitir, mas cada espectador assiste a uma por vez.
- **Prévia da própria transmissão:** quem transmite vê só o aviso de que está transmitindo.
- **Câmera.**

## Mudanças

### Protocolo (`shared` + `docs/protocol.md`)

Os tipos e as rotas não mudam. Muda uma claim do token de voz:

```text
canPublishSources: ["microphone", "screen_share"]
```

`screen_share_audio` continua de fora, então o LiveKit recusa áudio de transmissão mesmo que um cliente tente publicá-lo.

### Dados (SQLite / migrations)

Nenhuma.

### `sun`

Só a lista de fontes do token (`sun/src/voice/token.rs`) e o teste que a fixa. O `sun` não sabe quem está transmitindo: esse estado vem do LiveKit.

### `moon` (UI e core Rust)

**Quem transmite:**

- O painel da sala ganha o botão **Transmitir tela**, que vira **Parar transmissão** enquanto estiver no ar, e o aviso "Você está transmitindo".
- Fechar o seletor sem escolher nada não é erro: nada acontece, e nenhuma mensagem aparece.
- Parar pela barra do próprio navegador tem o mesmo efeito de clicar em **Parar transmissão**.
- Sair da sala, trocar de sala, sair da conta ou perder a conexão de voz encerra a transmissão.
- Trocar de canal de texto ou perder o WebSocket do chat não encerra.

**Quem assiste:**

- Na lista da sala, quem transmite aparece com a marcação "transmitindo" e o botão **Assistir**. O botão anuncia o nome da pessoa para leitores de tela.
- **Assistir** abre o vídeo no topo da área principal, com as mensagens abaixo, e os botões **Tela cheia** e **Parar de assistir**.
- Assistir a outra pessoa troca o vídeo. Quando a transmissão termina, o vídeo some, e aparece o aviso "A transmissão terminou".
- O vídeo não toca som e não muda a voz.

**O vídeo só é baixado por quem assiste.** O upload do host é o gargalo do Orbit (ver `docs/architecture.md`), então a sala passa a conectar com `autoSubscribe: false`:

- `room.ts` assina sozinho todos os microfones, inclusive os de quem entrar depois;
- a tela só é assinada ao clicar em **Assistir**, e a assinatura é cancelada ao parar de assistir.

**Qualidade:** até 1080p a 30 quadros por segundo e 4 Mbps, sem simulcast, otimizada para texto legível.

**Organização:**

- `room.ts` continua sendo o único lugar que usa o `livekit-client`. A interface `VoiceRoom` ganha transmitir, parar, assistir e parar de assistir, e `VoiceParticipant` ganha `sharing`.
- `useVoiceRoom` sobe para o `ChatLayout`, porque a lateral controla a sala e a área principal mostra o vídeo.
- Componente novo `ScreenStage` em `features/voice/`, usando o design system.
- Nenhuma permissão nova no core Rust, se a prova técnica confirmar o resultado preliminar abaixo.

## Resultado preliminar da prova técnica

Feita em 2026-11-04, no app real em modo de build (`http://tauri.localhost`, WebView2 156), lendo só os metadados da captura. Nenhuma imagem foi salva nem enviada.

- `getDisplayMedia` existe no app, e uma captura abriu em 1920x1080 a 30 quadros por segundo, sem mudar o core Rust.
- **Inconclusivo:** o script não consegue ver o seletor nem saber se alguém clicou nele. Com a seleção automática apontando para uma janela chamada "Orbit", a chamada ficou aguardando, o que indica que o seletor apareceu e esperou uma escolha. O seletor real é verificado no primeiro passo do plano.

## Critérios de aceite

**Testes automatizados:**

- [x] `sun/tests/voice.rs` fixa `canPublishSources` em `["microphone", "screen_share"]`, sem `screen_share_audio`.
- [x] `useVoiceRoom.test.ts`: transmitir e parar atualizam o estado; cancelar o seletor não gera erro; uma falha real mostra uma mensagem; sair e trocar de sala encerram a transmissão e o vídeo; o fim da transmissão assistida fecha o vídeo e mostra o aviso.
- [x] `VoicePanel.test.tsx`: os botões de transmitir, a marcação "transmitindo" e o **Assistir** aparecem para as pessoas certas, e nunca para você mesmo.
- [x] `ScreenStage.test.tsx`: nome de quem transmite, **Tela cheia** e **Parar de assistir**.
- [x] `ChatLayout.test.tsx`: trocar de canal de texto mantém o vídeo aberto.
- [x] `/verify` verde.

**No app real, em modo de build, com `sun` e LiveKit reais e duas janelas pelo IP do Tailscale.** O script troca o `getDisplayMedia` por um vídeo sintético, para nenhuma imagem da tela do usuário circular:

- [x] O vídeo publicado aparece na outra janela, com quadros novos chegando.
- [x] Antes de **Assistir**, a outra janela não recebe bytes de vídeo; ao **Parar de assistir**, eles param de chegar.
- [x] Os microfones continuam sendo ouvidos, inclusive o de quem entrou depois, com `autoSubscribe: false`.
- [x] Parar a track como a barra do navegador faz atualiza as duas janelas.
- [x] Sair da sala encerra a transmissão, e trocar de canal de texto ou derrubar o `/ws` não encerra.
- [x] O LiveKit recusa uma publicação de `screen_share_audio` feita com o token real.

**Verificação pelo usuário**, porque o script não enxerga o seletor nem julga a imagem:

- [ ] O seletor abre, permite escolher tela ou janela, e cancelar não mostra erro.
- [ ] A transmissão real fica legível em outro PC via Tailscale.
- [ ] O botão **Tela cheia** funciona na janela do app.

## Verificação automatizada

Feita em 2026-11-04 no modo de build, com duas janelas reais, `sun` e LiveKit reais pelo IP do Tailscale e microfone simulado, trocando o `getDisplayMedia` por um vídeo sintético de 1280x720. Todas as checagens passaram.

**O que não cobre:**

- o seletor real e a captura real da tela, que dependem do usuário;
- o limite de 4 Mbps, porque o vídeo sintético ficou bem abaixo dele (cerca de 300 Kbps);
- duas máquinas diferentes;
- a recusa do áudio da tela no próprio servidor: o SDK recusou antes de enviar, usando as permissões recebidas do LiveKit. O token não inclui a fonte, mas um cliente modificado não foi testado.

## Plano de implementação

Cada passo termina com `/verify` verde.

1. **Prova técnica com o usuário:** transmitir de verdade, para confirmar o seletor, a permissão e a tela cheia. Por depender do usuário, é feita com a funcionalidade pronta, junto da verificação final.
2. `sun`: fontes do token, teste e `docs/protocol.md`.
3. `room.ts`: `autoSubscribe: false`, com assinatura automática dos microfones, e confirmar por script que a voz segue funcionando.
4. `room.ts` e `useVoiceRoom`: transmitir, parar, assistir e parar de assistir, com testes.
5. Subir `useVoiceRoom` para o `ChatLayout`, sem mudança visível.
6. `VoicePanel` e `ScreenStage`, com testes.
7. Verificação no app real, documentação e ADR, se a prova técnica mudar alguma decisão.

## Decisões

- **Lugar do vídeo:** no topo da área principal, com as mensagens abaixo e um botão de tela cheia.
- **Qualidade:** 1080p a 30 quadros por segundo e até 4 Mbps.

## Questões em aberto

Nenhuma.
