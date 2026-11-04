# 0013. Reconexão automática mantendo a tela e o cache

- **Status:** Aceita
- **Data:** 2026-11-04

## Contexto

Até aqui, uma conexão perdida trocava a tela de chat por **Tentar de novo**. Com a presença (spec 008), isso ficou mais visível: uma oscilação de rede acima de 60 segundos tira a pessoa do grupo até ela clicar. Além disso, uma queda silenciosa de rede nem chega a fechar o WebSocket do lado do webview, e o app fica parado, parecendo conectado.

O ADR 0012 definiu que o cache dura uma conexão, porque descartar tudo era a forma mais simples de não perder o que chegou durante uma queda. Com a tela mantida, descartar apagaria a conversa que a pessoa está lendo.

## Decisão

O `moon` reconecta sozinho, sem desistir, esperando de 1 a 30 segundos entre as tentativas, e só para quando o token é recusado ou a pessoa sai. A tela de chat continua montada durante a queda.

A `Session` envia `ping` a cada 15 segundos e considera a conexão perdida depois de 45 segundos sem receber nada do servidor.

O `QueryClient` passa a durar o tempo da tela de chat, e não mais uma conexão. Ao conectar de novo, os dados são marcados como velhos (`invalidateQueries`) e buscados de novo sem sair da tela, e as mensagens guardadas do WebSocket são descartadas. Este ponto substitui o trecho do ADR 0012 sobre o tempo de vida do cache; o resto dele continua valendo.

## Alternativas consideradas

- **Recriar a tela a cada reconexão:** mantém o cache de uma conexão só, como no ADR 0012, mas perde o texto digitado, o canal selecionado e a posição da conversa a cada oscilação.
- **Buscar só o que faltou, com um cursor "depois de":** traria menos dados, mas exigiria uma rota nova e uma forma de juntar páginas com buracos entre elas. Em um grupo pequeno, buscar de novo custa pouco.
- **Desistir depois de algumas tentativas:** traria de volta o clique que esta decisão tira, e uma tentativa a cada 30 segundos não pesa em ninguém.
- **Detectar a queda só pelo fechamento do socket:** não percebe uma queda silenciosa.

## Consequências

- Reconectar busca de novo todas as páginas carregadas do canal aberto, em sequência. Quem carregou muitas páginas faz mais chamadas.
- Enquanto a queda durar, a conversa e a lista de pessoas mostram o último estado conhecido, com uma faixa avisando da queda.
- Com muitas mensagens durante a queda, a janela carregada passa a cobrir só as mais recentes, e as demais continuam acessíveis pela rolagem.
- Mesmo com o heartbeat, uma mensagem enviada no instante em que a conexão morre pode se perder, porque não há confirmação de entrega.
