import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { UserPresence } from "../../bindings";
import { MemberList, withPresence } from "./MemberList";

function member(id: number, username: string, online: boolean): UserPresence {
  return { user: { id, username }, online };
}

function namesIn(title: RegExp): string[] {
  const section = screen.getByRole("heading", { name: title }).parentElement as HTMLElement;
  return within(section)
    .getAllByRole("listitem")
    .map((item) => item.textContent ?? "");
}

describe("MemberList", () => {
  it("separa online e offline, com a contagem, em ordem alfabética", () => {
    render(
      <MemberList
        members={[
          member(1, "zeca", true),
          member(2, "beltrano", false),
          member(3, "ana", true),
          member(4, "ciclano", false),
          member(5, "bruno", false),
        ]}
      />,
    );

    expect(screen.getByRole("heading", { name: "Online — 2" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Offline — 3" })).toBeTruthy();
    expect(namesIn(/^Online/)).toEqual(["ana (online)", "zeca (online)"]);
    expect(namesIn(/^Offline/)).toEqual([
      "beltrano (offline)",
      "bruno (offline)",
      "ciclano (offline)",
    ]);
  });

  it("uma seção vazia não aparece", () => {
    render(<MemberList members={[member(1, "ana", true)]} />);

    expect(screen.queryByRole("heading", { name: /^Offline/ })).toBeNull();
  });

  it("sem dados, mostra Carregando", () => {
    render(<MemberList members={null} />);

    expect(screen.getByText("Carregando...")).toBeTruthy();
  });
});

describe("withPresence", () => {
  const ana = { id: 1, username: "ana" };

  it("muda o estado de quem já está na lista, sem duplicar", () => {
    expect(withPresence([member(1, "ana", false)], ana, true)).toEqual([member(1, "ana", true)]);
    expect(withPresence([member(1, "ana", true)], ana, true)).toEqual([member(1, "ana", true)]);
  });

  it("acrescenta uma conta desconhecida", () => {
    expect(withPresence([], ana, true)).toEqual([member(1, "ana", true)]);
  });
});
