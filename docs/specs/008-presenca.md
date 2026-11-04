# 008. Presença (online/offline)

- **Status:** Concluída
- **Fase:** 2 (ver `docs/roadmap.md`)

## Objetivo

Depois de entrar, o usuário vê quem do grupo está online e quem está offline, e a lista muda na hora quando alguém entra ou sai. Quem perde a conexão sem avisar, como em uma queda de energia ou de internet, também passa a aparecer como offline em cerca de um minuto. Este é o último item da Fase 2.

## Fora de escopo

- **Status manual e inatividade:** "ausente", "não perturbe" e "invisível".
- **"Visto por último":** a presença fica só em memória, nada vai para o banco.
- **"Digitando..."**.
- **Perfil, avatar e clique em uma pessoa:** a lista só mostra o nome e o estado.
- **Reconexão automática:** continua valendo o **Tentar de novo** da spec 004.

## Mudanças

### Protocolo (`shared` + `docs/protocol.md`)

```rust
/// Uma conta e se ela está conectada agora.
pub struct UserPresence {
    pub user: User,
    pub online: bool,
}

pub enum ServerMessage {
    // ...
    /// Todas as contas e o estado de cada uma. Só para a conexão que acabou de entrar.
    PresenceSnapshot { users: Vec<UserPresence> },
    /// Alguém ficou online, para todas as conexões.
    UserOnline { user: User },
    /// Alguém ficou offline, para todas as conexões.
    UserOffline { user: User },
}
```

No fio:

```json
{ "type": "presence_snapshot", "users": [{ "user": { "id": 1, "username": "fulano" }, "online": true }] }
{ "type": "user_online", "user": { "id": 2, "username": "ciclano" } }
{ "type": "user_offline", "user": { "id": 2, "username": "ciclano" } }
```

**Regras:**

- Logo depois do `auth_ok`, a conexão recebe `presence_snapshot`, antes de qualquer outra mensagem publicada.
- Uma pessoa está online enquanto tiver **pelo menos uma** conexão autenticada. Abrir uma segunda janela não anuncia nada, e fechar uma delas também não; só o fechamento da última anuncia `user_offline`.
- `user_online` também chega a quem acabou de entrar. Receber um estado que já se conhece não muda nada.
- Uma conta nova aparece para os outros quando conecta pela primeira vez: o `user_online` traz o `User` completo.
- **Conexão morta:** o servidor envia um ping de WebSocket a cada **20 segundos**. Sem receber nada do cliente por **60 segundos**, ele encerra a conexão, com o código 1001 (*going away*). O webview responde a esses pings sozinho, então o `moon` não precisa de código para isso.

### Dados (SQLite / migrations)

Sem migration. As contas vêm da tabela `users`, e quem está online fica só em memória: se o `sun` reiniciar, todas as conexões caem junto.

### `sun`

- `presence.rs` (novo): `Presence`, um campo novo do `AppState`, com um contador de conexões por usuário atrás de um `std::sync::Mutex`.
  - `connect(user)` devolve um `PresenceGuard`, e quando esse guard é destruído a conexão é descontada. Assim, qualquer saída do loop conta, inclusive por erro;
  - a publicação no `hub` acontece **com o lock na mão**, sem nenhum `.await`, para os anúncios saírem na mesma ordem das mudanças.
- `ws.rs`: depois do `auth_ok`, nesta ordem:
  1. `connect(user)`, que publica `user_online` se for a primeira conexão;
  2. lê as contas e quem está online e envia `presence_snapshot`. Uma falha ao ler o banco fecha a conexão com o código 1011 (*internal error*), e o cliente usa **Tentar de novo**;
  3. entra no loop, que ganha um terceiro braço no `select!`: o intervalo do ping e o prazo do cliente.
- **Por que a ordem não engana ninguém:** a conexão assina o `hub` antes de tudo. Um anúncio pode chegar depois de um `presence_snapshot` que já o inclui, mas o último anúncio de cada pessoa sempre reflete o estado mais novo.
- `AppState` ganha `ws_ping_interval` e `ws_idle_timeout`, como o `ws_auth_timeout`, para os testes usarem valores curtos.
- Sem dependências novas.

### `moon` (UI e core Rust)

- `UserPresence` entra em `bindings.ts`.
- `features/chat/ChatLayout.tsx`: guarda a presença em estado, a partir das mensagens do WebSocket. Ela não passa pela TanStack Query, porque não vem de uma chamada REST (ADR 0012).
- `features/chat/MemberList.tsx` (novo): coluna à direita da conversa (ver Questões em aberto, item 2).
  - seções **Online — N** e **Offline — N**, cada uma em ordem alfabética, incluindo o próprio usuário;
  - uma bolinha colorida ao lado do nome, e o estado também por extenso para leitores de tela, para não depender só da cor;
  - sem o `presence_snapshot` ainda, mostra "Carregando...".
- `ChatLayout.test.tsx` e `App.test.tsx` passam a considerar as mensagens novas.

## Critérios de aceite

