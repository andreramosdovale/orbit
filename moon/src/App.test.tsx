import { act, fireEvent, render, screen } from "@testing-library/react";
import { StrictMode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { FakeWebSocket, stubWebSocket } from "./test/fakeWebSocket";

// O comando real depende do core Rust do Tauri.
vi.mock("./bindings", () => ({
  commands: {
    appVersion: async () => "0.0.0",
    checkForUpdate: async () => ({ status: "ok", data: null }),
  },
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
    if (path === "/voice-channels") return jsonResponse(200, []);
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

afterEach(() => {
  vi.useRealTimers();
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

  it("com servidor inacessível, tenta sozinho seguindo as esperas", async () => {
    vi.useFakeTimers();
    renderWithSavedToken();
    const advance = (ms: number) => act(() => vi.advanceTimersByTimeAsync(ms));

    await act(async () => FakeWebSocket.last.fail());
    expect(FakeWebSocket.instances).toHaveLength(1);

    await advance(999);
    expect(FakeWebSocket.instances).toHaveLength(1);
    await advance(1);
    expect(FakeWebSocket.instances).toHaveLength(2);

    await act(async () => FakeWebSocket.last.fail());
    await advance(1999);
    expect(FakeWebSocket.instances).toHaveLength(2);
    await advance(1);
    expect(FakeWebSocket.instances).toHaveLength(3);

    await act(async () => {
      FakeWebSocket.last.open();
      FakeWebSocket.last.receive({ type: "auth_ok", user: USER });
    });
    expect(screen.getByRole("status").textContent).toBe("Conectado como fulano");
  });

  it("Tentar de novo não espera o prazo", async () => {
    vi.useFakeTimers();
    renderWithSavedToken();
    await act(async () => FakeWebSocket.last.fail());

    click("Tentar de novo");

    expect(FakeWebSocket.instances).toHaveLength(2);
    const button = screen.getByRole("button", { name: "Tentar de novo" }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
  });

  it("Sair durante a espera cancela a próxima tentativa", async () => {
    vi.useFakeTimers();
    renderWithSavedToken();
    await act(async () => FakeWebSocket.last.fail());

    click("Sair");
    await act(() => vi.advanceTimersByTimeAsync(60_000));

    expect(FakeWebSocket.instances).toHaveLength(1);
    expect(screen.getByRole("heading", { name: "Entrar" })).toBeTruthy();
  });

  it("a conexão caindo tenta de novo na hora, sem clique, e mantém o token", async () => {
    renderWithSavedToken();
    await serverAccepts();
    const before = FakeWebSocket.instances.length;

    await act(async () => FakeWebSocket.last.dropConnection());

    expect(localStorage.getItem("orbit.token")).toBe("a.b.c");
    expect(FakeWebSocket.instances.length).toBe(before + 1);
    expect(screen.getByRole("alert").textContent).toContain("Conexão perdida");
  });

  it("conectar de novo mantém o chat e o texto do campo", async () => {
    renderWithSavedToken();
    await serverAccepts();
    const field = (await screen.findByRole("textbox", {
      name: "Mensagem para #geral",
    })) as HTMLTextAreaElement;
    fireEvent.change(field, { target: { value: "ainda escrevendo" } });

    await act(async () => FakeWebSocket.last.dropConnection());
    expect(screen.getByRole("heading", { name: "# geral" })).toBeTruthy();
    expect(screen.getByRole("status").textContent).toBe("Sem conexão, entrou como fulano");
    await serverAccepts();

    expect(screen.queryByText(/Conexão perdida/)).toBeNull();
    expect(screen.getByRole("status").textContent).toBe("Conectado como fulano");
    expect(
      (screen.getByRole("textbox", { name: "Mensagem para #geral" }) as HTMLTextAreaElement).value,
    ).toBe("ainda escrevendo");
  });

  it("auth_error ao reconectar volta ao login e para as tentativas", async () => {
    renderWithSavedToken();
    await serverAccepts();
    await act(async () => FakeWebSocket.last.dropConnection());
    const before = FakeWebSocket.instances.length;

    await act(async () => {
      FakeWebSocket.last.open();
      FakeWebSocket.last.receive({ type: "auth_error" });
    });

    expect(localStorage.getItem("orbit.token")).toBeNull();
    expect(screen.getByRole("heading", { name: "Entrar" })).toBeTruthy();
    expect(FakeWebSocket.instances.length).toBe(before);
  });

  it("Sair durante a reconexão fecha a tentativa e não tenta mais", async () => {
    renderWithSavedToken();
    await serverAccepts();
    await act(async () => FakeWebSocket.last.dropConnection());
    const attempt = FakeWebSocket.last;
    const before = FakeWebSocket.instances.length;

    click("Sair");

    expect(attempt.closed).toBe(true);
    expect(FakeWebSocket.instances.length).toBe(before);
    expect(screen.getByRole("heading", { name: "Entrar" })).toBeTruthy();
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
