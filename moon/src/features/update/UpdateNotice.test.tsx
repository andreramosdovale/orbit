import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { UpdateNotice } from "./UpdateNotice";

const checkForUpdate = vi.hoisted(() => vi.fn());
const installUpdate = vi.hoisted(() => vi.fn());

vi.mock("../../bindings", () => ({ commands: { checkForUpdate, installUpdate } }));

const NEW_VERSION = { status: "ok", data: { version: "0.2.0" } };

describe("UpdateNotice", () => {
  beforeEach(() => {
    checkForUpdate.mockReset();
    installUpdate.mockReset();
    vi.spyOn(console, "warn").mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("não mostra nada sem atualização", async () => {
    checkForUpdate.mockResolvedValue({ status: "ok", data: null });
    const { container } = render(<UpdateNotice />);
    await vi.waitFor(() => expect(checkForUpdate).toHaveBeenCalled());
    await Promise.resolve();
    expect(container.innerHTML).toBe("");
  });

  it("mostra a versão e o botão quando há atualização", async () => {
    checkForUpdate.mockResolvedValue(NEW_VERSION);
    render(<UpdateNotice />);
    await screen.findByText("Versão 0.2.0 disponível");
    screen.getByRole("button", { name: "Atualizar" });
  });

  it("instala ao clicar e mostra que está atualizando", async () => {
    checkForUpdate.mockResolvedValue(NEW_VERSION);
    // O app fecha sozinho: em caso de sucesso, a resposta nem chega.
    installUpdate.mockReturnValue(new Promise(() => {}));
    render(<UpdateNotice />);
    fireEvent.click(await screen.findByRole("button", { name: "Atualizar" }));
    expect(installUpdate).toHaveBeenCalledOnce();
    await screen.findByText("Atualizando…");
    expect(screen.queryByRole("button", { name: "Atualizar" })).toBeNull();
  });

  it("mostra o erro e devolve o botão se a atualização falhar", async () => {
    checkForUpdate.mockResolvedValue(NEW_VERSION);
    installUpdate.mockResolvedValue({ status: "error", error: "Não foi possível baixar." });
    render(<UpdateNotice />);
    fireEvent.click(await screen.findByRole("button", { name: "Atualizar" }));
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("Não foi possível baixar.");
    screen.getByRole("button", { name: "Atualizar" });
  });

  it("não mostra nada se a checagem falhar", async () => {
    checkForUpdate.mockResolvedValue({ status: "error", error: "sem internet" });
    const { container } = render(<UpdateNotice />);
    await vi.waitFor(() => expect(console.warn).toHaveBeenCalledWith("sem internet"));
    expect(container.innerHTML).toBe("");
  });
});
