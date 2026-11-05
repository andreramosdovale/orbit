# 0019. Distribuição por GitHub Releases, sem chaves de assinatura

- **Status:** Aceita
- **Data:** 2026-11-05

## Contexto

Os amigos precisam instalar o Orbit e receber as versões novas. O projeto é de um grupo pequeno, sem fins comerciais, e deve continuar grátis e sem nada para o host guardar. O `tauri-plugin-updater`, previsto no roadmap, exige um par de chaves próprio: se a chave privada for perdida, os apps instalados deixam de aceitar atualizações. Não há como desligar essa exigência.

## Decisão

- Publicar o instalador NSIS em GitHub Releases, por um workflow disparado com uma tag `v<versão>`.
- Atualizar sem plugin: o core Rust consulta o último release do repositório, baixa o instalador por HTTPS e o abre em modo passivo.
- Não assinar o instalador nem as atualizações.

## Alternativas consideradas

- **`tauri-plugin-updater`:** é mais seguro, porque um instalador trocado seria recusado, mas exige guardar a chave privada e cadastrá-la como secret.
- **Certificado de assinatura de código:** tiraria o aviso do SmartScreen, mas é pago.
- **Só avisar e abrir a página de download:** exigiria menos código, mas cada atualização seria um download manual, com o aviso do SmartScreen de novo.

## Consequências

- Não há custo nem chave para guardar: publicar uma versão é criar uma tag.
- **O app confia no GitHub e no HTTPS.** Quem conseguir publicar um release neste repositório consegue rodar código no PC de cada amigo que clicar em **Atualizar**. A conta do GitHub do host deve ter autenticação em dois fatores.
- O Windows mostra o aviso do SmartScreen na primeira instalação, e o [guia dos amigos](../guia-dos-amigos.md) explica como seguir.
- A API do GitHub limita os pedidos sem login por IP. Passar do limite só faz a checagem falhar em silêncio até a próxima abertura do app.
- Se o repositório mudar de dono ou de nome, os apps instalados podem deixar de encontrar atualizações. Nesse caso, cada amigo precisa reinstalar à mão.
