import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FakeWebSocket, stubWebSocket } from "../../test/fakeWebSocket";
import {
  CONNECT_TIMEOUT_MS,
  connectSession,
  HEARTBEAT_INTERVAL_MS,
  HEARTBEAT_TIMEOUT_MS,
  retryDelayMs,
  SessionError,
  wsUrl,
} from "./session";

const SERVER = "http://host:7700";
const USER = { id: 1, username: "fulano" };

function expectKind(kind: string) {
  return expect.objectContaining({ name: SessionError.name, kind });
}

beforeEach(stubWebSocket);

afterEach(() => {
  vi.useRealTimers();
});

describe("retryDelayMs", () => {
  it("dobra a espera até 30 segundos", () => {
    const delays = [1, 2, 3, 4, 5, 6, 7, 50].map(retryDelayMs);
    expect(delays).toEqual([1000, 2000, 4000, 8000, 16000, 30000, 30000, 30000]);
  });
});

describe("heartbeat", () => {
  async function connected() {
    vi.useFakeTimers();
    const onClose = vi.fn();
    const pending = connectSession(SERVER, "a.b.c", onClose);
    FakeWebSocket.last.open();
    FakeWebSocket.last.receive({ type: "auth_ok", user: USER });
    const session = await pending;
    const socket = FakeWebSocket.last;
    const pings = () => socket.sent.filter((m) => JSON.parse(m).type === "ping").length;
    return { session, socket, onClose, pings };
  }

  it("envia ping a cada 15 segundos", async () => {
    const { pings } = await connected();

    vi.advanceTimersByTime(HEARTBEAT_INTERVAL_MS - 1);
    expect(pings()).toBe(0);

    vi.advanceTimersByTime(1);
    expect(pings()).toBe(1);

    vi.advanceTimersByTime(HEARTBEAT_INTERVAL_MS);
    expect(pings()).toBe(2);
  });

  it("qualquer mensagem do servidor adia o prazo", async () => {
    const { socket, onClose } = await connected();

    for (let i = 0; i < 10; i += 1) {
      vi.advanceTimersByTime(HEARTBEAT_INTERVAL_MS);
      socket.receive({ type: "pong" });
    }

    expect(onClose).not.toHaveBeenCalled();
    expect(socket.closed).toBe(false);
  });

  it("sem receber nada no prazo, avisa uma vez e fecha o socket", async () => {
    const { socket, onClose, pings } = await connected();

    vi.advanceTimersByTime(HEARTBEAT_TIMEOUT_MS - 1);
    expect(onClose).not.toHaveBeenCalled();

    vi.advanceTimersByTime(HEARTBEAT_INTERVAL_MS);
    expect(onClose).toHaveBeenCalledOnce();
    expect(socket.closed).toBe(true);

    const sent = pings();
    vi.advanceTimersByTime(HEARTBEAT_INTERVAL_MS * 4);
    expect(onClose).toHaveBeenCalledOnce();
    expect(pings()).toBe(sent);
  });

  it("close() para os pings", async () => {
    const { session, onClose, pings } = await connected();

    session.close();
    vi.advanceTimersByTime(HEARTBEAT_TIMEOUT_MS * 2);

    expect(pings()).toBe(0);
    expect(onClose).not.toHaveBeenCalled();
  });

  it("a conexão caindo para os pings", async () => {
    const { socket, onClose, pings } = await connected();

    socket.dropConnection();
    vi.advanceTimersByTime(HEARTBEAT_TIMEOUT_MS * 2);

    expect(pings()).toBe(0);
    expect(onClose).toHaveBeenCalledOnce();
  });
});

describe("wsUrl", () => {
  it("converte http, https e barras finais", () => {
    expect(wsUrl("http://host:7700")).toBe("ws://host:7700/ws");
    expect(wsUrl("https://host:7700")).toBe("wss://host:7700/ws");
    expect(wsUrl(" http://host:7700// ")).toBe("ws://host:7700/ws");
  });
});

