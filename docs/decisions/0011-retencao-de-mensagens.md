# 0011. Mensagens de texto guardadas sem limite de tempo

- **Status:** Aceita
- **Data:** 2026-11-03

## Contexto

O ADR 0004 deixou a política de retenção de mensagens para a Fase 2. As mensagens ficam no `orbit.db`, no PC do host, e o grupo tem até uns 10 amigos.

## Decisão

O `sun` guarda as mensagens de texto para sempre, sem limpeza automática.

## Alternativas consideradas

- **Apagar depois de um prazo (ex.: 90 dias):** reduziria o que fica no PC do host, mas o grupo perderia conversas antigas e o `sun` ganharia uma tarefa periódica.
- **Limitar a quantidade por canal:** tem as mesmas perdas e, além disso, um canal movimentado apagaria mensagens recentes.

## Consequências

- O `orbit.db` cresce sem parar, mas devagar: mesmo 1.000 mensagens por dia dão por volta de 100 MB por ano.
- Não há como apagar mensagens pelo app. Quem tem acesso ao arquivo pode apagar direto no banco, e o host continua responsável pelo que fica guardado no PC dele.
- Um limite pode entrar depois, em um ADR novo, sem mudar o protocolo.
