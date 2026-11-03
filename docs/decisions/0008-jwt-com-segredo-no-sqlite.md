# 0008. JWT HS256 com segredo guardado no SQLite

- **Status:** Aceita
- **Data:** 2026-11-03

## Contexto

O roadmap pede login com JWT. Depois do login, o `moon` precisa de algo que prove quem é o usuário no `/ws` e nas rotas futuras. Também falta definir onde fica o segredo de assinatura e quanto tempo o token vale. O host não deve ter que configurar nada a mais, e o grupo tem só uns 10 amigos numa rede fechada (ADR 0003).

## Decisão

- O token é um **JWT HS256** com `sub` (id do usuário como string), `username`, `iat` e `exp`.
- Ele vale **30 dias**. Não há refresh nem revogação.
- O segredo tem **32 bytes aleatórios**, é gerado pelo `sun` na primeira vez que ele sobe e fica guardado na tabela `secrets` do `orbit.db`.
- A biblioteca é `jsonwebtoken`, com o backend `rust_crypto`.

## Alternativas consideradas

- **Token opaco guardado no banco (sessões):** permitiria revogar um token, mas o roadmap já fixou JWT, e revogar não é uma necessidade do grupo hoje.
- **Segredo por variável de ambiente (`SUN_JWT_SECRET`):** seria mais uma coisa para o host configurar. Se ele esquecesse, um segredo novo a cada reinício derrubaria todos os logins.
- **Token curto + refresh token:** é mais seguro contra vazamento, mas dobra a complexidade no `sun` e no `moon` sem ganho real numa rede privada.
- **Backend `aws_lc_rs` do `jsonwebtoken`:** exige CMake/NASM para compilar no Windows, o que complica a CI e o setup local.

## Consequências

- Não dá para deslogar alguém à força antes do token expirar. Se for preciso, dá para apagar o segredo `jwt` da tabela `secrets` e reiniciar o `sun`, o que invalida o token de todo mundo.
- Apagar o `orbit.db` invalida todos os tokens, o que é esperado, porque as contas também somem.
- O segredo fica no mesmo arquivo que as contas. Quem tem acesso ao disco do host já tem acesso a tudo, então isso não muda o risco.
