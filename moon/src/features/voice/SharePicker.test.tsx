import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { CaptureTarget } from "../../bindings";
import { SharePicker } from "./SharePicker";

const GAME: CaptureTarget = {
  kind: "window",
  id: "10",
  title: "Jogo",
  app: "jogo.exe",
  process_id: 5,
};
const MONITOR: CaptureTarget = { kind: "screen", index: 1, name: "Tela 1" };

function setup(targets: CaptureTarget[] = [GAME, MONITOR]) {
  const onPick = vi.fn();
  const onCancel = vi.fn();
  render(<SharePicker targets={targets} onPick={onPick} onCancel={onCancel} />);
  return { onPick, onCancel };
}

describe("SharePicker", () => {
  it("avisa que janela tem áudio e tela não", () => {
    setup();

    expect(
      screen.getByRole("heading", { name: "Janelas, com o áudio do aplicativo" }),
    ).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Telas, sem áudio" })).toBeTruthy();
  });

  it("escolher uma janela devolve o alvo", () => {
    const { onPick } = setup();

    fireEvent.click(screen.getByRole("button", { name: /^Jogo/ }));

    expect(onPick).toHaveBeenCalledWith(GAME);
  });

  it("escolher uma tela devolve o alvo", () => {
    const { onPick } = setup();

    fireEvent.click(screen.getByRole("button", { name: "Tela 1" }));

    expect(onPick).toHaveBeenCalledWith(MONITOR);
  });

  it("cancelar avisa quem usa", () => {
    const { onCancel } = setup();

    fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(onCancel).toHaveBeenCalledOnce();
  });

  it("sem janelas, diz isso e ainda oferece as telas", () => {
    setup([MONITOR]);

    expect(screen.getByText("Nenhuma janela disponível.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Tela 1" })).toBeTruthy();
  });
});
