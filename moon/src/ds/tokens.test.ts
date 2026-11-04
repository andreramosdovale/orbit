import { describe, expect, it } from "vitest";
import base from "./base.css?raw";
import tokens from "./tokens.css?raw";

function color(name: string): string {
  const match = tokens.match(new RegExp(`--${name}:\\s*(#[0-9a-f]{6})`));
  if (!match) throw new Error(`token não encontrado: ${name}`);
  return match[1];
}

function luminance(hex: string): number {
  const channel = (start: number) => {
    const value = Number.parseInt(hex.slice(start, start + 2), 16) / 255;
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

function contrast(a: string, b: string): number {
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (light + 0.05) / (dark + 0.05);
}

const SURFACES = [
  "surface-base",
  "surface-raised",
  "surface-overlay",
  "surface-hover",
  "surface-active",
];
const TEXTS = ["text-primary", "text-muted", "success", "danger"];

describe("tokens", () => {
  it.each(TEXTS.flatMap((text) => SURFACES.map((surface) => [text, surface])))(
    "%s sobre %s passa no WCAG AA",
    (text, surface) => {
      expect(contrast(color(text), color(surface))).toBeGreaterThanOrEqual(4.5);
    },
  );

  it.each([
    ["text-on-accent", "accent"],
    ["text-on-accent", "accent-hover"],
    ["text-primary", "accent-soft"],
  ])("%s sobre %s passa no WCAG AA", (text, surface) => {
    expect(contrast(color(text), color(surface))).toBeGreaterThanOrEqual(4.5);
  });

  it("o foco é visível e as transições respeitam a preferência do sistema", () => {
    expect(base).toMatch(/:focus-visible\s*\{[^}]*outline:\s*2px solid var\(--accent\)/);
    expect(base).toMatch(/prefers-reduced-motion: reduce[\s\S]*--duration-fast: 0ms/);
  });
});