describe("connectSession", () => {
  it("envia auth como primeira mensagem, com o token", () => {
    connectSession(SERVER, "a.b.c", () => {});
    FakeWebSocket.last.open();

    expect(FakeWebSocket.last.url).toBe("ws://host:7700/ws");
    expect(FakeWebSocket.last.sent.map((m) => JSON.parse(m))).toEqual([
      { type: "auth", token: "a.b.c" },
    ]);
  });

  it("resolve com o usuário no auth_ok", async () => {
    const pending = connectSession(SERVER, "a.b.c", () => {});
    FakeWebSocket.last.open();
    FakeWebSocket.last.receive({ type: "auth_ok", user: USER });

    await expect(pending).resolves.toMatchObject({ user: USER });
  });

  it("rejeita como rejected no auth_error", async () => {
    const pending = connectSession(SERVER, "a.b.c", () => {});
    FakeWebSocket.last.open();
    FakeWebSocket.last.receive({ type: "auth_error" });

    await expect(pending).rejects.toEqual(expectKind("rejected"));
    expect(FakeWebSocket.last.closed).toBe(true);
  });

  it("rejeita como unavailable em erro antes da resposta", async () => {
    const pending = connectSession(SERVER, "a.b.c", () => {});
    FakeWebSocket.last.fail();

    await expect(pending).rejects.toEqual(expectKind("unavailable"));
  });

  it("rejeita como unavailable em fechamento antes da resposta", async () => {
    const pending = connectSession(SERVER, "a.b.c", () => {});
    FakeWebSocket.last.open();
    FakeWebSocket.last.dropConnection();

    await expect(pending).rejects.toEqual(expectKind("unavailable"));
  });

  it("rejeita como unavailable e fecha o socket depois do prazo", async () => {
    vi.useFakeTimers();
    const pending = connectSession(SERVER, "a.b.c", () => {});
    FakeWebSocket.last.open();

    vi.advanceTimersByTime(CONNECT_TIMEOUT_MS);

    await expect(pending).rejects.toEqual(expectKind("unavailable"));
    expect(FakeWebSocket.last.closed).toBe(true);
  });

  it("não expira depois do auth_ok", async () => {
    vi.useFakeTimers();
    const pending = connectSession(SERVER, "a.b.c", () => {});
    FakeWebSocket.last.open();
    FakeWebSocket.last.receive({ type: "auth_ok", user: USER });
    await pending;

    vi.advanceTimersByTime(CONNECT_TIMEOUT_MS);

    expect(FakeWebSocket.last.closed).toBe(false);
  });

  it("chama onClose quando a conexão cai depois do auth_ok", async () => {
    const onClose = vi.fn();
    const pending = connectSession(SERVER, "a.b.c", onClose);
    FakeWebSocket.last.open();
    FakeWebSocket.last.receive({ type: "auth_ok", user: USER });
    await pending;

    FakeWebSocket.last.dropConnection();

    expect(onClose).toHaveBeenCalledOnce();
  });

  it("close() fecha o socket sem chamar onClose", async () => {
    const onClose = vi.fn();
    const pending = connectSession(SERVER, "a.b.c", onClose);
    FakeWebSocket.last.open();
    FakeWebSocket.last.receive({ type: "auth_ok", user: USER });
    const session = await pending;

    session.close();

    expect(FakeWebSocket.last.closed).toBe(true);
    expect(onClose).not.toHaveBeenCalled();
  });

  async function authenticated() {
    const pending = connectSession(SERVER, "a.b.c", () => {});
    FakeWebSocket.last.open();
    FakeWebSocket.last.receive({ type: "auth_ok", user: USER });
    return pending;
  }

  const MESSAGE = {
    id: 1,
    channel_id: 1,
    author: USER,
    content: "oi",
    created_at: "2026-11-03T12:00:00.000Z",
  };

  it("send envia o JSON pelo socket", async () => {
    const session = await authenticated();

    session.send({ type: "send_message", channel_id: 1, content: "oi" });

    expect(JSON.parse(FakeWebSocket.last.sent[1])).toEqual({
      type: "send_message",
      channel_id: 1,
      content: "oi",
    });
  });

  it("subscribe entrega as mensagens, e cancelar interrompe a entrega", async () => {
    const session = await authenticated();
    const handler = vi.fn();
    const unsubscribe = session.subscribe(handler);

    FakeWebSocket.last.receive({ type: "new_message", message: MESSAGE });
    expect(handler).toHaveBeenCalledExactlyOnceWith({ type: "new_message", message: MESSAGE });

    unsubscribe();
    FakeWebSocket.last.receive({ type: "pong" });
    expect(handler).toHaveBeenCalledOnce();
  });

  it("o que chega antes da primeira assinatura é entregue a ela, uma vez", async () => {
    const session = await authenticated();
    FakeWebSocket.last.receive({ type: "new_message", message: MESSAGE });

    const first = vi.fn();
    const unsubscribe = session.subscribe(first);
    expect(first).toHaveBeenCalledExactlyOnceWith({ type: "new_message", message: MESSAGE });
    unsubscribe();

    // O StrictMode assina de novo: o que já foi entregue não se repete.
    const second = vi.fn();
    session.subscribe(second);
    expect(second).not.toHaveBeenCalled();
  });

  it("abort cancela a tentativa e fecha o socket", async () => {
    const controller = new AbortController();
    const pending = connectSession(SERVER, "a.b.c", () => {}, controller.signal);

    controller.abort();

    await expect(pending).rejects.toEqual(expectKind("unavailable"));
    expect(FakeWebSocket.last.closed).toBe(true);
  });

  it("endereço inválido rejeita como unavailable", async () => {
    vi.stubGlobal(
      "WebSocket",
      class {
        constructor() {
          throw new SyntaxError("URL inválida");
        }
      },
    );

    await expect(connectSession("nada", "a.b.c", () => {})).rejects.toEqual(
      expectKind("unavailable"),
    );
  });
});
