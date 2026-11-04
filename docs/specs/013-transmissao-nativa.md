# 013. Transmissão de tela nativa com áudio por aplicativo

- **Status:** Em andamento
- **Fase:** 5 (ver `docs/roadmap.md`)

## Objetivo

Quem está em uma sala de voz escolhe, no próprio Orbit, um jogo ou outro aplicativo para transmitir, e as pessoas que assistem recebem a imagem e **só o áudio daquele aplicativo**. Música, notificações e as vozes da sala ficam de fora. É a segunda versão da transmissão, no core Rust, como define o [ADR 0006](../decisions/0006-transmissao-de-tela-hibrida.md).

## Fora de escopo

- **Encoding em hardware:** fica para a spec seguinte (ver "Prova técnica" e Decisões).
- **Prévia da janela no seletor:** a lista mostra só o título e o aplicativo.
- **Áudio ao transmitir uma tela inteira:** só janelas têm áudio (ver Decisões).
- **Escolher qualidade na interface:** continua fixa.
- **Volume da transmissão por espectador**, **assistir a duas transmissões ao mesmo tempo** e **câmera**.
- **Instalador:** Fase 6.

## Prova técnica

Feita em 2026-11-04, em uma cópia do projeto fora do repositório, com um LiveKit próprio em outras portas e vídeo sintético. Nenhuma imagem da tela foi usada.

| Pergunta | Resultado |
|---|---|
| O `livekit` 0.10 compila junto do Tauri? | Sim, em 3min30s na primeira vez, com `+crt-static`, que o SDK exige no Windows. O app abriu normalmente com essa configuração. |
| O SDK publica como `<id>#screen`? | Sim: outro participante recebeu a identidade `7#screen`, a fonte `Screenshare` e 1920x1080. |
| VP8 em software dá conta? | Sim: 748 de 759 quadros chegaram, com 74% de um núcleo em um PC de 12 núcleos. O número inclui o receptor, que rodava no mesmo processo. |
| OpenH264 em software dá conta? | Não: só 185 de 759 quadros chegaram. |
| O SDK tem encoder em hardware no Windows? | **Não encontrei.** O código de NVENC e VAAPI do `webrtc-sys` só é compilado no Linux. O caminho seria escrever o encoder, com Media Foundation, e publicar pelo modo de quadros já codificados, que o `livekit-capture` marca como "Developer Preview". |
| Quanto o app cresce? | De 4,7 MB para algo perto de 23 MB, estimado pelo executável de teste, que usa o SDK de verdade. |

**Provado depois, com os testes `ignore` de `moon/src-tauri/tests/capture.rs`:** o áudio capturado de um processo trouxe o tom dele (força 248,6) e nada do tom de outro processo tocando ao mesmo tempo (0,0). A janela de teste chegou em quadros, e fechá-la avisou o fim.

## Mudanças

### Protocolo (`shared` + `docs/protocol.md`)

| Método | Rota | Auth | Sucesso | Erros |
|---|---|---|---|---|
| `POST` | `/voice-channels/{id}/screen-token` | `Bearer` | `200` + `VoiceJoin` | os mesmos da rota de token de voz |

O token da transmissão tem identidade `<id do usuário>#screen`, o `username` como nome e `canPublishSources: ["screen_share", "screen_share_audio"]`, com `canSubscribe` desligado. O token de voz volta a ser `["microphone"]`, depois que a transmissão pelo webview sair.

Comandos Tauri novos, gerados em `bindings.ts`:

| Comando | O que faz |
|---|---|
| `list_capture_targets` | Lista janelas e telas, sem a janela do próprio Orbit |
| `start_screen_share` | Recebe o alvo, o endereço e o token, e começa a publicar |
| `stop_screen_share` | Para de publicar |

Evento novo do core para o front, `screen_share_ended`, com o motivo: a janela foi fechada, a conexão caiu ou houve erro.

### Dados (SQLite / migrations)

Nenhuma.

### `sun`

A rota nova e os testes. O token nunca vai para os logs.

### `moon` (UI e core Rust)

**Core Rust, em `src-tauri/src/capture/`**, com o `unsafe` isolado:

- **Vídeo:** Windows Graphics Capture pelo `windows-capture`, com o cursor e sem a borda amarela. Até 1080p a 30 quadros por segundo e 4 Mbps, em VP8.
- **Áudio:** WASAPI process loopback pelo `wasapi`, incluindo os processos filhos do aplicativo, em 48 kHz estéreo. Exige Windows 10 2004 ou mais novo.
- **Publicação:** uma segunda conexão com o LiveKit, pelo SDK Rust, como `<id>#screen`.
- A janela fechada, minimizada ou redimensionada não pode derrubar o app.

**Front:**

- **Transmitir tela** abre um seletor do próprio Orbit, feito com o design system: janelas, com título e aplicativo, e telas, com o aviso de que vão sem áudio.
- Na lista da sala, `<id>#screen` não aparece como outra pessoa: vira a marcação "transmitindo" da pessoa, como pede o ADR 0006.
- **Assistir** assina o vídeo e o áudio da transmissão, e parar de assistir cancela os dois.
- Quem transmite nunca assina a própria transmissão, para não ouvir o próprio jogo em dobro.
- Sair da sala, trocar de sala, sair da conta ou fechar o app encerra a transmissão.

