# 0014. LiveKit no host: scripts, chaves e tokens sem SDK

- **Status:** Aceita
- **Data:** 2026-11-04

## Contexto

O ADR 0002 definiu o LiveKit como SFU, com o `sun` emitindo tokens pelo crate `livekit-api`, e deixou para a Fase 3 como o host sobe o processo. Faltava decidir como instalá-lo, onde guardar as chaves e por onde ele aceita conexões.

Ao adicionar o `livekit-api`, mesmo só com a feature `access-token`, o `Cargo.lock` ganhou 18 crates (`prost`, `petgraph`, `pbjson` e outros) e uma segunda versão do `jsonwebtoken`. O token é um JWT HS256 comum, e o `sun` já assina JWTs com o `jsonwebtoken` (ADR 0008).

## Decisão

- **Instalação:** `scripts/setup-livekit.ps1` baixa uma versão fixa do `livekit-server`, conferindo um SHA-256 gravado no próprio script.
- **Chaves:** o script gera a chave e o segredo e grava `livekit/livekit.yaml` e `livekit/sun.env`, fora do git. O `sun` as recebe por variáveis de ambiente, porque o LiveKit precisa delas antes de o `sun` abrir o banco.
- **Rede:** o LiveKit anuncia só endereços da faixa do Tailscale (`100.64.0.0/10`), e o firewall só aceita essa faixa, nas portas 7880 e 7881 (TCP) e 7882 (UDP), além da 7700 do `sun`.
- **Tokens:** o `sun` assina o token do LiveKit com o `jsonwebtoken` que já usa. Este ponto substitui o uso do `livekit-api` previsto no ADR 0002; o resto dele continua valendo.

## Alternativas consideradas

- **Crate `livekit-api`:** cuida do formato do token, mas traz 18 crates para algo que cabe em um arquivo. Se o `sun` precisar da API de salas do LiveKit, a decisão pode ser revista.
- **Guardar as chaves no SQLite, como o segredo do JWT:** o LiveKit lê as chaves de um arquivo próprio, então elas continuariam em disco, em dois lugares.
- **O `sun` iniciar o LiveKit:** um único processo para o host, mas o `sun` passaria a cuidar de um processo filho, e uma queda de um afetaria o outro.
- **Aceitar qualquer origem da rede local:** mais simples para testar, mas abriria a voz a quem está na rede do host, sem passar pelo Tailscale (ADR 0003).

## Consequências

- O formato do token fica por nossa conta. Uma mudança no LiveKit pode exigir ajuste, e por isso a versão do servidor é fixa, e o token é testado contra um LiveKit real na verificação manual.
- O Tailscale precisa estar ligado também no PC do host. Para testar sem ele, o script tem a opção `-Dev`.
- A pasta `livekit/` contém um segredo e nunca deve entrar no git.
