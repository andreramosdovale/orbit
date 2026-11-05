# 014. Encoding em hardware da transmissão

- **Status:** Em andamento
- **Fase:** 5 (ver `docs/roadmap.md`)

## Objetivo

Quando o PC de quem transmite tem um encoder na placa de vídeo, a transmissão passa a usá-lo, o que libera o processador para o jogo. Sem esse encoder, ou se ele falhar ao iniciar, a transmissão continua em VP8 no processador, sem a pessoa precisar fazer nada.

## Fora de escopo

- **Passar o quadro da captura direto para a placa, sem cópia:** o quadro passa pela memória do sistema. Fica para depois, se o desempenho pedir.
- **H.265 e AV1**, que nem todo espectador consegue decodificar.
- **Escolher o encoder ou a qualidade na interface.**
- **Trocar para VP8 no meio de uma transmissão**, se o encoder falhar depois de iniciar: a transmissão termina com um aviso, e basta iniciar de novo.

## Prova técnica

Feita em 2026-11-04, em um projeto de teste fora do repositório, com vídeo sintético em 1080p a 30 quadros por segundo e 4 Mbps, em um PC com NVIDIA GeForce RTX 4060 Ti e AMD Radeon integrada.

| Pergunta | Resultado |
|---|---|
| O SDK do LiveKit publica H.264 já codificado no Windows? | Sim, com `VideoEncoderBackend::PreEncoded` e `capture_encoded_frame`, que existem no `livekit` 0.10. |
| O encoder da NVIDIA funciona por Media Foundation? | Sim: 758 de 759 quadros codificados, com baixa latência, taxa constante e quadro-chave sob pedido aceitos. |
| Quanto do processador o envio usa? | 6% de um núcleo. O VP8 mediu 74%, mas com o receptor no mesmo processo, então os números não são diretamente comparáveis. |
| O webview do app decodifica? | Sim: 1920x1080, 121 quadros em 4 segundos, nenhum descartado, decodificado pela própria placa. |
| O encoder da AMD funciona? | **Não, neste PC.** Sem dispositivo, recusou a configuração (`0xC00D6D76`). Com o dispositivo D3D11 da placa, com quadros na memória ou em texturas, falhou com `0x80004005`. Fica em VP8 (ver Decisões). |
| O encoder da Intel funciona? | **Não testado:** este PC não tem um. |

O receptor nativo usado no teste informou um quadro de 2x2 para esse vídeo, enquanto o webview mostrou 1920x1080. Como quem assiste usa o webview, isso não afeta o Orbit, mas fica registrado.

**O que a prova não cobre:** o efeito no desempenho de um jogo real e a limitação de banda pedida pelo LiveKit sob uma rede ruim. O segundo item entra nos testes desta spec.

## Mudanças

### Protocolo (`shared` + `docs/protocol.md`)

As rotas e os tokens não mudam. `startScreenShare` passa a devolver qual encoder foi usado:

```text
startScreenShare(target, url, token) -> Result<VideoEncoderKind, string>
VideoEncoderKind = "hardware" | "software"
```

### Dados (SQLite / migrations)

Nenhuma.

### `sun`

Nenhuma.

### `moon` (UI e core Rust)

**Core Rust, em `src-tauri/src/capture/encoder.rs`,** com o `unsafe` isolado nesse arquivo:

- Procura os encoders H.264 em hardware por `MFTEnumEx`, abre o primeiro que funcionar, codifica um quadro de teste e só então publica. Assim, a escolha entre H.264 e VP8 acontece antes de a transmissão começar.
- Usa H.264 Constrained Baseline, baixa latência e taxa constante, até 1080p, 30 quadros por segundo e 4 Mbps.
- Atende os pedidos do LiveKit: quadro-chave para quem começa a assistir e redução de banda em uma rede ruim.
- **AMD e Intel:** o código tenta associar o encoder a um dispositivo D3D11 da mesma placa. Se ainda assim não funcionar, o próximo encoder é tentado e, por fim, o VP8.

**Captura e publicação:**

- A captura continua a mesma e passa a entregar os quadros ao encoder em NV12.
- O tamanho do encoder é fixo. Se a janela mudar de tamanho, ele é recriado no novo tamanho, com um quadro-chave.
- `publish.rs` escolhe entre os dois caminhos. O caminho em VP8 não muda.

**Front:**

