# 009. Reconexão automática

- **Status:** Concluída
- **Fase:** 2 (ver `docs/roadmap.md`)

## Objetivo

Quando a conexão com o servidor cai, o app volta sozinho assim que for possível, sem o usuário clicar em nada e sem perder a tela: o canal selecionado, o texto sendo digitado e a posição da conversa continuam lá, e o que chegou durante a queda aparece. Isso resolve a limitação deixada pela spec 008, em que uma oscilação acima de 60 segundos tirava a pessoa do grupo até ela clicar em **Tentar de novo**.

## Fora de escopo

- **Fila de envio:** enquanto a conexão estiver caída, o campo continua editável, mas Enter não envia nada. Não há mensagens guardadas para enviar depois.
- **Confirmação de entrega:** uma mensagem enviada no instante em que a conexão morre pode se perder, e o texto já saiu do campo (a spec 006 não tem exibição otimista).
- **Eventos do sistema**, como o aviso de que a rede voltou (`online`): a espera máxima de 30 segundos já cobre esse caso, e o botão reconecta na hora.
- **Mudanças no `sun` e no protocolo:** o `ping` e o `pong` já existem.

## Mudanças

### Protocolo (`shared` + `docs/protocol.md`)

Sem mudança no formato. O `docs/protocol.md` passa a registrar que o `moon` envia `ping` periodicamente e o que acontece quando não há resposta.

### Dados (SQLite / migrations)

Sem migration.

### `sun`

Sem mudança. Ao reconectar, a pessoa recebe um novo `presence_snapshot`. Se ela voltar antes de o servidor notar que a conexão antiga morreu, não há anúncio de saída, porque ainda existe uma conexão contada.

### `moon` (UI e core Rust)

**Quando reconectar:**

- a conexão cai depois de autenticada: tenta de novo **na hora**;
- uma tentativa falha: espera **1, 2, 4, 8 e 16 segundos e, depois, 30 segundos** entre as tentativas, sem desistir;
- ao abrir o app com o servidor fora do ar: o mesmo processo, sem clique;
- o token é recusado (`auth_error`): para de tentar e volta ao login, como hoje;
- **Sair** cancela qualquer tentativa ou espera.

**Conexão morta (`features/auth/session.ts`):** uma queda de rede silenciosa não fecha o WebSocket do lado do webview, então o app ficaria parado, parecendo conectado.

- a `Session` envia `ping` a cada **15 segundos**;
- sem receber **nada** do servidor por **45 segundos**, ela considera a conexão perdida, avisa pelo `onClose` e fecha o socket, sem esperar o navegador concluir o fechamento.

**Tela (`App.tsx` e `features/chat/ChatLayout.tsx`):**

- `App.tsx` passa a ter dois estados de conexão, `online` e `offline`. O `offline` guarda se há uma tentativa em andamento, quantas já falharam e a tela de chat aberta, se houver;
- com chat aberto, o `ChatLayout` **continua montado**. Uma faixa no topo da conversa mostra "Conexão perdida. Reconectando..." e **Tentar de novo**, que reconecta na hora e fica desativado durante uma tentativa;
- sem chat aberto, continua a tela de "Não foi possível conectar ao servidor", agora com o aviso de que o app está tentando sozinho;
- o `Composer` ganha `offline`: o campo continua editável, e Enter não envia nem apaga o texto.

**Dados ao reconectar (`ChatLayout.tsx`), quando a `Session` muda:**

- a lateral troca "Conectado como" por "Sem conexão, entrou como" durante a queda;
- `queryClient.invalidateQueries()`: os canais e as páginas do canal aberto são buscados de novo, sem apagar o que está na tela. Os outros canais são buscados ao serem abertos;
- as mensagens guardadas do WebSocket são descartadas, porque as páginas novas já as trazem. Mantê-las deixaria um buraco na conversa quando a queda passasse de 50 mensagens;
- a assinatura passa para a `Session` nova, que entrega o `presence_snapshot`.

Essa troca de `Session` precisa acontecer **antes** da nova assinatura, para não apagar o que chegou por ela.

Isso muda o ciclo de vida do cache definido no ADR 0012, e por isso há um ADR novo.

