# 0005. Tipos TypeScript gerados do Rust com specta

- **Status:** Aceita
- **Data:** 2026-11-03

## Contexto

O front (TS) consome o protocolo definido em Rust (`shared`) e os comandos do core Rust do `moon`. Manter tipos duplicados à mão causa divergência silenciosa, especialmente quando a IA edita só um dos lados.

## Decisão

Usar **`tauri-specta` + `specta` + `specta-typescript`** para gerar `moon/src/bindings.ts`:

- Comandos Tauri anotados com `#[specta::specta]` e registrados em `specta_builder()` (`moon/src-tauri/src/lib.rs`).
- Tipos do `shared` derivam `specta::Type` atrás da feature `specta` e são registrados com `.typ::<T>()`.
- O arquivo é regenerado em `tauri dev` e pelo teste `moon/src-tauri/tests/bindings.rs`. A CI falha se ele mudar.

As versões são fixadas com `=` no `Cargo.toml` da raiz, porque o specta v2 ainda está em RC e pode quebrar entre versões.

## Alternativas consideradas

- **ts-rs:** gera tipos, mas não os wrappers de comandos Tauri.
- **Tipos à mão:** divergem.

## Consequências

- `bindings.ts` é commitado e nunca editado à mão. O Biome o ignora.
- Atualizar o specta exige mudar as três versões juntas e rodar `/verify`.
