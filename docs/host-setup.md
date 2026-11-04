# Guia do host: voz com o LiveKit

Como preparar o PC do host para as salas de voz. O chat funciona sem nada disto: sem o LiveKit, só as salas de voz ficam indisponíveis.

## Antes de começar

- Windows 11, com o [Tailscale](https://tailscale.com/download) instalado e **ligado**.
- O repositório do Orbit neste PC e o `sun` compilado:

  ```powershell
  cargo build -p sun --release
  ```

## 1. Instalar

Abra um PowerShell **como administrador** na raiz do repositório:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\setup-livekit.ps1
```

O script:

- baixa o `livekit-server` para a pasta `livekit/` e confere o SHA-256 do arquivo;
- gera a chave e o segredo e grava `livekit/livekit.yaml` e `livekit/sun.env`;
- cria as regras de firewall, que só aceitam a faixa do Tailscale (`100.64.0.0/10`), nas portas abaixo.

| Porta | Protocolo | Para quê |
|---|---|---|
| 7700 | TCP | `sun` (chat e tokens) |
| 7880 | TCP | LiveKit: conexão inicial |
| 7881 | TCP | LiveKit: áudio, quando o UDP não passa |
| 7882 | UDP | LiveKit: áudio |

Sem administrador, o script faz todo o resto e só avisa que o firewall ficou de fora. Para concluir, rode de novo como administrador. Rodar de novo mantém as chaves.

> **A pasta `livekit/` contém um segredo.** Ela fica fora do git. Não a envie para ninguém.

## 2. Subir o servidor

Sem precisar de administrador:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\start-host.ps1
```

Isso sobe o LiveKit e o `sun` juntos. **Ctrl+C** encerra os dois.

## 3. Conferir

- O `sun` não mostra o aviso `voz desativada` ao subir.
- No Orbit, com o endereço do Tailscale do host (`http://100.x.y.z:7700`), entre em uma sala em **Salas de voz**. Seu nome aparece no painel da sala.
- Com outra pessoa na sala, quem fala ganha um contorno.

## Problemas comuns

| O que aparece | O que fazer |
|---|---|
| "A voz não está configurada neste servidor." | O `sun` subiu sem as chaves: use o `start-host.ps1`, e não o `sun.exe` direto |
| "Não foi possível conectar à sala de voz." | Confira se o LiveKit está rodando, se o Tailscale está ligado nos dois PCs e se as regras de firewall existem |
| "O Windows não liberou o microfone." | Em **Configurações > Privacidade e segurança > Microfone**, ligue o acesso de aplicativos da área de trabalho |
| Todos entram, mas ninguém se ouve | A porta 7882 (UDP) pode estar bloqueada. Confira o firewall |

## Testar sem o Tailscale

```powershell
powershell -ExecutionPolicy Bypass -File scripts\setup-livekit.ps1 -Dev -SkipFirewall
```

Isso tira a restrição de endereços do LiveKit. Use só para testes, e rode o script sem `-Dev` para voltar ao normal.
