# 0010. Token de sessão guardado no `localStorage`

- **Status:** Aceita
- **Data:** 2026-11-03

## Contexto

O JWT vale 30 dias (ADR 0008) justamente para o amigo não precisar logar toda hora, então o `moon` precisa guardá-lo entre uma abertura e outra. O front já guarda o endereço do servidor no `localStorage`, e quem fala com o `sun` é o próprio webview.

## Decisão

O `moon` guarda o token no `localStorage` do webview, na chave `orbit.token`. Ele é apagado quando o usuário clica em "Sair" ou quando o servidor responde `auth_error`.

## Alternativas consideradas

- **Credential Manager do Windows (via core Rust):** protege melhor o token em disco, mas exige uma dependência nativa e comandos Tauri. Também não impede um script no webview de pedir o token pelo mesmo comando.
- **Só em memória:** é o mais seguro, mas obriga a logar toda vez que o app abre e desperdiça a validade de 30 dias.

## Consequências

- Um script rodando no webview, ou quem tem acesso à conta do Windows, consegue ler o token. O risco é baixo: o app só carrega o próprio código e a rede é fechada (ADR 0003). Se o app passar a exibir conteúdo de terceiros, esta decisão precisa ser revista.
- Como não há revogação (ADR 0008), um token copiado continua válido até expirar, mesmo depois de "Sair".
- Não há dependência nova nem código no core Rust.
