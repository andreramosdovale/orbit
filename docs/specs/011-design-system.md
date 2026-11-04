# 011. Design system do `moon`

- **Status:** Concluída
- **Fase:** transversal (não pertence a uma fase do roadmap)

## Objetivo

Todas as telas do `moon` passam a usar um único módulo de design system, com formas arredondadas e visual minimalista. Quem desenvolve uma tela nova monta a interface com os componentes e os tokens do módulo, sem escolher cor, raio ou espaçamento à mão.

## Princípios

- **Arredondado:** nenhum canto reto em controles e superfícies. Botões em formato de pílula, campos e cartões com raio generoso.
- **Minimalista:** cor só quando comunica algo. Um único tom de destaque, separação por superfície em vez de bordas, sem sombras, gradientes ou decoração.
- **Poucas opções:** só entra no módulo o que uma tela atual usa.

Conceitos de design aplicados:

| Conceito | Como aparece |
|---|---|
| Hierarquia | Escala tipográfica curta, e o peso e a cor separam o principal do secundário |
| Proximidade | Grade de 4px: pouco espaço dentro de um grupo, mais espaço entre grupos |
| Profundidade por camadas | Cada superfície fica um pouco mais clara que a de baixo, sem sombras |
| Raios concêntricos | Quanto maior a superfície, maior o raio; a interna tem raio menor que a externa |
| Área de clique | Controles de 32px e 40px, alinhados entre si |
| Resposta à ação | Estados de passar o mouse, pressionar, foco e desativado em todo controle |
| Alinhamento óptico | Rótulos alinhados ao texto do campo, e não à curva da pílula |
| Regra de destaque | O violeta aparece só na ação principal, no item atual e no foco |

## Fora de escopo

- Tema claro e troca de tema: o app continua só escuro.
- Biblioteca de UI ou de CSS (Tailwind, Radix etc.).
- Ícones, fonte embutida e animações além de transições curtas.
- Página de catálogo dos componentes (Storybook ou parecida).
- Mudar o comportamento das telas. Na aparência, as únicas adições são a marca do Orbit no cartão de entrada, a inicial da pessoa em um círculo, o título "Canais" na lateral e os painéis flutuantes do chat. O campo de mensagem começa com uma linha e cresce com o texto.

## Mudanças

### Protocolo (`shared` + `docs/protocol.md`)

Nenhuma.

### Dados (SQLite / migrations)

Nenhuma.

### `sun`

Nenhuma.

### `moon` (UI e core Rust)

Módulo novo em `moon/src/ds/`, importado sempre por `src/ds/index.ts`:

```
src/ds/
├── tokens.css     # única fonte de cor, raio, espaçamento e tipografia
├── base.css       # reset, foco visível e prefers-reduced-motion
├── index.ts       # API pública
└── <Componente>.tsx + .css + .test.tsx
```

**Tokens**, como variáveis CSS:

| Grupo | Valores |
|---|---|
| Raio | `--radius-sm` 10px, `--radius-md` 14px, `--radius-lg` 20px, `--radius-xl` 28px, `--radius-full` 999px |
| Espaçamento | `--space-1` a `--space-6`: 4, 8, 12, 16, 24 e 32px |
| Controles | `--control-sm` 32px, `--control-md` 40px |
| Superfícies | `--surface-base`, `--surface-raised`, `--surface-overlay`, `--surface-hover`, `--surface-active` |
| Texto | `--text-primary`, `--text-muted`, `--text-on-accent` |
| Significado | `--accent`, `--accent-hover`, `--accent-soft`, `--success`, `--danger` |
| Tipografia | `--font-sans` (fonte do sistema) e `--text-xs` a `--text-xl`: 12, 14, 16, 20 e 28px; `--leading-tight`, `--leading-normal` |
| Movimento | `--duration-fast`, que vira 0 com `prefers-reduced-motion: reduce` |

**Componentes**, os que as telas atuais precisam:

| Componente | Substitui | Variações |
|---|---|---|
| `Button` | os botões soltos | `primary`, `secondary`, `ghost`, `link`; tamanhos `md` e `sm` |
| `NavItem` | `.channel` | atual via `aria-current` |
| `TextField` | `<label>` + `<input>` | pílula, com rótulo e dica |
| `TextArea` | textarea do `Composer` | |
| `Alert` | `.error`, `.notice`, `.connection-lost` | `danger`, `success`; com ação opcional |
| `Surface` | `.voice-panel` e painéis | `raised`, `overlay` |
| `SectionTitle` | títulos em maiúsculas das listas | |
| `StatusDot` | `.presence-dot` | `online`, `offline` |
| `Avatar` | novo: inicial em um círculo | `md`, `sm`; com estado opcional |
| `VisuallyHidden` | `.visually-hidden` | |

As telas passam a usar esses componentes. O `App.css` fica só com o layout das telas (grid, larguras e rolagem), usando tokens. Os nomes acessíveis, os papéis e os `role="alert"` continuam iguais.

## Critérios de aceite

- [x] Os componentes têm teste em `moon/src/ds/components.test.tsx`: variações, estado desativado e repasse de atributos, e de `ref` nos controles.
- [x] O teste `moon/src/ds/conventions.test.ts` passa e garante, varrendo os arquivos:
  - nenhum valor literal de cor fora de `tokens.css`;
  - nenhum `border-radius` fora dos tokens;
  - nenhum `<button>`, `<input>` ou `<textarea>` em `features/` e `App.tsx`: só dentro de `ds/`;
  - `features/` só importa do módulo por `ds/index.ts`.
- [x] O teste `moon/src/ds/tokens.test.ts` passa e garante contraste WCAG AA (4,5:1) entre cada cor de texto e as superfícies em que ela aparece.
- [x] Todo controle interativo tem foco visível em `:focus-visible`, verificado por teste que lê `base.css` e na conferência visual.
- [x] As transições são desligadas com `prefers-reduced-motion: reduce`.
- [x] `moon/package.json` não ganha dependências.
- [x] Os testes de tela existentes passam sem mudar as consultas por papel e nome. Só muda algum teste que dependa de classe CSS removida, e a mudança é informada no PR.
- [x] Capturas das telas de login, cadastro, chat, queda de conexão e sala de voz, no app real, em 1280x800.
- [x] **Conferência pelo usuário**, porque aparência não se prova por teste: as capturas e o app aprovados.
- [x] `/verify` verde.

## Plano de implementação

Cada passo termina com `/verify` verde.

1. ADR 0016 (design system próprio, sem biblioteca) e `moon/CLAUDE.md`.
2. `tokens.css`, `base.css` e o teste de contraste.
3. Componentes com testes.
4. Migrar login e cadastro, e tirar as capturas para o usuário conferir o rumo antes de seguir.
5. Migrar chat e voz.
6. Limpar o `App.css`, ligar `conventions.test.ts` e tirar as capturas finais.

## Decisões

- **Cor de destaque:** violeta suave (`#8b7cf6`), confirmado pelo usuário. Trocar é mudar um token.
- **Branch:** `feat/design-system` a partir de `feat/salas-de-voz`, porque a voz está no PR #10 e também precisa migrar. O PR do design system só faz merge depois do #10.

## Questões em aberto

Nenhuma.
