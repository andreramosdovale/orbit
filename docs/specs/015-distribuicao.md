# 015. Instalador e atualização automática

- **Status:** Em andamento
- **Fase:** 6 (ver `docs/roadmap.md`)

## Objetivo

Um amigo baixa um instalador do Orbit, instala sem precisar de administrador e conecta ao host seguindo um guia curto. Quando sai uma versão nova, o app avisa e se atualiza com um clique, sem perder o login. Para publicar uma versão, o host só precisa criar uma tag no git. Tudo é grátis e não há chave de assinatura para guardar ([ADR 0019](../decisions/0019-distribuicao-por-github-releases-sem-chaves.md)).

## Fora de escopo

- Assinatura de código (Authenticode) e assinatura das atualizações. O Windows mostra o aviso do SmartScreen na primeira instalação, e o guia explica como seguir.
- Instalador do `sun` e do LiveKit: o host continua usando o repositório e o [guia do host](../host-setup.md).
- Barra de progresso do download, notas da versão na UI e canais de versão (beta/estável).
- Atualização forçada: o amigo escolhe quando atualizar.
- Checar se há versão nova enquanto o app está aberto: a checagem é só ao abrir.
- Checar se a versão do `moon` é compatível com a do `sun`.

## Mudanças

### Protocolo (`shared` + `docs/protocol.md`)

Nada muda no fio. Dois comandos Tauri novos entram na seção de comandos de `docs/protocol.md`:

| Comando | Retorno | O que faz |
|---|---|---|
| `checkForUpdate()` | `Result<AvailableUpdate \| null, string>` | Procura uma versão nova. Em build de desenvolvimento, sempre retorna `null` |
| `installUpdate()` | `Result<null, string>` | Procura a versão mais recente, baixa o instalador, abre e fecha o app. O instalador reabre a versão nova |

`AvailableUpdate` tem só o campo `version`. O tipo fica no `moon`, porque o `sun` não o usa.

### Dados (SQLite / migrations)

Nenhuma.

### `sun`

Nada. O `sun` já libera a origem do build (`http://tauri.localhost`) no CORS.

### `moon` (UI e core Rust)

**Configuração (`tauri.conf.json`):**

- `bundle.targets` passa de `all` para `nsis`.
- NSIS em `installMode: currentUser` (sem administrador) e em português (`PortugueseBR`).
- O campo `version` sai do arquivo. O Tauri passa a ler a versão do `Cargo.toml`, a única fonte.

**Core Rust:**

- Novas dependências diretas, que já estão na árvore do `moon`: `reqwest` e `semver`.
- `src-tauri/src/update.rs` com os dois comandos, registrados no `specta_builder()`.
  - A versão mais recente vem de `https://api.github.com/repos/andreramosdovale/orbit/releases/latest`. Sem nenhum release publicado (404), não há atualização.
  - A URL do instalador é montada a partir da versão, sempre neste repositório: `https://github.com/andreramosdovale/orbit/releases/download/v<versão>/Orbit_<versão>_x64-setup.exe`.
  - O instalador vai para a pasta temporária e abre com `/P /UPDATE /R`: mostra só a barra de progresso, troca a versão e reabre o app.
- O webview não ganha nenhuma permissão nova em `capabilities/default.json`.

**Front:**

- `features/update/`: um aviso discreto, feito com o `ds/` e visível em todas as telas, inclusive no login.
  - Texto: "Versão X disponível", com o botão **Atualizar**.
  - Ao clicar, o aviso mostra "Atualizando…", e o app fecha sozinho para instalar. Quem estiver em uma sala de voz ou transmitindo cai dela.
  - Se a atualização falhar, o aviso mostra uma mensagem de erro e o botão volta.
  - Se a checagem falhar (sem internet, GitHub fora do ar), nada aparece: o erro só vai para o log.

### Publicação (`.github/workflows/release.yml`)

- Roda ao receber uma tag `v*`, em `windows-latest`.
- Falha logo no início se a tag for diferente da versão do `Cargo.toml`.
- Gera o instalador com `npm run tauri build` e publica em um GitHub Release com `gh release create`, usando o `GITHUB_TOKEN` que o próprio GitHub fornece. Não há secret para cadastrar.

### Documentação

- `docs/guia-dos-amigos.md`: instalar o Tailscale, aceitar o compartilhamento da máquina do host (ADR 0003), instalar o Orbit, passar pelo SmartScreen, informar o endereço do servidor e resolver problemas comuns.
- `docs/release.md`: como publicar uma versão.
- ADR 0019: distribuição por GitHub Releases, sem chaves.
- `moon/CLAUDE.md`, `docs/architecture.md`, `docs/README.md` e `docs/roadmap.md`.

## Critérios de aceite

Automáticos:

- [ ] O teste `moon/src-tauri/tests/distribution.rs` passa. Ele fixa a configuração do instalador (só `nsis`, `currentUser`, sem o campo `version`), a comparação de versões e a URL do instalador.
- [ ] Os testes de `features/update/` passam, cobrindo:
  - sem atualização, nada aparece;
  - com atualização, a versão e o botão aparecem;
  - clicar chama `installUpdate` e mostra "Atualizando…";
  - falha na atualização mostra o erro e devolve o botão;
  - falha na checagem não mostra nada.
- [ ] `git status --short moon/src-tauri/capabilities/default.json` não mostra mudança, e o `bindings.ts` contém os dois comandos.
- [ ] `/verify` verde.
- [ ] `npm run tauri build` gera `target/release/bundle/nsis/Orbit_<versão>_x64-setup.exe`, sem nenhuma chave no ambiente.

Com um release de verdade (feitos pelo host, porque dependem do GitHub e de um segundo PC):

- [ ] Uma tag com versão diferente da do `Cargo.toml` faz o workflow falhar sem publicar nada.
- [ ] Uma tag `v0.1.1`, com o `Cargo.toml` em 0.1.1, publica um Release com o instalador.
- [ ] Em um PC sem o Orbit e sem administrador, o instalador da versão anterior instala e o app abre e conecta ao `sun`.
- [ ] Esse app mostra "Versão 0.1.1 disponível". Depois de **Atualizar**, ele reabre em 0.1.1 ainda logado e com o servidor configurado.
- [ ] Um amigo que nunca viu o projeto conecta seguindo só o guia.

## Plano de implementação

Cada passo termina com `/verify` verde.

1. **Empacotamento:** alvo NSIS e versão única, com o teste de configuração. Conferir o instalador em `npm run tauri build`.
2. **Core da atualização:** `update.rs` e `bindings.ts`, além de `docs/protocol.md`.
3. **Front:** `features/update/` com os testes, ligado ao `App`.
4. **Publicação:** `release.yml`, `docs/release.md` e o ADR 0019.
5. **Guia dos amigos:** `docs/guia-dos-amigos.md` e os demais docs.
6. **Prova real:** publicar `v0.1.1` e passar pelos critérios feitos com o host.

## Questões em aberto

- **Release com defeito:** o Release sai publicado direto, e todo mundo vê o aviso ao abrir o app. Para desfazer, basta publicar uma versão maior com a correção.
