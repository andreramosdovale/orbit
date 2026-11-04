import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ChatLayout } from "./ChatLayout";

const GERAL = { id: 1, name: "geral" };
const JOGOS = { id: 2, name: "jogos" };

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), { status });
}

function renderLayout() {
  const onSessionExpired = vi.fn();
  render(
    <ChatLayout
      server="http://host:7700"
      token="a.b.c"
      user={{ id: 1, username: "fulano" }}
      onLogout={() => {}}
      onSessionExpired={onSessionExpired}
    />,
  );
  return { onSessionExpired };
}

function stubResponses(...responses: Array<() => Response>) {
  const fetchMock = vi.fn();
  for (const respond of responses) {
    fetchMock.mockImplementationOnce(async () => respond());
  }
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

function channelButton(name: string) {
  return screen.getByRole("button", { name: `# ${name}` });
}

function createNamed(name: string) {
  fireEvent.change(screen.getByLabelText("Novo canal"), { target: { value: name } });
  fireEvent.click(screen.getByRole("button", { name: "Criar" }));
}

describe("ChatLayout", () => {
  it("lista os canais e seleciona o primeiro", async () => {
    stubResponses(() => jsonResponse(200, [GERAL, JOGOS]));
    renderLayout();

    expect(await screen.findByRole("heading", { name: "# geral" })).toBeTruthy();
    expect(channelButton("geral").getAttribute("aria-current")).toBe("true");
    expect(channelButton("jogos").getAttribute("aria-current")).toBeNull();
  });

  it("clicar em outro canal troca a seleção", async () => {
    stubResponses(() => jsonResponse(200, [GERAL, JOGOS]));
    renderLayout();
    await screen.findByRole("heading", { name: "# geral" });

    fireEvent.click(channelButton("jogos"));

    expect(screen.getByRole("heading", { name: "# jogos" })).toBeTruthy();
    expect(channelButton("jogos").getAttribute("aria-current")).toBe("true");
    expect(channelButton("geral").getAttribute("aria-current")).toBeNull();
  });

  it("criar um canal adiciona e seleciona", async () => {
    stubResponses(
      () => jsonResponse(200, [GERAL]),
      () => jsonResponse(201, JOGOS),
    );
    renderLayout();
    await screen.findByRole("heading", { name: "# geral" });

    createNamed("Jogos");

    expect(await screen.findByRole("heading", { name: "# jogos" })).toBeTruthy();
    expect(channelButton("jogos").getAttribute("aria-current")).toBe("true");
    expect((screen.getByLabelText("Novo canal") as HTMLInputElement).value).toBe("");
  });

  it("mostra o erro de nome repetido junto do formulário", async () => {
    const message = "Já existe um canal com esse nome.";
    stubResponses(
      () => jsonResponse(200, [GERAL]),
      () => jsonResponse(409, { code: "channel_name_taken", message }),
    );
    renderLayout();
    await screen.findByRole("heading", { name: "# geral" });

    createNamed("geral");

    expect((await screen.findByRole("alert")).textContent).toBe(message);
    expect(screen.getAllByRole("listitem")).toHaveLength(1);
    expect((screen.getByLabelText("Novo canal") as HTMLInputElement).value).toBe("geral");
  });

  it("falha ao carregar mostra Tentar de novo, que carrega de novo", async () => {
    const fetchMock = vi.fn().mockRejectedValueOnce(new TypeError("failed to fetch"));
    fetchMock.mockImplementationOnce(async () => jsonResponse(200, [GERAL]));
    vi.stubGlobal("fetch", fetchMock);
    renderLayout();

    expect((await screen.findByRole("alert")).textContent).toBe(
      "Não foi possível conectar ao servidor.",
    );

    fireEvent.click(screen.getByRole("button", { name: "Tentar de novo" }));

    expect(await screen.findByRole("heading", { name: "# geral" })).toBeTruthy();
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("um 401 ao criar avisa que a sessão expirou", async () => {
    stubResponses(
      () => jsonResponse(200, [GERAL]),
      () => jsonResponse(401, { code: "unauthorized", message: "Sua sessão expirou." }),
    );
    const { onSessionExpired } = renderLayout();
    await screen.findByRole("heading", { name: "# geral" });

    createNamed("jogos");

    await vi.waitFor(() => expect(onSessionExpired).toHaveBeenCalledOnce());
  });
});
