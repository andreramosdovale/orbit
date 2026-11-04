# 007. Histórico paginado

- **Status:** Concluída
- **Fase:** 2 (ver `docs/roadmap.md`)

## Objetivo

Ao abrir um canal, o usuário vê as mensagens mais recentes que estão guardadas no servidor, em vez de um canal vazio, e pode voltar para as mais antigas aos poucos, até o começo da conversa: elas carregam sozinhas ao rolar até o topo, e um botão faz o mesmo. Cada mensagem mostra quem enviou, o dia e o horário. Isso fecha a limitação deixada pela spec 006, em que reabrir o app mostrava os canais vazios.

## Fora de escopo

- **Busca, salto para uma data ou para uma mensagem específica.**
- **Cache em disco:** o cache fica só em memória e dura uma conexão ([ADR 0012](../decisions/0012-tanstack-query-no-front.md)). Reabrir o app ou reconectar carrega tudo de novo.
- **Separadores de dia na lista.**
- **Login e cadastro com TanStack Query:** são chamadas únicas, sem nada para guardar. Continuam com `request()` direto.
- **Contador de não lidas, marcador de "última lida" e botão "ir para o fim".**
- **Reconexão automática:** continua valendo o **Tentar de novo** da spec 004. Ele recria a tela, então o histórico é carregado de novo e o que chegou durante a queda aparece.
- **Presença:** próximo item do roadmap.

## Mudanças

### Protocolo (`shared` + `docs/protocol.md`)

Rota nova:

| Método | Rota | Auth | Sucesso | Erros |
|---|---|---|---|---|
| `GET` | `/channels/{id}/messages?before=<id>` | `Bearer` | `200` + `MessagePage` | `400 invalid_request` / `401 unauthorized` / `404 unknown_channel` / `500 internal` |

```rust
/// Uma página do histórico de um canal.
pub struct MessagePage {
    /// Da mais antiga para a mais nova.
    pub messages: Vec<ChatMessage>,
    /// Existem mensagens mais antigas que as desta página.
    pub has_more: bool,
}
```

Código novo em `ApiErrorCode`: `UnknownChannel` (`"unknown_channel"`).

No fio:

```json
{
  "messages": [
    { "id": 7, "channel_id": 1, "author": { "id": 1, "username": "fulano" }, "content": "oi", "created_at": "2026-11-03T12:00:00.000Z" }
  ],
  "has_more": true
}
```

**Regras:**

- Sem `before`: as **50** mensagens mais recentes do canal (`sun::MESSAGE_PAGE_SIZE`).
- Com `before`: as 50 mais recentes entre as de `id` menor que `before`. O `before` não precisa ser o id de uma mensagem existente.
- O cursor é o `id`, e não um deslocamento: uma mensagem nova chegando entre duas páginas não repete nem pula nada.
- O tamanho da página é fixo; o cliente não escolhe.
- `id` do canal ou `before` que não sejam um inteiro de 0 a 2³²−1 recebem `400 invalid_request`.
- Canal sem mensagens: `200` com `messages` vazio e `has_more: false`.

### Dados (SQLite / migrations)

Sem migration. O índice `messages_channel_id (channel_id, id)`, da spec 006, já atende a consulta.

### `sun`

- `chat/messages.rs`: `message_page(db, channel_id, before)`.
  - confere se o canal existe (`MessageError::UnknownChannel`);
  - busca `MESSAGE_PAGE_SIZE + 1` linhas com `JOIN users`, em ordem decrescente de `id`. A linha a mais só serve para saber o `has_more` e é descartada;
  - inverte a ordem antes de devolver.
- `chat/mod.rs`: rota e handler fino. O `AuthUser` vem antes dos extratores de caminho e de query, para o `401` vir antes de qualquer validação. Rejeições desses extratores viram `ApiError::InvalidRequest`.
- `ApiError` ganha `UnknownChannel`, com status `404`.
- O conteúdo das mensagens não vai para o log. Sem dependências novas.

### `moon` (UI e core Rust)

