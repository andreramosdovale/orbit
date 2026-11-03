---
name: adr
description: Registra uma decisão de arquitetura em docs/decisions/. Use quando uma escolha técnica relevante for feita ou revertida (biblioteca, protocolo, formato de dados, estratégia).
argument-hint: <título da decisão>
---

# /adr

1. Veja os ADRs existentes em `docs/decisions/`. Se a nova decisão substitui uma antiga, mude o status da antiga para `Substituída por NNNN`. Nunca apague um ADR.
2. Crie `docs/decisions/NNNN-<slug>.md` a partir de `docs/decisions/0000-template.md`, com o próximo número livre.
3. Seja curto: contexto, decisão, alternativas consideradas (e por que não), consequências.
4. Adicione a linha no índice de `docs/decisions/README.md`.
5. Se a decisão muda algo descrito em `CLAUDE.md` ou `docs/architecture.md`, atualize esses arquivos também.
