# moon: app desktop

App do Orbit que os amigos instalam. Tauri 2: a UI fica em Vite + React + TypeScript no webview (WebView2) e o core em Rust (`src-tauri/`).

## Estrutura

```
src/                     # front (React)
├── main.tsx
├── App.tsx              # navegação entre telas e ciclo da conexão
├── api/http.ts          # request() e ApiError: toda chamada REST ao sun passa por aqui
├── ds/                  # design system: tokens e componentes, importados por ds/index.ts (ADR 0016)
├── features/auth/       # api.ts (fetch no sun), session.ts (/ws autenticado), storage.ts (servidor e token), Login, Register, Connected
├── features/chat/       # api.ts (canais e histórico), ChatLayout (tela depois de entrar e cache), ChannelMessages (histórico + ao vivo), MessageList, Composer, MemberList (presença)
├── features/voice/      # api.ts, room.ts (único lugar que usa o livekit-client), useVoiceRoom, VoiceSection, VoicePanel
├── test/                # setup do Vitest e WebSocket simulado
└── bindings.ts          # GERADO pelo Rust: nunca edite
scripts/dev.mjs          # npm run dev:app: abre o app na primeira porta livre
src-tauri/               # core Rust
├── src/lib.rs           # comandos + specta_builder() + run()
├── src/mic_permission.rs # libera o microfone às origens do app (ADR 0015); único trecho com unsafe
├── src/main.rs          # só chama moon_lib::run()
├── tests/bindings.rs    # regenera bindings.ts (a CI checa se mudou)
├── build.rs             # tauri-build + manifest do Windows para os testes
├── capabilities/        # permissões do Tauri (o que o webview pode chamar)
└── tauri.conf.json
```

## Front ↔ core Rust

**Sempre via `bindings.ts`.** Nunca use `invoke("nome")` com string solta.

Para criar um comando:

1. Em `src-tauri/src/lib.rs` (ou num módulo próprio), escreva a função com `#[tauri::command]` **e** `#[specta::specta]`.
2. Registre em `collect_commands![...]` dentro de `specta_builder()`.
3. Rode `cargo test -p moon` (ou `npm run tauri dev`) para regenerar `bindings.ts`.
4. No front: `import { commands } from "./bindings"` → `commands.nomeEmCamelCase(...)`.
5. Adicione a linha em `docs/protocol.md` (seção Comandos Tauri).

Tipos do protocolo (`orbit_shared`) que o front usa direto, como as mensagens do WebSocket, são registrados com `.typ::<T>()` no `specta_builder()`.

## Front: padrões

- React 19 com componentes funcionais e hooks. TypeScript `strict`.
- O front fala direto com o `sun` (HTTP/WebSocket) e com o LiveKit (SDK JS) a partir do webview. O core Rust fica para o que o webview não consegue fazer bem (captura nativa, APIs do Windows).
- **Aparência:** telas novas usam os componentes de `src/ds/`, e cor, raio e espaçamento vêm dos tokens de `ds/tokens.css`. O `App.css` guarda só o layout das telas. Visual arredondado e minimalista: controles em pílula, um único tom de destaque, profundidade por camadas de superfície, sem bordas de separação, sombras ou gradientes. Só entra no módulo o que uma tela usa. `ds/conventions.test.ts` falha se aparecer cor, raio ou controle solto fora do módulo.
- Lint e formatação: Biome (`npm run lint`, `npm run format`). O hook formata automaticamente após cada edição.
- **Chamadas ao `sun`:** em `features/<domínio>/api.ts`, usando `request()` de `api/http.ts` e os tipos de `bindings.ts`. Erros viram `ApiError` (`code` do `ApiErrorBody` ou `network`); a UI mostra o `message`, e um `unauthorized` encerra a sessão (`onSessionExpired`).
- **Cache:** as chamadas de chat usam TanStack Query, com o `QueryClient` criado pelo `ChatLayout`, que continua aberto durante uma queda e invalida tudo quando a `Session` muda (ADR 0012 e 0013). O WebSocket não escreve no cache: as mensagens que chegam por ele ficam em estado do `ChatLayout` e são juntas às páginas por `id`, e a presença também fica nesse estado.
- **Voz:** o `livekit-client` só é importado em `features/voice/room.ts`, de forma preguiçosa, e os testes usam uma sala simulada. A voz não depende da `Session`: ela sobrevive a uma queda do WebSocket.
- **Sessão:** o token fica em `features/auth/storage.ts` (ADR 0010) e nunca vai para `console` nem para mensagens de erro. A conexão com o `/ws` passa por `connectSession` (ADR 0009), e quem precisa dela recebe a `Session`: `send` envia e `subscribe` recebe as mensagens do servidor. Ela envia `ping` sozinha e avisa pelo `onClose` se a conexão cair, e o `App` reconecta sem desmontar o chat.
- **Testes:** Vitest + Testing Library em jsdom, ao lado do código (`*.test.ts(x)`). `fetch` com `vi.stubGlobal`, WebSocket com `src/test/fakeWebSocket.ts`. Rode as ferramentas com `npm run`/`npm test`, não com `npx`: fora de `moon/`, o `npx` baixa do registro um pacote qualquer de mesmo nome (`tsc`, `biome`).
- Estrutura conforme crescer: `src/features/<domínio>/` (componentes + hooks + chamadas), `src/components/` para componentes genéricos.

## Mais de uma instância em dev

`npm run dev:app` (`scripts/dev.mjs`) usa a primeira porta livre entre 1420 e 1439, a mesma faixa que o `sun` libera no CORS (`sun::DEV_PORTS`). Cada porta é uma origem diferente, então cada instância tem o próprio `localStorage` e pode entrar com outra conta. Da segunda porta em diante, o build fica em `target/dev-<porta>`, porque a URL entra no executável e um `moon.exe` aberto fica travado; a primeira compilação de cada porta demora mais. Abrir duas ao mesmo tempo pode dar conflito de porta: espere a primeira subir.

## Core Rust: padrões

- `unsafe` e FFI do Windows (captura nativa, Fase 5) ficam isolados num módulo próprio, como `src-tauri/src/capture/`, com uma API segura para o resto do código.
- **Testes que linkam o Tauri ficam em `src-tauri/tests/`**, não em `#[cfg(test)]` dentro de `src/`. O `build.rs` só embute o manifest do Windows em testes de integração. Sem isso, eles falham com `STATUS_ENTRYPOINT_NOT_FOUND`.
- Para testar a voz sem microfone real, abra o app com `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--use-fake-device-for-media-stream`.
- Novas permissões para o webview vão em `capabilities/default.json`, sempre com o mínimo necessário.

## Comandos

```sh
npm run tauri dev     # app em modo dev, na porta 1420
npm run dev:app       # o mesmo, na primeira porta livre: para abrir mais de uma instância
npm run build         # tsc + vite build → dist/ (necessário antes de compilar o Rust)
npm run check         # Biome em modo CI
npm test              # testes de front (Vitest)
cargo test -p moon    # (na raiz) regenera bindings.ts
```
