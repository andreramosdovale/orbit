# Como entrar no Orbit

Guia para quem vai usar o Orbit em um PC com Windows. Leva uns 10 minutos, e você vai precisar de duas coisas que o host manda: um **link do Tailscale** e o **endereço do servidor**, que parece com `http://100.x.y.z:7700`.

## 1. Instalar o Tailscale

O Tailscale cria a conexão privada entre o seu PC e o do host. É grátis.

1. Baixe em [tailscale.com/download](https://tailscale.com/download) e instale.
2. Abra o Tailscale e entre com uma conta do Google, da Microsoft ou do GitHub.
3. Abra o link que o host mandou e aceite o compartilhamento.

## 2. Instalar o Orbit

1. Abra a [página da última versão](https://github.com/andreramosdovale/orbit/releases/latest) e baixe o arquivo `Orbit_..._x64-setup.exe`.
2. Abra o arquivo. Não é preciso ser administrador.
3. O Windows pode mostrar **"O Windows protegeu o computador"**. Isso acontece porque o Orbit não tem um certificado pago. Clique em **Mais informações** e depois em **Executar assim mesmo**.
4. Siga o instalador até o fim.

## 3. Entrar

1. Abra o Orbit.
2. Em **Servidor**, coloque o endereço que o host mandou.
3. Crie sua conta e entre com ela.

Pronto: o Orbit lembra de você nas próximas vezes.

## Atualizações

Quando sair uma versão nova, o Orbit mostra um aviso ao abrir. Clique em **Atualizar**: ele fecha, instala e abre de novo sozinho. Se você estiver em uma sala de voz, vai sair dela, então atualize quando puder.

## Problemas comuns

| O que aparece | O que fazer |
|---|---|
| Não conecta ao servidor | Confira se o Tailscale está ligado, se o endereço está certo e se o PC do host está ligado com o servidor aberto |
| "A voz não está configurada neste servidor." | Avise o host |
| "O Windows não liberou o microfone." | Em **Configurações > Privacidade e segurança > Microfone**, ligue o acesso de aplicativos da área de trabalho |
| O aviso de atualização mostra um erro | Clique em **Atualizar** de novo. Se continuar, baixe a última versão na página do passo 2 e instale por cima |

## Para o host: como compartilhar o PC

1. No [painel do Tailscale](https://login.tailscale.com/admin/machines), abra o menu da sua máquina e escolha **Share**.
2. Gere o link e mande para o amigo.
3. Mande também o endereço `http://<IP do Tailscale do seu PC>:7700`. O IP começa com `100.` e aparece no mesmo painel.

O servidor precisa estar rodando: veja o [guia do host](host-setup.md).