## Critérios de aceite

- [x] `moon/src/features/auth/session.test.ts`:
  - envia `ping` a cada 15 segundos;
  - qualquer mensagem do servidor adia o prazo;
  - 45 segundos sem receber nada chamam `onClose` uma única vez e fecham o socket;
  - `close()` para os pings;
  - `retryDelayMs` dá 1, 2, 4, 8, 16, 30, 30 segundos.
- [x] `moon/src/App.test.tsx`:
  - a conexão caindo tenta de novo na hora, sem clique;
  - conectar de novo mantém o chat e o texto do campo;
  - com o servidor inacessível, as tentativas seguem as esperas, sem clique;
  - **Tentar de novo** reconecta sem esperar o prazo;
  - `auth_error` ao reconectar volta ao login e para as tentativas;
  - **Sair** durante a espera cancela a próxima tentativa.
- [x] `moon/src/features/chat/ChatLayout.test.tsx`:
  - com a conexão perdida, a faixa aparece, e Enter não chama `send`;
  - a `Session` nova busca de novo os canais e o histórico do canal aberto;
  - o canal selecionado continua o mesmo;
  - o que chega pela `Session` nova aparece, e uma mensagem já trazida pela página não se repete.
- [x] `moon/src/features/chat/Composer.test.tsx`: com `offline`, Enter não envia nem apaga o texto.
- [x] **Verificação manual** com um `sun` real e o app: derrubar e religar o `sun` reconecta sozinho, mantendo canal e texto; o que foi enviado durante a queda aparece.
- [x] **Verificação manual** de queda silenciosa, com um proxy TCP que para de repassar os dados sem fechar a conexão: o app mostra a faixa em até 60 segundos e volta quando o proxy volta.
- [x] `/verify` verde.

## Plano de implementação

Passos pequenos, cada um terminando com `/verify` verde.

1. **ADR 0013:** reconexão automática e cache entre conexões.
2. **moon, sessão:** heartbeat e `retryDelayMs` em `session.ts`, com testes.
3. **moon, tela:** estados de conexão em `App.tsx`, faixa e troca de `Session` no `ChatLayout`, `offline` no `Composer`, com testes.
4. **Verificação manual.**
5. **docs:** `docs/protocol.md`, `docs/architecture.md`, `moon/CLAUDE.md`, roadmap, `docs/specs/README.md` e esta spec como `Concluída`.

## Verificação manual

Feita em 2026-11-04 na janela real do app (WebView2), dirigida por script via depuração remota, contra um `sun` real com banco temporário.

**Queda silenciosa**, com um proxy TCP entre o app e o `sun` que para de repassar os dados sem fechar a conexão:

- o app mostrou a faixa 60 segundos depois, e o `sun` também encerrou a conexão e anunciou a saída;
- a conversa, o canal selecionado e o texto do campo continuaram na tela, e Enter não enviou nem apagou o texto;
- com a rede de volta, reconectou sozinho em cerca de 1 segundo, a mensagem enviada durante a queda apareceu, nada se repetiu, e o envio e a lista de pessoas voltaram a funcionar.

**`sun` derrubado e religado com o mesmo banco:**

- a faixa apareceu na hora e continuou enquanto o `sun` estava fora do ar;
- ao religar, o app voltou sozinho em cerca de 1 segundo, com o texto do campo, e voltou a receber mensagens.

Não cobre o build de release nem a conexão por outro PC via Tailscale, que ficam para a Fase 6.

## Decisões

Pedido do usuário em 2026-11-04, depois da spec 008, para resolver a limitação. As escolhas abaixo foram feitas na spec, sem pergunta específica ao usuário, e podem ser revistas:

1. **Tentar para sempre:** sem limite de tentativas. O custo é uma conexão a cada 30 segundos, e desistir traria de volta o clique que esta spec tira.
2. **Manter a tela:** em vez de voltar a uma tela de "reconectando", para não perder o texto nem a conversa em uma oscilação curta.
3. **Heartbeat no cliente:** 15 e 45 segundos. É mais curto que os 60 segundos do `sun`, para o app perceber a queda antes ou junto com o servidor.
