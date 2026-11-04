# 0016. Design system próprio, em CSS puro

- **Status:** Aceita
- **Data:** 2026-11-04

## Contexto

Cada tela do `moon` escolhia cor, raio e espaçamento à mão em um único `App.css`, e os controles usavam a aparência padrão do navegador. Queremos um visual único, arredondado e minimalista, para um app com poucas telas e um grupo pequeno de pessoas.

## Decisão

O `moon` tem um módulo próprio em `src/ds/`: tokens em variáveis CSS e poucos componentes React, sem dependência nova. As telas usam os componentes, e um teste falha se aparecer cor, raio ou controle solto fora do módulo.

## Alternativas consideradas

- **Tailwind:** traz uma etapa a mais no build e espalha as decisões visuais pelo JSX, justamente o que queremos concentrar.
- **Radix, MUI ou parecidas:** resolvem componentes complexos, como menus e diálogos, que o Orbit ainda não tem. Custariam peso e um visual para sobrescrever.

## Consequências

- Mudar a aparência é mudar um token, como a cor de destaque.
- Componentes complexos, se aparecerem, serão feitos à mão ou exigirão um ADR novo.
- Os testes de front processam CSS (`test.css` no `vite.config.ts`), porque os testes do módulo leem os arquivos.
