# 0003. Rede privada via Tailscale

- **Status:** Aceita
- **Data:** 2026-11-03

## Contexto

O servidor roda no PC do host, atrás de NAT doméstico. Os amigos precisam alcançá-lo sem abrir portas no roteador.

## Decisão

Usar **Tailscale**. O host compartilha só a própria máquina com cada amigo (node sharing). O `sun` não implementa TLS: o túnel WireGuard já criptografa o tráfego.

## Alternativas consideradas

- **Hamachi:** limite de 5 membros no plano grátis, relay frequente (mais latência), tecnologia antiga.
- **ZeroTier:** bom, mas com UX de compartilhamento pior.
- **Port forwarding + TLS:** expõe o PC do host à internet e exige gerenciar certificados.

## Consequências

- Cada amigo instala o Tailscale (documentar na Fase 6).
- O `sun` confia na rede, mas mesmo assim exige autenticação (Fase 1).
