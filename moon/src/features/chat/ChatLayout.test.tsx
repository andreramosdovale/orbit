import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ServerMessage } from "../../bindings";
import type { Session } from "../auth/session";
import { ChatLayout } from "./ChatLayout";

const GERAL = { id: 1, name: "geral" };
const JOGOS = { id: 2, name: "jogos" };

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), { status });
}

/** Sessão simulada: `emit` faz de conta que a mensagem veio do servidor. */
function fakeSession() {
  let listener: ((message: ServerMessage) => void) | null = null;
  const send = vi.fn();
  const session: Session = {
    user: { id: 1, username: "fulano" },
    send,
    subscribe(handler) {
      listener = handler;
      return () => {
        listener = null;
      };
    },
    close() {},
  };
  const emit = (message: ServerMessage) => act(() => listener?.(message));
  return { session, send, emit };
}

function messageIn(channelId: number, content: string): ServerMessage {
  return {
    type: "new_message",
    message: {
      id: Number(`${channelId}${content.length}`),
      channel_id: channelId,
      author: { id: 2, username: "ciclano" },
      content,
      created_at: "2026-11-03T12:00:00.000Z",
    },
  };
}

function renderLayout() {
  const onSessionExpired = vi.fn();
  const { session, send, emit } = fakeSession();
  render(
    <ChatLayout
      server="http://host:7700"
      token="a.b.c"
      session={session}
      onLogout={() => {}}
      onSessionExpired={onSessionExpired}
    />,
  );
  return { onSessionExpired, send, emit };
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

  it("a mensagem do canal selecionado aparece com autor e texto", async () => {
    stubResponses(() => jsonResponse(200, [GERAL, JOGOS]));
    const { emit } = renderLayout();
    await screen.findByRole("heading", { name: "# geral" });

    emit(messageIn(GERAL.id, "bom dia"));

    expect(screen.getByText("bom dia")).toBeTruthy();
    expect(screen.getByText("ciclano")).toBeTruthy();
  });

  it("a mensagem de outro canal só aparece ao selecionar esse canal", async () => {
    stubResponses(() => jsonResponse(200, [GERAL, JOGOS]));
    const { emit } = renderLayout();
    await screen.findByRole("heading", { name: "# geral" });

    emit(messageIn(JOGOS.id, "partida hoje"));
    expect(screen.queryByText("partida hoje")).toBeNull();

    fireEvent.click(channelButton("jogos"));
    expect(screen.getByText("partida hoje")).toBeTruthy();
  });

  it("channel_created adiciona o canal sem trocar a seleção", async () => {
    stubResponses(() => jsonResponse(200, [GERAL]));
    const { emit } = renderLayout();
    await screen.findByRole("heading", { name: "# geral" });

    emit({ type: "channel_created", channel: JOGOS });

    expect(channelButton("jogos").getAttribute("aria-current")).toBeNull();
    expect(screen.getByRole("heading", { name: "# geral" })).toBeTruthy();
  });

  it("criar um canal recebendo o REST e o channel_created não duplica", async () => {
    stubResponses(
      () => jsonResponse(200, [GERAL]),
      () => jsonResponse(201, JOGOS),
    );
    const { emit } = renderLayout();
    await screen.findByRole("heading", { name: "# geral" });

    // O anúncio chega antes da resposta do POST.
    emit({ type: "channel_created", channel: JOGOS });
    createNamed("jogos");

    expect(await screen.findByRole("heading", { name: "# jogos" })).toBeTruthy();
    expect(screen.getAllByRole("button", { name: "# jogos" })).toHaveLength(1);

    // E um anúncio atrasado também não duplica.
    emit({ type: "channel_created", channel: JOGOS });
    expect(screen.getAllByRole("button", { name: "# jogos" })).toHaveLength(1);
  });

  it("um canal anunciado enquanto a lista carrega não se perde", async () => {
    let respond: (res: Response) => void = () => {};
    vi.stubGlobal("fetch", () => new Promise((resolve) => (respond = resolve)));
    const { emit } = renderLayout();

    emit({ type: "channel_created", channel: JOGOS });
    await act(async () => respond(jsonResponse(200, [GERAL])));

    expect(channelButton("jogos")).toBeTruthy();
    expect(channelButton("geral").getAttribute("aria-current")).toBe("true");
  });

  it("Enter envia send_message com o canal selecionado", async () => {
    stubResponses(() => jsonResponse(200, [GERAL, JOGOS]));
    const { send } = renderLayout();
    await screen.findByRole("heading", { name: "# geral" });
    fireEvent.click(channelButton("jogos"));

    const field = screen.getByRole("textbox", { name: "Mensagem para #jogos" });
    fireEvent.change(field, { target: { value: "oi" } });
    fireEvent.keyDown(field, { key: "Enter" });

    expect(send).toHaveBeenCalledExactlyOnceWith({
      type: "send_message",
      channel_id: JOGOS.id,
      content: "oi",
    });
  });

  it("message_rejected mostra o erro, e enviar de novo limpa", async () => {
    stubResponses(() => jsonResponse(200, [GERAL]));
    const { emit } = renderLayout();
    await screen.findByRole("heading", { name: "# geral" });

    emit({ type: "message_rejected", reason: "unknown_channel" });
    expect(screen.getByRole("alert").textContent).toBe("Esse canal não existe mais.");

    const field = screen.getByRole("textbox", { name: "Mensagem para #geral" });
    fireEvent.change(field, { target: { value: "oi" } });
    fireEvent.keyDown(field, { key: "Enter" });
    expect(screen.queryByRole("alert")).toBeNull();
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
