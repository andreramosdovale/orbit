import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Composer } from "./Composer";

function renderComposer(error: string | null = null) {
  const onSend = vi.fn();
  render(<Composer channelName="geral" error={error} onSend={onSend} />);
  const field = screen.getByRole("textbox", {
    name: "Mensagem para #geral",
  }) as HTMLTextAreaElement;
  return { onSend, field };
}

function type(field: HTMLTextAreaElement, value: string) {
  fireEvent.change(field, { target: { value } });
}

describe("Composer", () => {
  it("Enter envia sem os espaços das pontas e limpa o campo", () => {
    const { onSend, field } = renderComposer();

    type(field, "  oi  ");
    fireEvent.keyDown(field, { key: "Enter" });

    expect(onSend).toHaveBeenCalledExactlyOnceWith("oi");
    expect(field.value).toBe("");
  });

  it("com a conexão caída, Enter não envia nem apaga o texto", () => {
    const onSend = vi.fn();
    render(<Composer channelName="geral" error={null} offline onSend={onSend} />);
    const field = screen.getByRole("textbox") as HTMLTextAreaElement;

    type(field, "guardado");
    fireEvent.keyDown(field, { key: "Enter" });

    expect(onSend).not.toHaveBeenCalled();
    expect(field.value).toBe("guardado");
  });

  it("Shift+Enter não envia", () => {
    const { onSend, field } = renderComposer();

    type(field, "oi");
    fireEvent.keyDown(field, { key: "Enter", shiftKey: true });

    expect(onSend).not.toHaveBeenCalled();
    expect(field.value).toBe("oi");
  });

  it("não envia texto vazio", () => {
    const { onSend, field } = renderComposer();

    type(field, "   ");
    fireEvent.keyDown(field, { key: "Enter" });

    expect(onSend).not.toHaveBeenCalled();
  });

  it("não envia acima de 2000 caracteres, e avisa sem apagar o texto", () => {
    const { onSend, field } = renderComposer();
    const tooLong = "a".repeat(2001);

    type(field, tooLong);
    fireEvent.keyDown(field, { key: "Enter" });

    expect(onSend).not.toHaveBeenCalled();
    expect(field.value).toBe(tooLong);
    expect(screen.getByRole("alert").textContent).toContain("2000");
  });

  it("aceita exatamente 2000 caracteres, contando acentos como um", () => {
    const { onSend, field } = renderComposer();
    const longest = "á".repeat(2000);

    type(field, longest);
    fireEvent.keyDown(field, { key: "Enter" });

    expect(onSend).toHaveBeenCalledExactlyOnceWith(longest);
  });

  it("mostra o erro recebido", () => {
    renderComposer("Esse canal não existe mais.");

    expect(screen.getByRole("alert").textContent).toBe("Esse canal não existe mais.");
  });
});
