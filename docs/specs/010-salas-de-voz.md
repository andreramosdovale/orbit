# 010. Salas de voz

- **Status:** Em andamento
- **Fase:** 3 (ver `docs/roadmap.md`)

## Objetivo

O host instala e sobe o LiveKit com um script. Depois de entrar no Orbit, cada pessoa escolhe uma sala de voz, entra com um clique e conversa com quem estiver lá, podendo se silenciar e vendo quem está falando. A voz não atrapalha o chat: dá para trocar de canal de texto e continuar na sala.

## Fora de escopo

- **Ver quem está em uma sala sem ter entrado nela** (ver Decisões, item 2).
- **Escolha de microfone e de saída de áudio:** usa os padrões do Windows.
- **Apertar para falar, volume por pessoa e silenciar outra pessoa.**
- **Câmera e transmissão de tela:** Fases 4 e 5. O token desta spec só permite publicar o microfone.
- **Renomear ou apagar salas e limitar quem entra:** como nos canais de texto (ADR 0007).
- **Iniciar o Orbit e o LiveKit com o Windows e instalador do host:** Fase 6.

## Mudanças

### Protocolo (`shared` + `docs/protocol.md`)

```rust
/// Sala de voz.
pub struct VoiceChannel { pub id: u32, pub name: String }

/// Corpo de `POST /voice-channels`.
pub struct CreateVoiceChannelRequest { pub name: String }

/// O necessário para entrar em uma sala no LiveKit.
pub struct VoiceJoin {
    /// Token de acesso, válido só para esta sala.
    pub token: String,
    /// Porta do LiveKit no mesmo host do `sun`.
    pub livekit_port: u16,
}

pub enum ServerMessage {
    // ...
    /// Sala nova, enviada a todas as conexões.
    VoiceChannelCreated { channel: VoiceChannel },
}
```

| Método | Rota | Auth | Sucesso | Erros |
|---|---|---|---|---|
| `GET` | `/voice-channels` | `Bearer` | `200` + `VoiceChannel[]`, na ordem de criação | `401` / `500` |
| `POST` | `/voice-channels` | `Bearer` | `201` + `VoiceChannel` | `400 invalid_channel_name` / `401` / `409 channel_name_taken` / `500` |
| `POST` | `/voice-channels/{id}/token` | `Bearer` | `200` + `VoiceJoin` | `400 invalid_request` / `401` / `404 unknown_channel` / `503 voice_unavailable` / `500` |

Código novo em `ApiErrorCode`: `VoiceUnavailable` (`"voice_unavailable"`), quando o `sun` subiu sem as chaves do LiveKit.

**Regras:**

- O nome da sala segue as regras do nome de canal de texto, e os nomes não conflitam entre texto e voz.
- O `sun` não sabe por qual endereço cada amigo o alcança, então devolve só a porta. O `moon` monta `ws://<host do servidor>:<livekit_port>`.
- **Token:** dura 10 minutos, o suficiente para conectar, e o SDK cuida da renovação depois disso. Identidade = id do usuário; nome = `username`; sala = `voice-<id>`; `canSubscribe` e `canPublish` ligados, com `canPublishSources: ["microphone"]`.

### Dados (SQLite / migrations)

Migration `0005_voice_channels.sql`:

```sql
CREATE TABLE voice_channels (
    id         INTEGER PRIMARY KEY,
    name       TEXT    NOT NULL UNIQUE,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

INSERT INTO voice_channels (name) VALUES ('geral');
```

Uma tabela separada evita que uma sala de voz receba mensagens de texto ou apareça na lista de canais de texto.

### `sun`

- `voice/mod.rs` (novo): rotas, com handlers finos.
- `voice/channels.rs`: listagem e criação, reaproveitando a validação de nome do `chat/channels.rs`. `POST` publica `voice_channel_created` no `hub`.
- `voice/token.rs`: emissão do token com o `jsonwebtoken` que o `sun` já usa. A versão aprovada desta spec previa o `livekit-api`, mas ele traria 18 crates para assinar um JWT (ADR 0014).
- **Variáveis de ambiente novas:**