- `MessagePage` entra no `specta_builder` e em `bindings.ts`.
- **Dependência nova:** `@tanstack/react-query` ([ADR 0012](../decisions/0012-tanstack-query-no-front.md)), usada nas chamadas de chat: canais (`useQuery` e `useMutation`) e histórico (`useInfiniteQuery`).
- `features/chat/api.ts`: `listMessages(server, token, channelId, before?)`.
- `features/chat/ChatLayout.tsx`:
  - cria o `QueryClient`, que dura o mesmo que a tela, ou seja, uma conexão;
  - a lista de canais e **Novo canal** passam para a TanStack Query, com o mesmo comportamento na tela;
  - guarda as mensagens do WebSocket por canal, como já faz.
- `features/chat/ChannelMessages.tsx` (novo): uma instância por canal.
  - carrega a primeira página na **primeira vez** que o canal é selecionado. Voltar a ele usa o cache, porque o WebSocket o mantém atualizado;
  - o `before` da próxima página é o menor `id` da última página recebida;
  - junta as páginas e as mensagens do WebSocket **por `id`**, sem repetir e em ordem crescente: uma mensagem pode chegar pelos dois caminhos enquanto a página carrega;
  - `unauthorized` encerra a sessão (`onSessionExpired`), como nas outras chamadas.
- `features/chat/MessageList.tsx`:
  - cada mensagem mostra autor, dia e horário locais (`DD/MM/AAAA HH:MM`) e texto, inclusive as recebidas ao vivo;
  - mostra "Carregando mensagens..." no primeiro carregamento e só depois dele "Nenhuma mensagem ainda.";
  - erro no carregamento: mostra a mensagem e **Tentar de novo**;
  - **rolagem infinita:** chegar a até 80 px do topo carrega a página anterior, uma por vez. Depois de um erro, não tenta de novo sozinha, para não repetir a chamada a cada movimento;
  - botão **Carregar mensagens anteriores** no topo da lista, enquanto `has_more` for verdadeiro e desativado durante o carregamento. Ele continua necessário quando uma página não enche a tela, porque aí não há o que rolar;
  - **rolagem:**
    - abrir um canal: vai para o fim;
    - carregar mensagens anteriores: mantém na tela as mensagens que o usuário estava lendo;
    - mensagem nova: vai para o fim só se ele já estava perto do fim (até 80 px) ou se foi ele quem enviou. Sem isso, quem está lendo o histórico seria arrastado para baixo a cada mensagem.
- `ChatLayout.test.tsx`: os testes existentes passam a simular também a resposta de `GET /channels/{id}/messages`.

## Critérios de aceite

- [x] `shared`: o teste de formato do fio de `MessagePage` e de `ApiErrorCode::UnknownChannel` passa.
- [x] `sun/tests/history.rs::latest_page`: com 51 mensagens no canal, sem `before`, vêm as 50 mais recentes em ordem crescente de `id`, com autor, conteúdo e horário, e `has_more: true`.
- [x] `sun/tests/history.rs::exact_page_has_no_more`: com exatamente 50 mensagens, `has_more` é `false`.
- [x] `sun/tests/history.rs::before_walks_to_the_start`: seguindo o `before` a partir de 120 mensagens, as páginas têm 50, 50 e 20 itens, cobrem todas as mensagens sem repetir, e a última tem `has_more: false`.
- [x] `sun/tests/history.rs::new_message_does_not_shift_pages`: uma mensagem gravada entre duas páginas não altera a segunda.
- [x] `sun/tests/history.rs::only_requested_channel`: mensagens de outro canal não aparecem nem contam para o `has_more`.
- [x] `sun/tests/history.rs::empty_channel`: canal sem mensagens devolve `200`, lista vazia e `has_more: false`.
- [x] `sun/tests/history.rs::unknown_channel`: canal inexistente devolve `404 unknown_channel`.
- [x] `sun/tests/history.rs::invalid_params`: `before=abc`, `before=-1` e `/channels/abc/messages` devolvem `400 invalid_request`.
- [x] `sun/tests/history.rs::requires_auth`: sem token, com token inválido e com parâmetro inválido sem token, a resposta é `401 unauthorized`.
- [x] `moon/src/bindings.ts` contém `MessagePage` e `unknown_channel`.
- [x] `moon/src/features/chat/api.test.ts`: `listMessages` chama a URL certa com e sem `before`, com o token.
- [x] `moon/src/features/chat/ChatLayout.test.tsx`:
  - selecionar um canal carrega e mostra o histórico;
  - voltar a um canal já carregado não busca de novo;
  - **Carregar mensagens anteriores** usa o menor `id` da última página como `before` e coloca as mensagens antes das atuais;
  - o botão não aparece com `has_more: false`;
  - uma mensagem recebida pelo WebSocket e também pela página aparece uma vez só;
  - a resposta de um canal que deixou de estar selecionado não aparece no canal atual;
  - erro no carregamento mostra a mensagem, e **Tentar de novo** busca de novo;
  - `unauthorized` no histórico chama `onSessionExpired`.
