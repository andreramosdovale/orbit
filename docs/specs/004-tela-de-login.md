# 004. Tela de login no `moon` e testes de front

- **Status:** Concluída
- **Fase:** 1 (ver `docs/roadmap.md`)

## Objetivo

Um amigo abre o `moon`, informa o endereço do servidor, entra com usuário e senha e fica conectado ao `/ws` autenticado. Nas próximas vezes, o app reabre a sessão sozinho até o token expirar. Quando o token é recusado, o app volta para o login; quando o servidor está fora do ar, oferece tentar de novo sem perder a sessão.

Esta spec fecha dois itens do roadmap: a tela de login e os testes de front com Vitest (ver Questões em aberto (originais), item 3).

## Fora de escopo

- **Chat, canais e presença**: Fase 2. Depois de entrar, o app mostra só uma tela simples com o usuário conectado.
- **Reconexão automática**: quando a conexão cai, o usuário clica em "Tentar de novo". Reconectar sozinho entra quando o chat precisar.
- **Mudanças no `sun` e no protocolo**: os tipos e as rotas necessários já existem.
- **Revogar o token ao sair**: não existe revogação (ADR 0008). "Sair" só apaga o token deste PC.
- **Vários servidores ou contas salvos**.
- **Identidade visual**: continua só o estilo básico.

## Mudanças

### Protocolo (`shared` + `docs/protocol.md`)

Nenhuma. O `bindings.ts` não muda.

### Dados (SQLite / migrations)

Nenhuma no servidor. No `moon`, o `localStorage` guarda:

| Chave | Conteúdo | Apagada quando |
|---|---|---|
| `orbit.server` | Endereço do servidor (já existe) | Nunca: "Sair" mantém o endereço |
| `orbit.token` | JWT da sessão (ver Questões em aberto (originais), item 1) | Ao sair ou quando o servidor responde `auth_error` |

### `sun`

Nenhuma.

### `moon` (UI e core Rust)

**Telas e transições** (`App.tsx`):

| Situação | O que aparece |
|---|---|
| Sem token salvo | Login |
| Token salvo, conectando | "Conectando..." |
| `auth_ok` | Tela conectada: "Conectado como `fulano`" e botão **Sair** |
| `auth_error` | Login, com "Sua sessão expirou. Entre novamente." e o token apagado |
| Servidor inacessível, ou a conexão caiu | "Não foi possível conectar ao servidor", com **Tentar de novo** e **Sair**. O token é mantido |

**Login** (`features/auth/Login.tsx`):

- Campos **Servidor** (salvo em `orbit.server`, como no cadastro), **Usuário** e **Senha**.
- Botão **Entrar**, desabilitado enquanto envia.
- Mostra o `message` do erro do servidor, ou "Não foi possível conectar ao servidor." em falha de rede.
- A senha sai da memória do formulário depois do envio bem-sucedido.
- Link para **Criar conta**.

**Cadastro** (`features/auth/Register.tsx`):

- Ganha um link para voltar ao login.
- Depois de criar a conta, volta ao login com o usuário preenchido e a mensagem "Conta `fulano` criada. Entre com sua senha." Não faz login automático (ver Questões em aberto (originais), item 4).
- A leitura e a gravação do servidor saem de `Register.tsx` para `storage.ts`.

**Código novo em `features/auth/`:**

- `api.ts`: `login(server, req): Promise<LoginResponse>`, compartilhando com `register` o código de `POST` e de tratamento de erro.
- `storage.ts`: leitura e gravação de `orbit.server` e `orbit.token`, tolerando `localStorage` indisponível.
- `session.ts`:
  - `wsUrl(server)`: `http://` vira `ws://`, `https://` vira `wss://`, e o caminho é `/ws`;
  - `connectSession(server, token, onClose)`: abre o WebSocket e envia `auth` assim que abrir. Resolve com `{ user, close }` no `auth_ok`. Rejeita como `rejected` no `auth_error` e como `unavailable` em erro, fechamento antes da resposta ou **15 segundos** sem resposta. O prazo é maior que os 10 segundos do `sun`, para uma recusa dele chegar primeiro.
- Tela conectada em `features/auth/Connected.tsx`.

**Regras:**

- O token nunca vai para `console` nem para mensagens de erro.
- Fechar ou desmontar a tela fecha o WebSocket (o `StrictMode` monta os efeitos duas vezes em dev).

**Testes de front:**

- Vitest + Testing Library em jsdom, com `fetch` e `WebSocket` simulados. Testes em `src/**/*.test.ts(x)`.
- Script `npm test` (`vitest run`), adicionado à CI e ao `/verify`.
- Dependências novas de desenvolvimento: `vitest`, `jsdom`, `@testing-library/react` e `@testing-library/dom`. Nenhuma dependência nova de produção.

## Critérios de aceite

- [x] `moon/src/features/auth/api.test.ts`:
  - `login` envia `POST /auth/login` e resolve com `LoginResponse`;
  - um `401` rejeita com `ApiError` de código `invalid_credentials` e o `message` do servidor;
  - falha de rede rejeita com `ApiError` de código `network`.
