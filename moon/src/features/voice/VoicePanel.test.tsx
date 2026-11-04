import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { VoiceParticipant } from "./room";
import type { VoiceState } from "./useVoiceRoom";
import { VoicePanel } from "./VoicePanel";

function participant(overrides: Partial<VoiceParticipant>): VoiceParticipant {
  return { id: "1", name: "ana", isLocal: false, speaking: false, muted: false, ...overrides };
}

function setup(overrides: Partial<VoiceState> = {}) {
  const onToggleMute = vi.fn();
  const onLeave = vi.fn();
  const state: VoiceState = {
    status: "connected",
    channelId: 1,
    participants: [],
    muted: false,
    error: null,
    ...overrides,
  };
  render(
    <VoicePanel channelName="geral" state={state} onToggleMute={onToggleMute} onLeave={onLeave} />,
  );
  return { onToggleMute, onLeave };
}

describe("VoicePanel", () => {
  it("mostra os participantes, quem é você, quem fala e quem está silenciado", () => {
    setup({
      participants: [
        participant({ id: "1", name: "ana", isLocal: true }),
        participant({ id: "2", name: "beto", speaking: true }),
        participant({ id: "3", name: "caio", muted: true }),
      ],
    });

    expect(screen.getByRole("heading", { name: "Em geral" })).toBeTruthy();
    const items = screen.getAllByRole("listitem");
    expect(items[0].textContent).toBe("ana (você)");
    expect(items[1].textContent).toBe("beto (falando)");
    expect(items[1].classList.contains("speaking")).toBe(true);
    expect(items[2].textContent).toContain("(silenciado)");
    expect(items[2].classList.contains("speaking")).toBe(false);
  });

  it("os botões funcionam", () => {
    const { onToggleMute, onLeave } = setup();

    fireEvent.click(screen.getByRole("button", { name: "Silenciar" }));
    expect(onToggleMute).toHaveBeenCalledOnce();

    fireEvent.click(screen.getByRole("button", { name: "Sair da sala" }));
    expect(onLeave).toHaveBeenCalledOnce();
  });

  it("silenciado, o botão oferece ativar o microfone", () => {
    setup({ muted: true });

    expect(screen.getByRole("button", { name: "Ativar microfone" })).toBeTruthy();
  });

  it("fora de connected, mostra o estado e desativa o microfone, mas deixa sair", () => {
    setup({ status: "reconnecting" });

    expect(screen.getByText("Reconectando...")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Silenciar" }) as HTMLButtonElement).disabled).toBe(
      true,
    );
    expect(
      (screen.getByRole("button", { name: "Sair da sala" }) as HTMLButtonElement).disabled,
    ).toBe(false);
  });
});
