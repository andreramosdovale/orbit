import { describe, expect, it } from "vitest";
import { normalizeServerUrl } from "./http";

describe("normalizeServerUrl", () => {
  it("remove espaços e barras finais", () => {
    expect(normalizeServerUrl("  http://host:7700//  ")).toBe("http://host:7700");
  });

  it("mantém um endereço já normalizado", () => {
    expect(normalizeServerUrl("http://host:7700")).toBe("http://host:7700");
  });
});