- [x] `moon/src/features/auth/session.test.ts`:
  - `wsUrl` converte `http`, `https` e barras finais;
  - `auth` é a primeira mensagem enviada e contém o token;
  - `auth_ok` resolve com o usuário;
  - `auth_error` rejeita como `rejected`;
  - erro ou fechamento antes da resposta rejeita como `unavailable`;
  - 15 segundos sem resposta rejeita como `unavailable` e fecha o socket;
  - fechamento depois do `auth_ok` chama `onClose`;
  - `close()` chamado pelo app fecha o socket sem chamar `onClose`.
- [x] `moon/src/features/auth/Login.test.tsx`:
  - o botão fica desabilitado enquanto envia;
  - o erro do servidor aparece na tela;
  - o servidor é salvo e a senha não é gravada no `localStorage`.
- [x] `moon/src/App.test.tsx`:
  - sem token, mostra o login;
  - login com sucesso grava o token e mostra "Conectado como fulano";
  - com token salvo e `auth_ok`, abre direto a tela conectada;
  - com `auth_error`, apaga o token e volta ao login com a mensagem de sessão expirada;
  - com servidor inacessível, mantém o token e mostra **Tentar de novo**, que conecta de novo ao ser clicado;
  - a conexão caindo depois de conectar mostra **Tentar de novo**;
  - **Sair** apaga o token, mantém o servidor e fecha o socket;
  - criar a conta volta ao login com o usuário preenchido.
- [x] `cd moon && npm test` passa, e `.github/workflows/` e `.claude/skills/verify/SKILL.md` rodam esse comando.
- [x] `git diff --exit-code origin/main -- moon/src/bindings.ts` não mostra mudança.
- [x] **Verificação manual** com o `sun` real e `npm run tauri dev`: criar conta, entrar, fechar e reabrir o app (a sessão volta), parar o `sun` e clicar em **Tentar de novo** (mostra o erro; com o `sun` de volta, conecta), e **Sair**.
- [x] `/verify` verde.

## Plano de implementação

Passos pequenos, cada um terminando com `/verify` verde.

1. **Vitest:** dependências, configuração, `npm test`, CI, `/verify` e `moon/CLAUDE.md`, com os testes de `normalizeServerUrl` como primeiro teste real.
2. **ADR 0010:** onde o token fica guardado no `moon`.
3. **API e armazenamento:** `login` em `api.ts`, `storage.ts`, e `Register.tsx` passa a usar `storage.ts`, com testes.
4. **Sessão:** `session.ts` com testes.
5. **Telas:** `Login.tsx`, `Connected.tsx`, navegação em `App.tsx`, ajustes em `Register.tsx` e no CSS, com testes.
6. **Verificação manual** com o `sun` real.
7. **docs:** `docs/architecture.md`, `moon/CLAUDE.md`, roadmap (marcar os dois itens), `docs/specs/README.md`, `docs/decisions/README.md` e esta spec como `Concluída`.

## Verificação manual

Feita em 2026-11-03 na janela real do app (WebView2), dirigida por script via depuração remota, contra um `sun` real em `127.0.0.1:7711` com banco temporário. Em vez de `npm run tauri dev`, rodou o Vite e o `moon.exe` de debug, que é o que esse comando sobe. Passaram: criar conta, usuário repetido, senha errada, entrar, fechar a janela e reabrir, parar o `sun`, **Tentar de novo** com ele parado e de volta, token recusado e **Sair**. Token e senha não apareceram no log do `sun`.

Não cobre o build de release (origem `tauri.localhost`) nem a conexão por outro PC via Tailscale, que ficam para a Fase 6.

## Decisões

Questões resolvidas com o usuário em 2026-11-03: as quatro propostas abaixo foram aceitas, e a do token foi registrada no [ADR 0010](../decisions/0010-token-no-localstorage.md).

## Questões em aberto (originais)

1. **Onde guardar o token:** proposto **`localStorage`**, igual ao endereço do servidor e sem dependência nova. O risco é que qualquer script rodando no webview, ou quem tem acesso à conta do Windows, consiga ler o token. O app só carrega o próprio código e a rede é fechada (ADR 0003), então o risco é baixo.
   - **Credential Manager do Windows (via core Rust):** protege melhor o token em disco, mas exige uma dependência nativa e comandos Tauri, e não impede um script no webview de pedir o token pelo mesmo comando.
   - **Só em memória:** o mais seguro, mas obriga a logar toda vez que abre o app e desperdiça a validade de 30 dias.
2. **Conectar ao `/ws` já nesta spec:** proposto **sim, de forma mínima**. Não existe outra forma de saber se um token salvo ainda vale, e isso confirma o fluxo completo com o `sun` real antes do chat.
3. **Juntar o Vitest a esta spec:** proposto **sim**. A tela é o primeiro código de front que vale testar, e os critérios de aceite dela dependem disso. As quatro dependências novas de desenvolvimento precisam da sua aprovação.
4. **Depois de criar a conta:** proposto **voltar ao login com o usuário preenchido**, o que mantém um único caminho de entrada e faz o usuário confirmar a senha. Entrar direto economizaria um passo, mas misturaria os dois fluxos.
