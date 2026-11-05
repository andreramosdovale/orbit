# Documentação do Orbit

| Arquivo | Para que serve |
|---|---|
| [architecture.md](architecture.md) | Visão geral: componentes, fluxo de dados, onde cada coisa roda |
| [protocol.md](protocol.md) | Contrato entre `sun` e `moon`: REST, WebSocket, comandos Tauri |
| [roadmap.md](roadmap.md) | Fases do projeto e o que está pronto |
| [decisions/](decisions/README.md) | ADRs: decisões de arquitetura e por que foram tomadas |
| [specs/](specs/README.md) | Uma spec por funcionalidade, escrita antes do código |

## Regras

- **A doc acompanha o código no mesmo commit.** Mudou o protocolo? Atualize `protocol.md`. Mudou a arquitetura? Atualize `architecture.md` e, se for uma decisão, escreva um ADR.
- **Funcionalidade nova começa por uma spec** (`/spec`), com critérios de aceite verificáveis.
- **Decisões não se rediscutem sem um ADR novo** (`/adr`) que substitua o anterior.
- [`host-setup.md`](host-setup.md): como o host instala e sobe o LiveKit.
- [`guia-dos-amigos.md`](guia-dos-amigos.md): como um amigo instala o Tailscale e o Orbit e conecta.
- [`release.md`](release.md): como publicar uma versão do app.
