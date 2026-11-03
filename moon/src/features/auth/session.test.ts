import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FakeWebSocket, stubWebSocket } from "../../test/fakeWebSocket";
import { CONNECT_TIMEOUT_MS, connectSession, SessionError, wsUrl } from "./session";

const SERVER = "http://host:7700";
const USER = { id: 1, username: "fulano" };

function expectKind(kind: string) {
  return expect.objectContaining({ name: SessionError.name, kind });
}

beforeEach(stubWebSocket);

afterEach(() => {
  vi.useRealTimers();
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
