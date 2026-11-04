# 0012. TanStack Query para as chamadas de chat e o cache em memória do `moon`

- **Status:** Aceita
- **Data:** 2026-11-04

## Contexto

O histórico paginado (spec 007) precisa buscar páginas, guardar o que já veio por canal e controlar carregamento e erro de cada um. O `ChatLayout` já fazia isso à mão para os canais, com `useEffect` e `useState`, e repetir o padrão para as páginas de cada canal aumentaria muito esse código.

## Decisão

O `moon` usa o `@tanstack/react-query` para as chamadas REST de chat (canais e histórico). O `QueryClient` é criado junto com o `ChatLayout`, então **o cache dura uma conexão**: ao sair ou reconectar, tudo é descartado e buscado de novo. Ele fica só em memória, e nada vai para o disco.

Dentro da conexão, os dados nunca são considerados velhos (`staleTime: Infinity`), porque o WebSocket entrega o que há de novo. As mensagens recebidas por ele ficam separadas do cache, em estado do `ChatLayout`, e são juntas às páginas por `id` na hora de mostrar. Sem novas tentativas automáticas: um erro aparece na hora, com **Tentar de novo**.

## Alternativas consideradas

- **Continuar com `useEffect` e `useState`:** sem dependência nova, mas com carregamento, erro, cancelamento e cache escritos à mão para cada canal.
- **Gravar as mensagens do WebSocket no cache (`setQueryData`):** é o uso mais comum, mas uma mensagem que chega durante a primeira busca não tem onde entrar e pode se perder.
- **Guardar o cache entre conexões ou em disco:** abriria o app mais rápido, mas exigiria descobrir o que se perdeu durante a queda. Em disco, deixaria conversas no PC de cada amigo, e não só no do host.

## Consequências

- Uma dependência a mais no front, de cerca de 11 KB comprimidos.
- Login e cadastro continuam com `request()` direto: são chamadas únicas, que não têm o que guardar.
- O `request()` e o `ApiError` de `api/http.ts` continuam sendo o único caminho até o `sun`: a TanStack Query só organiza quando chamá-los.
- Reconectar sempre recarrega o histórico, o que custa uma busca por canal aberto, mas garante que nada que chegou durante a queda fique de fora.