| Variável | Padrão | Descrição |
|---|---|---|
| `LIVEKIT_API_KEY` | nenhum | Chave do LiveKit |
| `LIVEKIT_API_SECRET` | nenhum | Segredo do LiveKit. Nunca vai para o log |
| `LIVEKIT_PORT` | `7880` | Porta devolvida ao `moon` |

- Sem as duas chaves, o `sun` sobe normalmente, avisa no log e responde `503 voice_unavailable` na rota do token: o chat não pode depender da voz. Só uma das duas é erro de configuração, e o `sun` não sobe.
- O `sun` não fala com o LiveKit, só assina tokens.

### Host (`scripts/`, `docs/host-setup.md`)

- `scripts/setup-livekit.ps1`:
  - baixa uma versão **fixa** do `livekit-server` para Windows (hoje, 1.13.11) para `livekit/`, conferindo um SHA-256 gravado no próprio script;
  - gera a chave e o segredo aleatórios e grava `livekit/livekit.yaml` e `livekit/sun.env`, sem sobrescrever os existentes;
  - cria as regras de firewall, que só aceitam a faixa do Tailscale (`100.64.0.0/10`). Esta é a única parte que pede administrador.
- `scripts/start-host.ps1`: sobe o LiveKit e o `sun` com as variáveis de `livekit/sun.env`.
- `.gitignore`: `livekit/`, porque contém o segredo.
- Configuração gerada:

```yaml
port: 7880
rtc:
  tcp_port: 7881
  udp_port: 7882
  use_external_ip: false
  ips:
    includes: [100.64.0.0/10]
keys:
  <chave>: <segredo>
```

Uma única porta UDP simplifica o firewall, e `ips.includes` faz o LiveKit anunciar só o endereço do Tailscale, em vez dos IPs da rede local do host.

- `docs/host-setup.md`: guia passo a passo para o host, incluindo como conferir se está funcionando.

### `moon` (UI e core Rust)

