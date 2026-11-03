# shared: protocolo

Tipos trafegados entre `sun` e `moon`. **Mudar algo aqui é mudar o protocolo:** siga o checklist "Mudou o protocolo?" do `CLAUDE.md` da raiz.

## Regras

- Todo tipo público deriva `Debug, Clone, PartialEq, Serialize, Deserialize` (+ `Eq` quando der) e `#[cfg_attr(feature = "specta", derive(specta::Type))]`, para ser exportado ao TypeScript.
- Enums de mensagem: `#[serde(tag = "type", rename_all = "snake_case")]`.
- Documente cada tipo e variante com `///`: o comentário vai parar no `bindings.ts` como JSDoc.
- **Todo formato novo ganha um teste** que fixa o JSON exato (veja `client_message_wire_format`).
- Nada de lógica nem I/O aqui: só tipos, e no máximo construtores e validações puras. Sem tokio, sem axum, sem tauri.
- Dependências permitidas: `serde` e `specta` (opcional). Pergunte antes de adicionar outra.

## Testes

```sh
cargo test -p orbit-shared
```
