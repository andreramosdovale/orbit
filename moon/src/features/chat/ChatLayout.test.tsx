import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ChatMessage, ServerMessage } from "../../bindings";
import type { Session } from "../auth/session";
import { connectVoiceRoom } from "../voice/room";
import { ChatLayout } from "./ChatLayout";

// A conexão real depende de WebRTC e de um LiveKit.
vi.mock("../voice/room", async (original) => ({
  ...(await original<typeof import("../voice/room")>()),
  connectVoiceRoom: vi.fn(),
}));

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

const EMPTY_PAGE = { messages: [], has_more: false };
const VOICE_CHANNELS = [{ id: 1, name: "sala" }];

function pathOf(url: string): string {
  const { pathname, search } = new URL(url);
  return `${pathname}${search}`;
}

/**
 * `sun` simulado: `history` responde ao histórico, por caminho (canal vazio se não
 * estiver lá), e `responses` às outras chamadas, em ordem.
 */
function stubServer(
  responses: Array<() => Response>,
  history: Record<string, () => Response | Promise<Response>> = {},
) {
  const queue = [...responses];
  const fetchMock = vi.fn(async (url: string) => {
    const path = pathOf(url);
    if (path.endsWith("/token")) return jsonResponse(200, { token: "x.y.z", livekit_port: 7880 });
    if (path === "/voice-channels") return jsonResponse(200, VOICE_CHANNELS);
    if (path.includes("/messages")) {
      return history[path]?.() ?? jsonResponse(200, EMPTY_PAGE);
    }
    const respond = queue.shift();
    if (!respond) throw new Error(`chamada inesperada no teste: ${path}`);
    return respond();
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

function stubResponses(...responses: Array<() => Response>) {
  return stubServer(responses);
}

function historyCalls(fetchMock: ReturnType<typeof stubServer>): string[] {
  return fetchMock.mock.calls.map(([url]) => pathOf(url)).filter((p) => p.includes("/messages"));
}

function stored(id: number, channelId = GERAL.id): ChatMessage {
  return {
    id,
    channel_id: channelId,
    author: { id: 2, username: "ciclano" },
    content: `guardada ${id}`,
    created_at: "2026-11-03T12:00:00.000Z",
  };
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
    expect(screen.getAllByRole("button", { name: /^# / })).toHaveLength(1);
    expect((screen.getByLabelText("Novo canal") as HTMLInputElement).value).toBe("geral");
  });

  it("falha ao carregar mostra Tentar de novo, que carrega de novo", async () => {
    const fetchMock = stubResponses(
      () => {
        throw new TypeError("failed to fetch");
      },
      () => jsonResponse(200, [GERAL]),
    );
    renderLayout();

    expect((await screen.findByRole("alert")).textContent).toBe(
      "Não foi possível conectar ao servidor.",
    );

    fireEvent.click(screen.getByRole("button", { name: "Tentar de novo" }));

    expect(await screen.findByRole("heading", { name: "# geral" })).toBeTruthy();
    expect(fetchMock.mock.calls.filter(([url]) => pathOf(url) === "/channels")).toHaveLength(2);
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
    const channels = new Promise<Response>((resolve) => {
      respond = resolve;
    });
    vi.stubGlobal("fetch", async (url: string) => {
      if (pathOf(url) === "/channels") return channels;
      return jsonResponse(200, pathOf(url) === "/voice-channels" ? [] : EMPTY_PAGE);
    });
    const { emit } = renderLayout();

    emit({ type: "channel_created", channel: JOGOS });
    respond(jsonResponse(200, [GERAL]));

    expect(await screen.findByRole("button", { name: "# jogos" })).toBeTruthy();
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

  describe("voz", () => {
    const VOICE_ROOM = { setMuted: vi.fn(), disconnect: vi.fn() };

    function voiceButton(name: string) {
      return screen.findByRole("button", { name: `Entrar na sala ${name}` });
    }

    it("voice_channel_created adiciona a sala, sem duplicar", async () => {
      stubResponses(() => jsonResponse(200, [GERAL]));
      const { emit } = renderLayout();
      await voiceButton("sala");

      const novaSala = { id: 2, name: "jogos" };
      emit({ type: "voice_channel_created", channel: novaSala });
      emit({ type: "voice_channel_created", channel: novaSala });

      expect(screen.getAllByRole("button", { name: "Entrar na sala jogos" })).toHaveLength(1);
    });

    it("trocar de canal de texto não sai da sala", async () => {
      vi.mocked(connectVoiceRoom).mockResolvedValue(VOICE_ROOM);
      VOICE_ROOM.disconnect.mockClear();
      stubResponses(() => jsonResponse(200, [GERAL, JOGOS]));
      renderLayout();

      fireEvent.click(await voiceButton("sala"));
      expect(await screen.findByRole("heading", { name: "Em sala" })).toBeTruthy();

      fireEvent.click(channelButton("jogos"));

      expect(screen.getByRole("heading", { name: "# jogos" })).toBeTruthy();
      expect(screen.getByRole("heading", { name: "Em sala" })).toBeTruthy();
      expect(VOICE_ROOM.disconnect).not.toHaveBeenCalled();
    });

    it("uma sessão nova do chat não sai da sala", async () => {
      vi.mocked(connectVoiceRoom).mockResolvedValue(VOICE_ROOM);
      VOICE_ROOM.disconnect.mockClear();
      stubServer([() => jsonResponse(200, [GERAL]), () => jsonResponse(200, [GERAL])]);
      const props = {
        server: "http://host:7700",
        token: "a.b.c",
        onLogout: () => {},
        onSessionExpired: () => {},
      };
      const { rerender } = render(<ChatLayout {...props} session={fakeSession().session} />);
      fireEvent.click(await voiceButton("sala"));
      await screen.findByRole("heading", { name: "Em sala" });

      rerender(<ChatLayout {...props} session={fakeSession().session} />);

      expect(screen.getByRole("heading", { name: "Em sala" })).toBeTruthy();
      expect(VOICE_ROOM.disconnect).not.toHaveBeenCalled();
    });
  });

  describe("reconexão", () => {
    const GERAL_MESSAGES = `/channels/${GERAL.id}/messages`;

    function page(ids: number[]) {
      return () => jsonResponse(200, { messages: ids.map((id) => stored(id)), has_more: false });
    }

    function layout(session: Session, connectionLost?: { retrying: boolean; onRetry: () => void }) {
      return (
        <ChatLayout
          server="http://host:7700"
          token="a.b.c"
          session={session}
          connectionLost={connectionLost}
          onLogout={() => {}}
          onSessionExpired={() => {}}
        />
      );
    }

    it("com a conexão perdida, a faixa aparece e Enter não envia", async () => {
      stubResponses(() => jsonResponse(200, [GERAL]));
      const { session, send } = fakeSession();
      const onRetry = vi.fn();
      render(layout(session, { retrying: false, onRetry }));
      await screen.findByRole("heading", { name: "# geral" });

      expect(screen.getByRole("alert").textContent).toContain("Conexão perdida");
      fireEvent.click(screen.getByRole("button", { name: "Tentar de novo" }));
      expect(onRetry).toHaveBeenCalledOnce();

      const field = screen.getByRole("textbox", { name: "Mensagem para #geral" });
      fireEvent.change(field, { target: { value: "oi" } });
      fireEvent.keyDown(field, { key: "Enter" });
      expect(send).not.toHaveBeenCalled();
    });

    it("durante uma tentativa, Tentar de novo fica desativado", async () => {
      stubResponses(() => jsonResponse(200, [GERAL]));
      render(layout(fakeSession().session, { retrying: true, onRetry: () => {} }));
      await screen.findByRole("heading", { name: "# geral" });

      const button = screen.getByRole("button", { name: "Tentar de novo" }) as HTMLButtonElement;
      expect(button.disabled).toBe(true);
    });

    it("a sessão nova busca de novo, mantém o canal e não repete mensagens", async () => {
      const answers = [page([1]), page([1, 2, 3])];
      const fetchMock = stubServer(
        [() => jsonResponse(200, [GERAL, JOGOS]), () => jsonResponse(200, [GERAL, JOGOS])],
        { [GERAL_MESSAGES]: () => (answers.shift() ?? page([]))() },
      );
      const first = fakeSession();
      const { rerender } = render(layout(first.session));
      await screen.findByText("guardada 1");
      first.emit({ type: "new_message", message: stored(2) });
      expect(screen.getByText("guardada 2")).toBeTruthy();

      const second = fakeSession();
      rerender(layout(second.session));

      expect(await screen.findByText("guardada 3")).toBeTruthy();
      expect(screen.getAllByText("guardada 2")).toHaveLength(1);
      expect(channelButton("geral").getAttribute("aria-current")).toBe("true");
      expect(historyCalls(fetchMock)).toEqual([GERAL_MESSAGES, GERAL_MESSAGES]);
      expect(fetchMock.mock.calls.filter(([url]) => pathOf(url) === "/channels")).toHaveLength(2);

      // A assinatura passou para a sessão nova, e o que chega por ela não se perde.
      second.emit({ type: "new_message", message: { ...stored(4), content: "ao vivo 4" } });
      expect(screen.getByText("ao vivo 4")).toBeTruthy();
    });

    it("o que a sessão nova entrega ao assinar não é apagado pela troca", async () => {
      stubServer([() => jsonResponse(200, [GERAL]), () => jsonResponse(200, [GERAL])]);
      const first = fakeSession();
      const { rerender } = render(layout(first.session));
      await screen.findByRole("heading", { name: "# geral" });

      const second = fakeSession();
      const subscribe = second.session.subscribe;
      second.session.subscribe = (handler) => {
        const unsubscribe = subscribe(handler);
        handler({ type: "new_message", message: { ...stored(9), content: "ao vivo 9" } });
        return unsubscribe;
      };
      rerender(layout(second.session));

      expect(await screen.findByText("ao vivo 9")).toBeTruthy();
    });
  });

  describe("presença", () => {
    const ANA = { id: 7, username: "ana" };
    const BETO = { id: 8, username: "beto" };

    function people() {
      return within(screen.getByRole("complementary", { name: "Pessoas" }));
    }

    async function withSnapshot() {
      stubResponses(() => jsonResponse(200, [GERAL]));
      const { emit } = renderLayout();
      await screen.findByRole("heading", { name: "# geral" });
      emit({
        type: "presence_snapshot",
        users: [
          { user: ANA, online: true },
          { user: BETO, online: false },
        ],
      });
      return emit;
    }

    it("presence_snapshot preenche a lista", async () => {
      await withSnapshot();

      expect(people().getByRole("heading", { name: "Online — 1" })).toBeTruthy();
      expect(people().getByRole("heading", { name: "Offline — 1" })).toBeTruthy();
      expect(people().getByText("ana")).toBeTruthy();
      expect(people().getByText("beto")).toBeTruthy();
    });

    it("user_online e user_offline movem a pessoa de seção", async () => {
      const emit = await withSnapshot();

      emit({ type: "user_online", user: BETO });
      expect(people().getByRole("heading", { name: "Online — 2" })).toBeTruthy();
      expect(people().queryByRole("heading", { name: /^Offline/ })).toBeNull();

      emit({ type: "user_offline", user: ANA });
      emit({ type: "user_offline", user: BETO });
      expect(people().getByRole("heading", { name: "Offline — 2" })).toBeTruthy();
      expect(people().queryByRole("heading", { name: /^Online/ })).toBeNull();
    });

    it("user_online de uma conta desconhecida a adiciona", async () => {
      const emit = await withSnapshot();

      emit({ type: "user_online", user: { id: 9, username: "caio" } });

      expect(people().getByText("caio")).toBeTruthy();
      expect(people().getByRole("heading", { name: "Online — 2" })).toBeTruthy();
    });

    it("um estado repetido não duplica a pessoa", async () => {
      const emit = await withSnapshot();

      emit({ type: "user_online", user: ANA });

      expect(people().getAllByText("ana")).toHaveLength(1);
      expect(people().getByRole("heading", { name: "Online — 1" })).toBeTruthy();
    });
  });

  describe("histórico", () => {
    const GERAL_MESSAGES = `/channels/${GERAL.id}/messages`;
    const JOGOS_MESSAGES = `/channels/${JOGOS.id}/messages`;

    function page(ids: number[], hasMore = false, channelId = GERAL.id) {
      return () =>
        jsonResponse(200, { messages: ids.map((id) => stored(id, channelId)), has_more: hasMore });
    }

    function shownMessages(): string[] {
      return screen
        .getAllByText(/^(guardada|ao vivo) \d+$/)
        .map((element) => element.textContent ?? "");
    }

    it("selecionar um canal carrega e mostra o histórico", async () => {
      stubServer([() => jsonResponse(200, [GERAL, JOGOS])], {
        [JOGOS_MESSAGES]: page([7], false, JOGOS.id),
      });
      renderLayout();
      await screen.findByText("Nenhuma mensagem ainda.");

      fireEvent.click(await screen.findByRole("button", { name: "# jogos" }));

      expect(await screen.findByText("guardada 7")).toBeTruthy();
      expect(screen.getByText("ciclano")).toBeTruthy();
    });

    it("voltar a um canal já carregado não busca de novo", async () => {
      const fetchMock = stubServer([() => jsonResponse(200, [GERAL, JOGOS])], {
        [GERAL_MESSAGES]: page([1]),
      });
      renderLayout();
      await screen.findByText("guardada 1");

      fireEvent.click(channelButton("jogos"));
      await screen.findByText("Nenhuma mensagem ainda.");
      fireEvent.click(channelButton("geral"));

      expect(screen.getByText("guardada 1")).toBeTruthy();
      expect(historyCalls(fetchMock)).toEqual([GERAL_MESSAGES, JOGOS_MESSAGES]);
    });

    it("o botão usa o menor id da última página e coloca as antigas antes", async () => {
      const fetchMock = stubServer([() => jsonResponse(200, [GERAL])], {
        [GERAL_MESSAGES]: page([20, 21], true),
        [`${GERAL_MESSAGES}?before=20`]: page([10, 11]),
      });
      renderLayout();
      await screen.findByText("guardada 21");

      fireEvent.click(screen.getByRole("button", { name: "Carregar mensagens anteriores" }));

      await screen.findByText("guardada 10");
      expect(historyCalls(fetchMock)).toEqual([GERAL_MESSAGES, `${GERAL_MESSAGES}?before=20`]);
      expect(shownMessages()).toEqual(["guardada 10", "guardada 11", "guardada 20", "guardada 21"]);
    });

    it("o botão não aparece com has_more falso", async () => {
      stubServer([() => jsonResponse(200, [GERAL])], { [GERAL_MESSAGES]: page([1]) });
      renderLayout();
      await screen.findByText("guardada 1");

      expect(screen.queryByRole("button", { name: "Carregar mensagens anteriores" })).toBeNull();
    });

    it("uma mensagem recebida pelo WebSocket e pela página aparece uma vez só", async () => {
      let respond: (res: Response) => void = () => {};
      const answer = new Promise<Response>((resolve) => {
        respond = resolve;
      });
      stubServer([() => jsonResponse(200, [GERAL])], { [GERAL_MESSAGES]: () => answer });
      const { emit } = renderLayout();
      await screen.findByRole("heading", { name: "# geral" });

      // Chega enquanto a página está a caminho, e a página também a traz.
      emit({ type: "new_message", message: stored(5) });
      emit({ type: "new_message", message: { ...stored(6), content: "ao vivo 6" } });
      respond(page([4, 5])());

      await screen.findByText("guardada 4");
      expect(shownMessages()).toEqual(["guardada 4", "guardada 5", "ao vivo 6"]);
    });

    it("a resposta de um canal que deixou de estar selecionado não aparece no atual", async () => {
      let respond: (res: Response) => void = () => {};
      const answer = new Promise<Response>((resolve) => {
        respond = resolve;
      });
      stubServer([() => jsonResponse(200, [GERAL, JOGOS])], {
        [GERAL_MESSAGES]: () => answer,
        [JOGOS_MESSAGES]: page([9], false, JOGOS.id),
      });
      renderLayout();
      await screen.findByRole("heading", { name: "# geral" });

      fireEvent.click(channelButton("jogos"));
      await screen.findByText("guardada 9");
      await act(async () => respond(page([1])()));

      expect(screen.queryByText("guardada 1")).toBeNull();
      // Ela fica guardada para quando o canal voltar a ser aberto.
      fireEvent.click(channelButton("geral"));
      expect(await screen.findByText("guardada 1")).toBeTruthy();
    });

    it("erro no carregamento mostra a mensagem, e Tentar de novo busca de novo", async () => {
      const fails = vi.fn(() =>
        jsonResponse(500, { code: "internal", message: "Erro no servidor." }),
      );
      const fetchMock = stubServer([() => jsonResponse(200, [GERAL])], {
        [GERAL_MESSAGES]: () => (fails.mock.calls.length === 0 ? fails() : page([1])()),
      });
      renderLayout();

      expect((await screen.findByRole("alert")).textContent).toBe("Erro no servidor.");
      fireEvent.click(screen.getByRole("button", { name: "Tentar de novo" }));

      expect(await screen.findByText("guardada 1")).toBeTruthy();
      expect(screen.queryByRole("alert")).toBeNull();
      expect(historyCalls(fetchMock)).toEqual([GERAL_MESSAGES, GERAL_MESSAGES]);
    });

    it("erro em uma página antiga não perde o que já apareceu, e Tentar de novo pede a mesma", async () => {
      const older = `${GERAL_MESSAGES}?before=20`;
      const fails = vi.fn(() =>
        jsonResponse(500, { code: "internal", message: "Erro no servidor." }),
      );
      const fetchMock = stubServer([() => jsonResponse(200, [GERAL])], {
        [GERAL_MESSAGES]: page([20], true),
        [older]: () => (fails.mock.calls.length === 0 ? fails() : page([10])()),
      });
      renderLayout();
      await screen.findByText("guardada 20");

      fireEvent.click(screen.getByRole("button", { name: "Carregar mensagens anteriores" }));
      expect((await screen.findByRole("alert")).textContent).toBe("Erro no servidor.");
      expect(screen.getByText("guardada 20")).toBeTruthy();

      fireEvent.click(screen.getByRole("button", { name: "Tentar de novo" }));

      expect(await screen.findByText("guardada 10")).toBeTruthy();
      expect(historyCalls(fetchMock)).toEqual([GERAL_MESSAGES, older, older]);
    });

    it("unauthorized no histórico avisa que a sessão expirou", async () => {
      stubServer([() => jsonResponse(200, [GERAL])], {
        [GERAL_MESSAGES]: () =>
          jsonResponse(401, { code: "unauthorized", message: "Sua sessão expirou." }),
      });
      const { onSessionExpired } = renderLayout();

      await waitFor(() => expect(onSessionExpired).toHaveBeenCalledOnce());
    });
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
