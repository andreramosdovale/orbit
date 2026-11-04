import { act, fireEvent, render, screen } from "@testing-library/react";
import { StrictMode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { FakeWebSocket, stubWebSocket } from "./test/fakeWebSocket";

// O comando real depende do core Rust do Tauri.
vi.mock("./bindings", () => ({
  commands: { appVersion: async () => "0.0.0" },
}));

const SERVER = "http://host:7700";
const USER = { id: 1, username: "fulano" };

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), { status });
}

/** `sun` simulado: responde às rotas conhecidas; `channels` pode ser trocado por teste. */
function stubServer(
  routes: Record<string, () => Response> = {},
  channels: () => Response = () => jsonResponse(200, [{ id: 1, name: "geral" }]),
) {
  vi.stubGlobal("fetch", async (url: string) => {
    const path = new URL(url).pathname;
    if (path === "/channels") return channels();
    if (path.endsWith("/messages")) return jsonResponse(200, { messages: [], has_more: false });
    const route = routes[path];
    if (!route) throw new Error(`rota inesperada no teste: ${path}`);
    return route();
  });
}

/** Simula o app aberto com uma sessão salva neste PC. */
function renderWithSavedToken() {
  localStorage.setItem("orbit.server", SERVER);
  localStorage.setItem("orbit.token", "a.b.c");
  render(<App />);
}

async function serverAccepts() {
  await act(async () => {
    FakeWebSocket.last.open();
    FakeWebSocket.last.receive({ type: "auth_ok", user: USER });
  });
}

function fill(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

function click(name: string) {
  fireEvent.click(screen.getByRole("button", { name }));
}

beforeEach(() => {
  stubWebSocket();
  stubServer();
});

describe("App", () => {
  it("sem token, mostra o login e não abre conexão", async () => {
    render(<App />);

    expect(await screen.findByRole("heading", { name: "Entrar" })).toBeTruthy();
    expect(FakeWebSocket.instances).toHaveLength(0);
  });

  it("login com sucesso grava o token e mostra a tela conectada", async () => {
    stubServer({ "/auth/login": () => jsonResponse(200, { token: "a.b.c", user: USER }) });
    render(<App />);

    fill("Servidor", SERVER);
    fill("Usuário", "fulano");
    fill("Senha", "senha-boa-123");
    click("Entrar");
    // Espera o socket, não o texto: "Conectando..." aparece antes do efeito que o abre.
    await vi.waitFor(() => expect(FakeWebSocket.instances).toHaveLength(1));

    expect(localStorage.getItem("orbit.token")).toBe("a.b.c");
    await serverAccepts();
    expect(FakeWebSocket.last.sent.map((m) => JSON.parse(m))).toEqual([
      { type: "auth", token: "a.b.c" },
    ]);
    expect(screen.getByRole("status").textContent).toBe("Conectado como fulano");
  });

  it("no StrictMode, a tentativa descartada fecha e só uma conexão fica aberta", async () => {
    localStorage.setItem("orbit.server", SERVER);
    localStorage.setItem("orbit.token", "a.b.c");
    render(
      <StrictMode>
        <App />
      </StrictMode>,
    );

    await serverAccepts();

    expect(FakeWebSocket.instances.filter((s) => !s.closed)).toHaveLength(1);
    expect(screen.getByRole("status").textContent).toBe("Conectado como fulano");
  });

  it("desmontar o app fecha a conexão", async () => {
    localStorage.setItem("orbit.server", SERVER);
    localStorage.setItem("orbit.token", "a.b.c");
    const { unmount } = render(<App />);
    await serverAccepts();

    unmount();

    expect(FakeWebSocket.last.closed).toBe(true);
  });

  it("com token salvo e auth_ok, abre direto a tela conectada", async () => {
    renderWithSavedToken();

    await serverAccepts();

    expect(FakeWebSocket.last.url).toBe("ws://host:7700/ws");
    expect(screen.getByRole("status").textContent).toBe("Conectado como fulano");
  });

  it("com auth_error, apaga o token e volta ao login com o aviso", async () => {
    renderWithSavedToken();

    await act(async () => {
      FakeWebSocket.last.open();
      FakeWebSocket.last.receive({ type: "auth_error" });
    });

    expect(localStorage.getItem("orbit.token")).toBeNull();
    expect(screen.getByRole("heading", { name: "Entrar" })).toBeTruthy();
    expect(screen.getByRole("status").textContent).toBe("Sua sessão expirou. Entre novamente.");
  });

  it("com servidor inacessível, mantém o token e Tentar de novo conecta", async () => {
    renderWithSavedToken();

    await act(async () => FakeWebSocket.last.fail());

    expect(localStorage.getItem("orbit.token")).toBe("a.b.c");
    expect(screen.getByRole("alert").textContent).toBe("Não foi possível conectar ao servidor.");

    const before = FakeWebSocket.instances.length;
    click("Tentar de novo");
    expect(FakeWebSocket.instances.length).toBe(before + 1);

    await serverAccepts();
    expect(screen.getByRole("status").textContent).toBe("Conectado como fulano");
  });

  it("a conexão caindo depois de conectar mostra Tentar de novo", async () => {
    renderWithSavedToken();
    await serverAccepts();

    await act(async () => FakeWebSocket.last.dropConnection());

    expect(localStorage.getItem("orbit.token")).toBe("a.b.c");
    expect(screen.getByRole("button", { name: "Tentar de novo" })).toBeTruthy();
  });

  it("Sair apaga o token, mantém o servidor e fecha o socket", async () => {
    renderWithSavedToken();
    await serverAccepts();

    click("Sair");

    expect(localStorage.getItem("orbit.token")).toBeNull();
    expect(localStorage.getItem("orbit.server")).toBe(SERVER);
    expect(FakeWebSocket.last.closed).toBe(true);
    expect(screen.getByRole("heading", { name: "Entrar" })).toBeTruthy();
  });

  it("um 401 ao carregar os canais apaga o token e volta ao login com o aviso", async () => {
    stubServer({}, () =>
      jsonResponse(401, { code: "unauthorized", message: "Sua sessão expirou." }),
    );
    renderWithSavedToken();

    await serverAccepts();

    expect(await screen.findByRole("heading", { name: "Entrar" })).toBeTruthy();
    expect(localStorage.getItem("orbit.token")).toBeNull();
    expect(screen.getByRole("status").textContent).toBe("Sua sessão expirou. Entre novamente.");
    expect(FakeWebSocket.last.closed).toBe(true);
  });

  it("criar a conta volta ao login com o usuário preenchido", async () => {
    stubServer({ "/auth/register": () => jsonResponse(201, USER) });
    render(<App />);

    click("Criar conta");
    fill("Servidor", SERVER);
    fill("Usuário", "fulano");
    fill("Senha", "senha-boa-123");
    fill("Confirmar senha", "senha-boa-123");
    click("Criar conta");

    expect((await screen.findByRole("status")).textContent).toBe(
      "Conta fulano criada. Entre com sua senha.",
    );
    expect((screen.getByLabelText("Usuário") as HTMLInputElement).value).toBe("fulano");
    expect(screen.getByRole("heading", { name: "Entrar" })).toBeTruthy();
  });
});