**Transmissão pelo webview:** é removida (ver Decisões).

### Impactos no projeto

- **Dependências novas no `moon`:** `livekit`, `windows-capture` e `wasapi`, sob licenças Apache-2.0 e MIT.
- **`.cargo/config.toml` novo, com `+crt-static`:** vale para todo o workspace no Windows, inclusive o `sun`.
- **WebRTC pré-compilado:** a primeira compilação baixa cerca de 500 MB, sem conferência de hash pelo SDK. A CI precisa guardar isso em cache.
- **Caminhos longos:** o compilador C++ falhou com um `CARGO_TARGET_DIR` muito longo. O `target/` normal do repositório deve caber, e o plano confere isso.

## Critérios de aceite

**Testes automatizados:**

- [x] `sun/tests/voice.rs`: a rota nova devolve a identidade, as fontes e o `canSubscribe` certos, e recusa sem autenticação, em sala inexistente e sem LiveKit.
- [x] `moon/src-tauri/tests/capture.rs`: a lista não inclui o próprio Orbit; a conversão de quadros e o tamanho do áudio, em blocos de 10 ms, passam com dados fixos.
- [x] `useVoiceRoom.test.ts` e `room`: `<id>#screen` vira a marcação da pessoa, e quem transmite não assina a própria transmissão.
- [x] Testes de front do seletor: lista, escolha, cancelamento e erro.
- [x] `/verify` verde.
- [ ] CI verde com o WebRTC em cache, que só roda com um PR aberto.

**No app real, em modo de build, com `sun` e LiveKit reais, por script.** O alvo é uma janela de teste e um processo que toca um tom conhecido, nunca os aplicativos do usuário:

- [x] A janela de teste chega à outra janela do app, com quadros novos e a imagem certa.
- [x] O tom do processo escolhido chega a quem assiste.
- [x] O tom de **outro** processo, tocando ao mesmo tempo, não chega.
- [x] Antes de **Assistir** e depois de parar, não chegam bytes de vídeo nem de áudio da transmissão.
- [x] Fechar a janela de teste encerra a transmissão, e as duas janelas do app ficam certas.
- [x] Sair da sala e fechar o app encerram a publicação no LiveKit, sem processo nem conexão sobrando.
- [x] O token da transmissão não aparece nos logs.

**Verificação pelo usuário**, porque um script não julga a experiência:

- [ ] Transmitir um jogo real em outro PC via Tailscale, com imagem legível e áudio sincronizado.
- [ ] Confirmar que a música, as notificações e as vozes da sala não vão junto.
- [ ] Ver se o desempenho do jogo continua aceitável com o encoder em software.

## Verificação automatizada

Feita em 2026-11-04, no modo de build, com duas janelas reais, o core Rust real e `sun` e LiveKit reais pelo IP do Tailscale, em portas próprias. O alvo foi uma janela de teste tocando 1 kHz, com outro processo tocando 1,5 kHz ao mesmo tempo. Todas as checagens passaram:

- a janela chegou em VP8, com a imagem certa e quadros novos;
- o tom escolhido chegou (-60,8 dB), e o do outro processo não (-163 dB, abaixo do ruído);
- antes de **Assistir** e depois de parar, não chegaram bytes de vídeo, e o áudio da transmissão saiu;
- assistir de novo à mesma transmissão funcionou;
- `<id>#screen` não apareceu como outra pessoa, e quem transmite não recebeu a própria transmissão;
- fechar a janela transmitida encerrou a transmissão, com o aviso;
- sair da sala encerrou, e fechar o app pelo X encerrou em 201 ms, com o processo terminando;
- nenhum token apareceu nos logs.

**Exceção ao alvo de teste:** para exercitar a tela inteira, o script publicou a tela real no LiveKit local por alguns segundos. Ninguém assinou, e nada foi gravado. Essa etapa só confirmou que a publicação começa e termina, sem conferir a imagem.

**O que não cobre:**

- um jogo real, inclusive os com anti-cheat, que podem bloquear a captura;
- o limite de 4 Mbps, porque a janela de teste usa bem menos que isso;
- o sincronismo entre imagem e áudio;
- o desempenho com o VP8 em software durante um jogo;
- duas máquinas diferentes.

## Plano de implementação

Cada passo termina com `/verify` verde.

1. **ADR 0017** e a configuração do projeto: dependências, `crt-static` e cache na CI. Confirma que tudo compila no `target/` normal.
2. **Prova do áudio:** capturar um processo e ignorar outro, em um módulo isolado com teste.
3. **Prova do vídeo:** capturar a janela de teste e converter os quadros.
4. `sun`: rota do token, testes e `docs/protocol.md`.
5. Core: publicar vídeo e áudio pelo SDK, com os comandos e o evento.
6. Front: seletor, agrupamento de `#screen` e áudio ao assistir.
7. Verificação por script no app real.
8. Remover a transmissão pelo webview, atualizar a documentação e pedir a verificação do usuário.

## Decisões

- **Encoding em hardware:** fica para uma spec própria, e o item continua aberto no roadmap.
- **Transmissão pelo webview:** removida.
- **Tela inteira:** sem áudio.

## Questões em aberto

Nenhuma.