- [x] `shared`: os testes de formato do fio de `UserPresence`, `PresenceSnapshot`, `UserOnline` e `UserOffline` passam.
- [x] `sun/tests/presence.rs::snapshot_lists_all_accounts`: quem entra recebe, logo depois do `auth_ok`, todas as contas, com o estado certo, incluindo ele próprio como online.
- [x] `sun/tests/presence.rs::connect_announces_online`: quem já está conectado recebe `user_online` quando outra pessoa entra.
- [x] `sun/tests/presence.rs::disconnect_announces_offline`: quem continua conectado recebe `user_offline` quando a outra pessoa sai.
- [x] `sun/tests/presence.rs::second_connection_is_silent`: com duas conexões da mesma conta, a segunda não anuncia nada, fechar uma não anuncia nada, e fechar a última anuncia `user_offline`.
- [x] `sun/tests/presence.rs::failed_auth_is_not_announced`: uma tentativa com token inválido não anuncia nada.
- [x] `sun/tests/presence.rs::dead_client_goes_offline`: com prazos curtos, um cliente que para de responder é desconectado, e os outros recebem `user_offline`.
- [x] `sun/tests/presence.rs::responsive_client_stays_online`: com prazos curtos, um cliente que responde aos pings continua conectado depois de passar o prazo.
- [x] `sun/tests/messages.rs` e os demais testes de WebSocket continuam passando, com os helpers ignorando as mensagens de presença.
- [x] `moon/src/bindings.ts` contém `UserPresence` e as mensagens novas.
- [x] `moon/src/features/chat/MemberList.test.tsx`:
  - separa online e offline, com a contagem, em ordem alfabética;
  - cada pessoa tem o estado por extenso;
  - sem dados, mostra "Carregando...".
- [x] `moon/src/features/chat/ChatLayout.test.tsx`:
  - `presence_snapshot` preenche a lista;
  - `user_online` e `user_offline` movem a pessoa de seção;
  - `user_online` de uma conta desconhecida a adiciona;
  - um estado repetido não duplica a pessoa.
- [x] **Verificação manual** com um `sun` real e duas janelas do app, com contas diferentes: entrar e sair aparece na outra janela; duas janelas da mesma conta só ficam offline ao fechar a última; encerrar o processo do app à força aparece como offline.
- [x] **Verificação manual** do layout: a conversa e a coluna cabem na janela padrão do app, sem rolagem horizontal.
- [x] `/verify` verde.

## Plano de implementação

Passos pequenos, cada um terminando com `/verify` verde.

1. **shared:** tipos, mensagens e testes de formato + `docs/protocol.md` + `specta_builder` e regeneração do `bindings.ts`.
2. **sun, presença:** `presence.rs`, com testes de unidade da contagem.
3. **sun, conexão:** `presence_snapshot` e anúncios em `ws.rs`, `sun/tests/presence.rs` e ajuste dos helpers de teste.
4. **sun, conexão morta:** ping e prazo no loop, com os testes de prazo curto.
5. **moon:** estado em `ChatLayout`, `MemberList.tsx`, CSS e testes.
6. **Verificação manual.**
7. **docs:** `docs/architecture.md`, `sun/CLAUDE.md`, `moon/CLAUDE.md`, roadmap (marcar "Presença" e a Fase 2), `docs/specs/README.md` e esta spec como `Concluída`.

## Verificação manual

Feita em 2026-11-04 em três janelas reais do app (WebView2), cada uma com a própria pasta de dados, dirigidas por script via depuração remota, contra um `sun` real com banco temporário. Passaram:

- quem entra aparece como online na outra janela;
- com duas janelas da mesma conta, a segunda não gera anúncio, encerrar o processo de uma não muda nada, e encerrar o da última mostra a pessoa como offline;
- uma janela parada por mais de 2 minutos continua conectada, ou seja, o WebView2 responde aos pings sozinho;
- a conversa e a coluna cabem em 1280x800, sem rolagem horizontal.

Encerrar o processo fecha a conexão TCP, então o servidor percebe na hora. O caso de uma conexão que morre sem avisar, como em uma queda de rede, só foi exercitado em `dead_client_goes_offline`, com prazos curtos. Também não cobre o build de release nem a conexão por outro PC via Tailscale, que ficam para a Fase 6.

## Decisões

Questões resolvidas com o usuário em 2026-11-04: as quatro propostas abaixo foram aceitas.

## Questões em aberto (originais)

1. **Quem aparece na lista:** proposto **todas as contas**, com online e offline separados. Em um grupo de 10 pessoas, a lista cabe na tela e mostra quem está faltando. A alternativa é mostrar só quem está online, o que é um pouco mais simples, mas some com o "offline" que o roadmap pede.
2. **Onde fica a lista:** proposto **uma coluna à direita da conversa**, como no Discord. A lateral esquerda já tem canais, **Novo canal** e **Sair**, e colocar a lista lá apertaria tudo. O custo é a conversa ficar uns 180 px mais estreita.
3. **Tempo para detectar uma queda:** proposto **ping a cada 20 segundos e prazo de 60 segundos**. Prazos menores detectam mais rápido, mas derrubariam quem tem uma oscilação curta de rede, e hoje voltar exige clicar em **Tentar de novo**.
4. **Oscilação ao reconectar:** proposto **não esconder**. Se alguém recarrega ou reconecta, os outros veem offline e online em seguida. Esperar alguns segundos antes de anunciar a saída evitaria isso, mas também atrasaria todo anúncio de saída e deixaria o código maior.
