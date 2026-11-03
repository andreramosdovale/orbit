# 0007. Auto-cadastro sem convite

- **Status:** Aceita
- **Data:** 2026-11-03

## Contexto

O roadmap previa um código de convite gerado pelo host (CLI do `sun`) para criar contas. Para um grupo de ~10 amigos, isso obriga o host a gerar e mandar um código para cada pessoa, e não protege muito: só alcança o `sun` quem o host já compartilhou a máquina pelo Tailscale (ADR 0003).

## Decisão

Cada amigo cria a **própria conta** pelo `moon` (`POST /auth/register`) com usuário e senha. Não há convite nem papel de admin. **O controle de acesso é o compartilhamento do Tailscale:** quem alcança o servidor pode se cadastrar.

## Alternativas consideradas

- **Código de convite pelo CLI do host:** mais controle, mas dá trabalho manual ao host para um ganho pequeno numa rede já fechada.
- **Admin cadastra os usuários:** o host teria que escolher e repassar senhas, o que é pior para todo mundo.

## Consequências

- Se o host compartilhar a máquina com alguém por engano, essa pessoa consegue criar conta. Remover o compartilhamento no Tailscale corta o acesso à rede, mas a conta continua no banco.
- Não há rate limiting de cadastro por enquanto (rede restrita).
- Se surgir a necessidade, um convite opcional pode voltar num ADR novo sem quebrar o fluxo atual.
