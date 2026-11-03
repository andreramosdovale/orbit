# 0009. Autenticação do WebSocket pela primeira mensagem

- **Status:** Aceita
- **Data:** 2026-11-03

## Contexto

O `/ws` precisa saber qual usuário está em cada conexão. A spec 002 previa enviar o JWT (ADR 0008) em `Authorization: Bearer`, mas a API `WebSocket` do webview não permite definir cabeçalhos. Além disso, quando o handshake é recusado, o front não consegue ler o status HTTP: um `401` parece igual a um servidor fora do ar, e o app não saberia se deve voltar para o login ou tentar reconectar.

## Decisão

- O upgrade do `/ws` não exige nada. O cliente envia `{ "type": "auth", "token": "<jwt>" }` como **primeira mensagem**, em até **10 segundos**.
- Token válido: o servidor responde `auth_ok` com o usuário.
- Qualquer outro caso: o servidor responde `auth_error` e fecha com o código **1008**, sem dizer o motivo.
- O token só é checado ao conectar. Uma conexão aberta não cai quando o token expira.
- As rotas REST futuras continuam usando `Authorization: Bearer`.

## Alternativas consideradas

- **Query string (`/ws?token=...`):** mais simples e recusa antes do upgrade, mas o token fica na URL e o front não lê a recusa.
- **Subprotocolo (`Sec-WebSocket-Protocol`):** é um uso improvisado desse cabeçalho e tem o mesmo problema de o front não ler a recusa.
- **`tauri-plugin-websocket` (conexão pelo core Rust):** permite cabeçalhos, mas adiciona uma dependência e tira o WebSocket do webview só por causa disso.

## Consequências

- O front distingue "token recusado" (`auth_error`) de "servidor fora do ar" (falha ao conectar).
- Uma conexão fica aberta sem autenticação por até 10 segundos. É aceitável numa rede restrita aos amigos (ADR 0003).
- Antes de autenticar, mensagem inválida fecha a conexão. A regra "mensagem inválida é ignorada" só vale depois de autenticar.
- Um token que vença com a conexão aberta continua valendo até a próxima reconexão.
