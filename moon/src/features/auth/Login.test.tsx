import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Login } from "./Login";

const SERVER = "http://host:7700";
const PASSWORD = "senha-boa-123";

function renderLogin() {
  const onLoggedIn = vi.fn();
  render(<Login onLoggedIn={onLoggedIn} onCreateAccount={() => {}} />);
  fireEvent.change(screen.getByLabelText("Servidor"), { target: { value: SERVER } });
  fireEvent.change(screen.getByLabelText("Usuário"), { target: { value: "fulano" } });
  fireEvent.change(screen.getByLabelText("Senha"), { target: { value: PASSWORD } });
  return { onLoggedIn };
}

function submit() {
  fireEvent.click(screen.getByRole("button", { name: "Entrar" }));
}

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), { status });
}

describe("Login", () => {
  it("desabilita o botão enquanto envia", async () => {
    let respond: (res: Response) => void = () => {};
    vi.stubGlobal("fetch", () => new Promise((resolve) => (respond = resolve)));
    const { onLoggedIn } = renderLogin();

    submit();

    const button = screen.getByRole("button", { name: "Entrando..." }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);

    respond(jsonResponse(200, { token: "a.b.c", user: { id: 1, username: "fulano" } }));
    await vi.waitFor(() => expect(onLoggedIn).toHaveBeenCalledWith("a.b.c"));
  });

  it("mostra o erro do servidor", async () => {
    const body = { code: "invalid_credentials", message: "Usuário ou senha incorretos." };
    vi.stubGlobal("fetch", async () => jsonResponse(401, body));
    const { onLoggedIn } = renderLogin();

    submit();

    expect((await screen.findByRole("alert")).textContent).toBe("Usuário ou senha incorretos.");
    expect(onLoggedIn).not.toHaveBeenCalled();
  });

  it("salva o servidor e não grava a senha", async () => {
    vi.stubGlobal("fetch", async () =>
      jsonResponse(200, { token: "a.b.c", user: { id: 1, username: "fulano" } }),
    );
    const { onLoggedIn } = renderLogin();

    submit();
    await vi.waitFor(() => expect(onLoggedIn).toHaveBeenCalled());

    expect(localStorage.getItem("orbit.server")).toBe(SERVER);
    const stored = Object.values({ ...localStorage }).join(" ");
    expect(stored).not.toContain(PASSWORD);
  });
});
