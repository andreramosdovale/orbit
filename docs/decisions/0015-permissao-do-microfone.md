# 0015. O core do `moon` permite o microfone, sem diálogo

- **Status:** Aceita
- **Data:** 2026-11-04

## Contexto

No build, o WebView2 abre um diálogo próprio quando o app pede o microfone: "http://tauri.localhost quer / Use seus microfones / Bloquear / Permitir". A resposta fica guardada no perfil. Quem clica em **Bloquear** não tem como voltar atrás pelo app: o diálogo não reaparece, e o WebView2 não tem tela de configurações. Um clique errado deixaria um amigo sem voz até alguém apagar arquivos do perfil dele (prova técnica na spec 010).

## Decisão

O core Rust do `moon` responde ao evento `PermissionRequested` do WebView2, permitindo **só o microfone** e **só para as origens do próprio app**. O código fica isolado em `src-tauri/src/mic_permission.rs`, e o `webview2-com` passa a ser dependência direta, na mesma versão usada pelo Tauri.

## Alternativas consideradas

- **Manter o diálogo:** nenhum código nativo, mas a recuperação de um clique errado seria apagar o arquivo de preferências do perfil.
- **Um comando Rust para desfazer o bloqueio:** também exige FFI e, além disso, uma tela para isso.

## Consequências

- Ninguém vê o diálogo, e entrar em uma sala passa a ser a autorização. Quem nunca clicar em uma sala nunca abre o microfone: o app só o pede ao entrar.
- O bloqueio nas configurações de privacidade do Windows continua valendo, e o app mostra um erro nesse caso.
- Um código injetado na página poderia usar o microfone sem aviso. O app só carrega o próprio front, e o React escapa o conteúdo das mensagens. Vale lembrar disso ao adicionar conteúdo externo ou mudar a CSP.
- Ao atualizar o Tauri, a versão do `webview2-com` precisa acompanhar a dele. Se divergirem, a compilação falha.
