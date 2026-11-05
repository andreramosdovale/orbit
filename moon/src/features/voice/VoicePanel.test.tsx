import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { VoiceParticipant } from "./room";
import type { VoiceState } from "./useVoiceRoom";
import { VoicePanel } from "./VoicePanel";

function participant(overrides: Partial<VoiceParticipant>): VoiceParticipant {
  return {
    id: "1",
    name: "ana",
    isLocal: false,
    speaking: false,
    muted: false,
    sharing: false,
    ...overrides,
  };
}

function setup(overrides: Partial<VoiceState> = {}, sharing = false) {
  const onToggleMute = vi.fn();
  const onToggleShare = vi.fn();
  const onWatch = vi.fn();
  const onLeave = vi.fn();
  const state: VoiceState = {
    status: "connected",
    channelId: 1,
    participants: [],
    muted: false,
    sharePending: false,
    sharing,
    shareEncoder: null,
    shareTargets: null,
    watching: null,
    notice: null,
    error: null,
    ...overrides,
  };
  render(
    <VoicePanel
      channelName="geral"
      state={state}
      sharing={sharing}
      onToggleMute={onToggleMute}
      onToggleShare={onToggleShare}
      onWatch={onWatch}
      onLeave={onLeave}
    />,
  );
  return { onToggleMute, onToggleShare, onWatch, onLeave };
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

  describe("transmissão", () => {
    it("o botão inicia e, no ar, vira parar", () => {
      const { onToggleShare } = setup();

      fireEvent.click(screen.getByRole("button", { name: "Transmitir tela" }));
      expect(onToggleShare).toHaveBeenCalledOnce();
      expect(screen.queryByText("Você está transmitindo")).toBeNull();
    });

    it.each([
      ["hardware", "Você está transmitindo pela placa de vídeo"],
      ["software", "Você está transmitindo pelo processador"],
    ] as const)("com o encoder %s, o aviso diz qual é", (shareEncoder, text) => {
      setup({ shareEncoder }, true);

      expect(screen.getByText(text)).toBeTruthy();
    });

    it("no ar, avisa e oferece parar", () => {
      setup({}, true);

      expect(screen.getByText("Você está transmitindo")).toBeTruthy();
      expect(screen.getByRole("button", { name: "Parar transmissão" })).toBeTruthy();
    });

    it("com o seletor aberto, o botão fica desativado", () => {
      setup({ sharePending: true });
      expect(
        (screen.getByRole("button", { name: "Transmitir tela" }) as HTMLButtonElement).disabled,
      ).toBe(true);
    });

    it("quem transmite ganha a marcação e o botão de assistir", () => {
      const { onWatch } = setup({
        participants: [participant({ id: "2", name: "beto", sharing: true })],
      });

      expect(screen.getByRole("listitem").textContent).toContain("beto · transmitindo");
      fireEvent.click(screen.getByRole("button", { name: "Assistir à transmissão de beto" }));
      expect(onWatch).toHaveBeenCalledWith("2");
    });

    it("não oferece assistir a si mesmo nem a quem já está na tela", () => {
      setup({
        watching: "2",
        participants: [
          participant({ id: "1", name: "ana", isLocal: true, sharing: true }),
          participant({ id: "2", name: "beto", sharing: true }),
          participant({ id: "3", name: "caio" }),
        ],
      });

      expect(screen.queryByRole("button", { name: /Assistir/ })).toBeNull();
      expect(screen.getAllByRole("listitem")[0].textContent).toBe("ana (você)");
    });
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