- O aviso passa a dizer "Você está transmitindo pela placa de vídeo" ou "Você está transmitindo pelo processador". Isso ajuda a entender um desempenho ruim e permite conferir o encoder usado.
- Quem assiste não muda nada.

**Dependência:** o crate `windows` 0.62 passa a ser dependência direta do `moon`. Ele já vem junto de outras dependências, então o tamanho do app quase não muda.

## Critérios de aceite

**Testes automatizados:**

- [x] `moon/src-tauri/tests/capture.rs`: a conversão para NV12 e a escolha do caminho, em que uma falha ao abrir o encoder leva ao VP8, passam com dados fixos.
- [x] `useVoiceRoom.test.ts` e `VoicePanel.test.tsx`: o aviso mostra o encoder devolvido pelo core.
- [x] `/verify` verde.
- [ ] CI verde, que só roda com um PR aberto.

**Com o hardware real**, em testes `ignore`, que ficam fora da CI:

- [x] O encoder escolhido codifica quadros, e o primeiro é um quadro-chave.
- [x] Um pedido de quadro-chave é atendido.
- [x] Um pedido de redução de banda baixa a taxa medida.
- [x] Recriar o encoder com outro tamanho funciona.

**No app real, por script**, com a janela de teste, em duas janelas e com `sun` e LiveKit reais:

- [x] Em um PC com encoder em hardware, a outra janela recebe H.264 no tamanho certo, com quadros novos, e o aviso diz "pela placa de vídeo".
- [x] Sem encoder em hardware, simulado por uma variável de ambiente de teste, a outra janela recebe VP8, e o aviso diz "pelo processador".
- [x] Redimensionar a janela transmitida não derruba a transmissão, e o vídeo muda de tamanho.
- [x] Quem começa a assistir depois recebe imagem em até 3 segundos.
- [x] Parar, fechar a janela e fechar o app continuam encerrando a transmissão e liberando o encoder.

**Verificação pelo usuário**, junto do teste da spec 013:

- [ ] Transmitir um jogo real e confirmar que o desempenho fica melhor que em VP8.
- [ ] Confirmar a imagem em outro PC via Tailscale.

## Verificação automatizada

Feita em 2026-11-04, no modo de build, com duas janelas reais, o core Rust real e `sun` e LiveKit reais, usando a janela de teste:

- **Pela placa de vídeo, com o encoder da NVIDIA:** a outra janela recebeu H.264, com a imagem certa e quadros novos, e o aviso disse "pela placa de vídeo". A primeira imagem chegou em cerca de 170 ms.
- **Sem encoder, simulado por variável de ambiente:** a outra janela recebeu VP8, e o aviso disse "pelo processador". Esta rodada foi feita antes da correção do áudio descrita abaixo.
- **Só com os encoders da AMD disponíveis:** todos falharam, e `HardwareEncoder::start` devolveu `Unavailable`, que é o que leva ao VP8.
- **Nas duas formas:** redimensionar mudou o tamanho do vídeo sem derrubar a transmissão, o áudio seguiu isolado, e fechar o app encerrou em cerca de 190 ms.
- **Encoder real, em `capture.rs`:** o primeiro quadro foi chave, e o pedido de quadro-chave e a troca de tamanho funcionaram. Ao pedir 0,5 Mbps com um vídeo difícil de comprimir, a taxa caiu de 4,4 para 1,6 Mbps: diminuiu, mas não chegou ao valor pedido.

O teste encontrou um defeito: quando a transmissão assistida terminava sozinha, o elemento de áudio dela podia ficar na página. Agora ele é removido na hora, e duas rodadas seguidas passaram.

**O que não cobre:** um jogo real, a qualidade em uma rede ruim, o encoder da Intel, outras placas da AMD, e duas máquinas diferentes.

## Plano de implementação

Cada passo termina com `/verify` verde.

1. ADR 0018 e `capture/encoder.rs`, com os testes de hardware.
2. Tentar o encoder da AMD com o dispositivo D3D11 da placa certa. Se não funcionar com pouco esforço, ele fica para o VP8, e registro isso na spec.
3. Ligar o encoder à captura e à publicação, com o VP8 como reserva.
4. Devolver o encoder usado e mostrar o aviso no front.
5. Verificação por script, documentação e pedido da verificação do usuário.

## Decisões

- **AMD:** fica em VP8 por enquanto. Passar por isso exigiria investigar o driver, além do "pouco esforço" previsto no plano. O código continua tentando, então outras placas da AMD ainda podem funcionar.

## Questões em aberto

Nenhuma.