- **Dependência nova:** `livekit-client` (ADR 0002), carregado só ao entrar na primeira sala.
- `src-tauri/src/mic_permission.rs`: responde ao pedido do microfone (ADR 0015).
- `VoiceChannel`, `CreateVoiceChannelRequest` e `VoiceJoin` entram em `bindings.ts`.
- `features/voice/api.ts`: `listVoiceChannels`, `createVoiceChannel` e `requestVoiceToken`, pela TanStack Query (ADR 0012).
- `features/voice/room.ts`: isola o `livekit-client` atrás de uma interface pequena (`connect`, `setMuted`, `disconnect` e eventos), para os testes não dependerem de WebRTC.
- `features/voice/useVoiceRoom.ts`: estado da sala: `idle`, `connecting`, `connected`, `reconnecting` ou `error`, os participantes, quem está falando e o mute.
- Lateral do chat: seção **Salas de voz**, com a lista e **Nova sala**. Clicar em uma sala entra nela; entrar em outra sai da atual.
- `features/voice/VoicePanel.tsx`: enquanto estiver em uma sala, mostra o nome dela, os participantes, **Silenciar**/**Ativar microfone** e **Sair da sala**.
  - quem está falando ganha um contorno, e quem está silenciado, um ícone. Os dois estados também vêm por extenso para leitores de tela;
  - cada áudio recebido toca em um elemento `<audio>` criado pelo SDK e removido ao sair.
- **Erros mostrados na tela:** microfone negado ou inexistente, `voice_unavailable` ("A voz não está configurada neste servidor.") e LiveKit fora do ar.
- **Vida da voz:** o LiveKit não depende do WebSocket do chat. Uma queda do `/ws` não derruba a sala, e o SDK reconecta a voz por conta própria. **Sair** da conta encerra a sala.

## Critérios de aceite

- [x] `shared`: os testes de formato do fio de `VoiceChannel`, `CreateVoiceChannelRequest`, `VoiceJoin`, `VoiceChannelCreated` e `VoiceUnavailable` passam.
- [x] `sun/tests/voice.rs::lists_default_channel`: um servidor novo lista a sala `geral`.
- [x] `sun/tests/voice.rs::create_channel`: criar devolve `201`, publica `voice_channel_created` e aplica as regras do nome; um nome repetido devolve `409`.
- [x] `sun/tests/voice.rs::requires_auth`: sem token, as três rotas devolvem `401`.
- [x] `sun/tests/voice.rs::token_grants`: o token, decodificado com o segredo, traz a identidade, o nome, a sala certa, `canPublishSources: ["microphone"]` e `exp` em 10 minutos.
- [x] `sun/tests/voice.rs::token_for_unknown_channel`: uma sala inexistente devolve `404 unknown_channel`.
- [x] `sun/tests/voice.rs::token_without_keys`: sem as chaves, devolve `503 voice_unavailable`, e o resto do servidor funciona.
- [x] O teste de configuração do `sun` confirma que só uma das duas chaves é recusada.
- [x] O segredo do LiveKit não aparece no log nem em nenhuma resposta.
- [x] `moon/src/bindings.ts` contém os tipos e a mensagem novos.
- [x] `moon/src/features/voice/api.test.ts`: as chamadas usam as URLs certas, com o token.
- [x] `moon/src/features/voice/useVoiceRoom.test.ts`, com uma sala simulada:
  - entrar monta `ws://<host>:<porta>` a partir do servidor e do `livekit_port`;
  - entrar em outra sala desconecta da atual;
  - mute chama `setMuted`;
  - participantes e quem fala seguem os eventos;
  - microfone negado, `voice_unavailable` e falha de conexão viram erro legível;
  - desmontar desconecta.
- [x] `moon/src/features/voice/VoicePanel.test.tsx`: mostra participantes, quem fala, quem está silenciado, e os botões funcionam.
- [x] `moon/src/features/chat/ChatLayout.test.tsx`: `voice_channel_created` adiciona a sala sem duplicar; trocar de canal de texto não sai da sala.
- [x] `scripts/setup-livekit.ps1`: rodar duas vezes não troca a chave, e um arquivo baixado com SHA-256 errado é recusado.
- [x] **Verificação automatizada** no app real, com `sun` e LiveKit reais e duas janelas usando um microfone simulado do Chromium (`--use-fake-device-for-media-stream`): uma entra e a outra vê; o áudio de uma chega à outra (track assinada, com dados chegando); o indicador de fala acende; mute apaga; sair remove a pessoa da lista; matar o `/ws` não derruba a sala.
- [ ] **Verificação pelo usuário**, porque só uma pessoa consegue ouvir: duas pessoas, com microfones de verdade, conversam em uma sala, de preferência em PCs diferentes via Tailscale.
- [x] `/verify` verde.

## Plano de implementação

Passos pequenos, cada um terminando com `/verify` verde.

1. **Prova técnica, antes de tudo:** no WebView2 real, com a origem `http://tauri.localhost` do build e com a de dev, confirmar que `getUserMedia` funciona e como é o pedido de permissão do microfone, e conectar a um LiveKit real. Se isso exigir código no core Rust ou mudar a abordagem, a spec é revista antes de continuar.
2. **ADRs 0014 e 0015:** como o host sobe o LiveKit e guarda as chaves, por que só pela faixa do Tailscale e a permissão do microfone.
3. **Host:** `setup-livekit.ps1`, `start-host.ps1`, `.gitignore` e `docs/host-setup.md`.
4. **shared:** tipos, mensagem e testes de formato + `docs/protocol.md` + `specta_builder` e regeneração do `bindings.ts`.
5. **sun, salas:** migration, `voice/channels.rs`, rotas e testes.
6. **sun, token:** configuração, `voice/token.rs` e testes.
7. **moon, dados:** `api.ts`, lista na lateral e `voice_channel_created`, com testes.
8. **moon, sala:** `room.ts`, `useVoiceRoom.ts`, `VoicePanel.tsx`, CSS e testes.
9. **Verificação automatizada e, depois, a do usuário.**
10. **docs:** `docs/architecture.md`, `sun/CLAUDE.md`, `moon/CLAUDE.md`, roadmap, `docs/specs/README.md` e esta spec como `Concluída`.

## Verificação automatizada

Feita em 2026-11-04 em duas janelas reais do app no **modo de build** (origem `http://tauri.localhost`), dirigidas por script via depuração remota, com o microfone simulado do Chromium. O LiveKit foi instalado pelo `setup-livekit.ps1` real, com a configuração restrita ao Tailscale, e subiu pelo `start-host.ps1`. O `sun` escutou só no IP do Tailscale deste PC, e o app se conectou por esse endereço. Passaram:

- nenhum diálogo de permissão abriu;
- o LiveKit aceitou o token assinado pelo `sun`, e cada janela viu a outra, com o nome certo;
- o áudio chegou nos dois sentidos (cerca de 10 a 20 KB em 2 segundos), pelo endereço do Tailscale (`100.89.93.14`);
- o indicador de fala acendeu;
- silenciar marcou a pessoa na outra janela e zerou o áudio recebido, e ativar o trouxe de volta;
- trocar de canal de texto e derrubar o `/ws` do chat não tiraram ninguém da sala, e o áudio continuou;
- uma sala nova apareceu na outra janela, e entrar nela saiu da anterior;
- sair da sala e sair da conta removeram o painel e o áudio;
- o segredo do LiveKit não apareceu no log.

**O que não cobre:** microfone de verdade e ouvir o resultado; duas máquinas diferentes, porque as duas janelas estavam no mesmo PC; as regras de firewall, que exigem administrador (`-SkipFirewall`); o Ctrl+C do `start-host.ps1`.

## Resultado da prova técnica

Feita em 2026-11-04 no WebView2 real, com o LiveKit 1.13.11 para Windows e um microfone simulado do Chromium (`--use-fake-device-for-media-stream`).

**O que funcionou:**

- `http://tauri.localhost`, a origem do build, é um contexto seguro, então `getUserMedia` e `RTCPeerConnection` existem nela.
- Em `http://localhost:1420`, duas janelas entraram em uma sala de um LiveKit real: cada uma viu a outra, recebeu áudio (cerca de 15 a 19 KB em 2 segundos), com ICE estabelecido, e o evento de quem está falando chegou.
- O LiveKit roda no Windows. Ele avisa que o gerenciamento de capacidade não existe nessa plataforma, o que não importa para um grupo pequeno.

**O problema encontrado, no build:** sem código nativo, o WebView2 abre um diálogo próprio: "http://tauri.localhost quer / Use seus microfones / Bloquear / Permitir".

- **Permitir** fica guardado no perfil: ao reabrir o app, o microfone responde em 32 ms, sem novo pedido.
- **Bloquear** também fica guardado, e **não há como voltar atrás pelo app**: `getUserMedia` falha na hora com `NotAllowedError`, o diálogo não reaparece, e o WebView2 não tem tela de configurações.

**Saída testada:** o core Rust responde ao evento `PermissionRequested` do WebView2, permitindo o microfone. Em um perfil novo, não abre diálogo, e o microfone responde em 38 ms. Um perfil que já tinha bloqueado continua bloqueado, porque o evento não dispara nesse caso. Isso não afeta ninguém, já que a voz ainda não foi lançada e, com a resposta do core, o diálogo não aparece mais.

**O que a prova não cobre:**

- microfone de verdade e o bloqueio de microfone nas configurações de privacidade do Windows, que continuam podendo recusar o acesso e precisam da mensagem de erro;
- entrar em uma sala a partir da origem do build: só a origem de dev entrou de fato em uma sala. Na do build, foram testados o microfone e o alcance do LiveKit pelo WebSocket;
- se o diálogo aparece em dev: durante a prova, alguém pode ter clicado nos diálogos que abriram na tela, então esse resultado não é conclusivo. A verificação final precisa usar a origem do build.

## Decisões

Questões resolvidas com o usuário em 2026-11-04:

1. **Salas:** criadas pelo app, como os canais de texto, com a `geral` já pronta.
2. **Ver quem está em uma sala sem entrar:** fica para uma spec depois desta. Até lá, só dá para saber quem está em uma sala entrando nela.
3. **Acesso:** o firewall e o LiveKit aceitam só a faixa do Tailscale (`100.64.0.0/10`). O Tailscale precisa estar ligado também no PC do host, e o script tem a opção `-Dev` para testar sem ele.
4. **Uma spec só:** a parte do servidor sozinha não entrega nada que o grupo possa usar.
5. **Permissão do microfone:** o core Rust permite o microfone, sem diálogo, só para as origens do próprio app ([ADR 0015](../decisions/0015-permissao-do-microfone.md)).

## Questões em aberto

Nenhuma.
