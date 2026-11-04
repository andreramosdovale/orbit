import { describe, expect, it } from "vitest";

const css = import.meta.glob("/src/**/*.css", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

const code = import.meta.glob(
  ["/src/**/*.{ts,tsx}", "!/src/**/*.test.{ts,tsx}", "!/src/bindings.ts", "!/src/test/**"],
  { query: "?raw", import: "default", eager: true },
) as Record<string, string>;

const LITERAL_COLOR = /#[0-9a-fA-F]{3,8}\b|\b(?:rgb|hsl)a?\(|:\s*(?:white|black|red|green|blue)\b/;

function isInDs(path: string): boolean {
  return path.startsWith("/src/ds/");
}

/** Os arquivos cujo conteúdo casa com `pattern`, para o erro dizer onde está o problema. */
function matching(files: Record<string, string>, pattern: RegExp): string[] {
  return Object.entries(files)
    .filter(([, content]) => pattern.test(content))
    .map(([path]) => path);
}

describe("convenções do design system", () => {
  it("encontra os arquivos que deve varrer", () => {
    expect(Object.keys(css)).toContain("/src/App.css");
    expect(Object.keys(css)).toContain("/src/ds/tokens.css");
    expect(Object.keys(code)).toContain("/src/App.tsx");
    expect(Object.keys(code)).toContain("/src/features/chat/Composer.tsx");
  });

  it("as cores só existem em tokens.css", () => {
    const { "/src/ds/tokens.css": _tokens, ...others } = css;
    expect(matching(others, LITERAL_COLOR)).toEqual([]);
    expect(matching(code, /#[0-9a-fA-F]{6}\b|\brgba?\(/)).toEqual([]);
  });

  it("os raios só vêm dos tokens", () => {
    const literal = Object.entries(css).flatMap(([path, content]) =>
      [...content.matchAll(/border(?:-[a-z]+)*-radius:\s*([^;]+);/g)]
        .filter((match) => !/^var\(--radius-[a-z]+\)$/.test(match[1]))
        .map((match) => `${path}: ${match[0]}`),
    );
    expect(literal).toEqual([]);
  });

  it("as telas não usam controles soltos", () => {
    const screens = Object.fromEntries(Object.entries(code).filter(([path]) => !isInDs(path)));
    expect(matching(screens, /<(?:button|input|textarea)[\s>]/)).toEqual([]);
  });

  it("as telas só importam o módulo por ds/index.ts", () => {
    const screens = Object.fromEntries(Object.entries(code).filter(([path]) => !isInDs(path)));
    expect(matching(screens, /from "[^"]*\/ds\/[^"]+"/)).toEqual([]);
  });
});