- [x] `moon/src/features/chat/MessageList.test.tsx` (com medidas de rolagem simuladas):
  - mensagem nova rola até o fim quando se está perto do fim;
  - mensagem nova de outra pessoa não rola quando se está lendo acima;
  - mensagem nova do próprio usuário sempre rola;
  - carregar mensagens anteriores mantém a posição das mensagens na tela;
  - chegar perto do topo pede a página anterior; longe do topo, durante um carregamento ou depois de um erro, não pede;
  - cada mensagem mostra autor, dia e horário (`03/11/2026 14:32`).
- [x] **Verificação manual** com um `sun` real e o app: reabrir o app mostra as mensagens anteriores; com mais de 100 mensagens, rolar até o topo carrega as páginas até a primeira mensagem sem a tela pular, e o botão faz o mesmo; uma mensagem recebida enquanto se lê o histórico não muda a rolagem.
- [x] `/verify` verde.

## Plano de implementação

Passos pequenos, cada um terminando com `/verify` verde.

1. **shared:** `MessagePage`, `UnknownChannel` e testes de formato + `docs/protocol.md` + `specta_builder` e regeneração do `bindings.ts`.
2. **sun:** `message_page`, rota, `ApiError::UnknownChannel` e `sun/tests/history.rs`.
3. **moon, dados:** `listMessages` e o estado por canal no `ChatLayout`, com a junção por `id` e os testes.
4. **moon, tela:** carregamento, erro, botão e rolagem no `MessageList`, CSS e testes.
5. **Verificação manual.**
6. **docs:** `docs/architecture.md`, `sun/CLAUDE.md`, `moon/CLAUDE.md`, roadmap (marcar "Histórico paginado"), `docs/specs/README.md` e esta spec como `Concluída`.

## Verificação manual

Feita em 2026-11-04 na janela real do app (WebView2), dirigida por script via depuração remota, contra um `sun` real com banco temporário, 130 mensagens e duas contas. Passaram:

- reabrir o app mostra as 50 mensagens mais recentes, no fim da lista, com autor, dia e horário;
- rolar até o topo carrega as páginas até a primeira mensagem, na ordem e sem repetir, e a mensagem que estava na tela não sai do lugar;
- sem mais páginas, o botão some;
- o botão, usado sem rolar, carrega a página anterior;
- quem está lendo o histórico não é levado para o fim por uma mensagem de outra pessoa; a própria mensagem leva, e uma mensagem nova também quando já se está no fim;
- trocar de canal e voltar não busca de novo;
- o conteúdo das mensagens não aparece no log do `sun`.

Não cobre o build de release nem a conexão por outro PC via Tailscale, que ficam para a Fase 6. As falhas de rede só foram exercitadas nos testes automáticos.

## Decisões

Questões resolvidas com o usuário em 2026-11-04:

1. **Botão e rolagem infinita:** os dois. A rolagem carrega sozinha, e o botão fica visível para quando não há o que rolar ou quando a busca falhou.
2. **Tamanho da página:** 50 mensagens, fixo.
3. **Dia nas mensagens:** sempre visível, em todas as mensagens, junto do autor e do horário.
4. **TanStack Query:** adotada no front para as chamadas e o cache ([ADR 0012](../decisions/0012-tanstack-query-no-front.md)).
