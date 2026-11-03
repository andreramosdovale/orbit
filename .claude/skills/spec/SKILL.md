---
name: spec
description: Cria uma spec de funcionalidade em docs/specs/ a partir do template, antes de qualquer código. Use quando o usuário pedir uma funcionalidade nova ou quando for começar uma fase do roadmap.
argument-hint: <nome-da-funcionalidade>
---

# /spec

1. Leia `docs/roadmap.md`, `docs/architecture.md` e `docs/protocol.md` para entender o contexto.
2. Crie `docs/specs/NNN-<slug>.md` copiando `docs/specs/_template.md`, onde `NNN` é o próximo número livre.
3. Preencha com o usuário: objetivo, fora de escopo, mudanças de protocolo, de dados (SQLite) e de UI.
4. **Critérios de aceite** precisam ser verificáveis por um teste ou comando. "Funciona bem" não serve; "o teste `sun/tests/chat.rs::message_broadcast` passa" serve.
5. Liste o plano de implementação em passos pequenos, cada um terminando com `/verify` verde.
6. Mostre a spec ao usuário e espere aprovação antes de implementar.
7. Ao terminar a implementação, mude o status da spec para `Concluída` e marque o item em `docs/roadmap.md`.
