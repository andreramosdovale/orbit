import { describe, expect, it, vi } from "vitest";
import { ApiError } from "../../api/http";
import { createChannel, listChannels, listMessages } from "./api";

const SERVER = "http://host:7700";

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), { status });
}

describe("canais", () => {
  it("listChannels envia o token e resolve com os canais", async () => {
    const channels = [{ id: 1, name: "geral" }];
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, channels));
    vi.stubGlobal("fetch", fetchMock);

    await expect(listChannels(SERVER, "a.b.c")).resolves.toEqual(channels);

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("http://host:7700/channels");
    expect(init.method).toBe("GET");
    expect(init.headers.Authorization).toBe("Bearer a.b.c");
  });

  it("createChannel envia o token e o corpo", async () => {
    const channel = { id: 2, name: "jogos" };
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(201, channel));
    vi.stubGlobal("fetch", fetchMock);

    await expect(createChannel(SERVER, "a.b.c", { name: "jogos" })).resolves.toEqual(channel);

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("http://host:7700/channels");
    expect(init.method).toBe("POST");
    expect(init.headers.Authorization).toBe("Bearer a.b.c");
    expect(JSON.parse(init.body)).toEqual({ name: "jogos" });
  });

  it("um 401 rejeita com ApiError unauthorized", async () => {
    const body = { code: "unauthorized", message: "Sua sessão expirou. Entre novamente." };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse(401, body)));

    const err = await listChannels(SERVER, "a.b.c").catch((e) => e);

    expect(err).toBeInstanceOf(ApiError);
    expect(err.code).toBe("unauthorized");
  });
});

describe("histórico", () => {
  const page = { messages: [], has_more: false };

  it("listMessages sem before busca as mais recentes, com o token", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, page));
    vi.stubGlobal("fetch", fetchMock);

    await expect(listMessages(SERVER, "a.b.c", 3)).resolves.toEqual(page);

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("http://host:7700/channels/3/messages");
    expect(init.method).toBe("GET");
    expect(init.headers.Authorization).toBe("Bearer a.b.c");
  });

  it("listMessages com before pede as anteriores", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, page));
    vi.stubGlobal("fetch", fetchMock);

    await listMessages(SERVER, "a.b.c", 3, 42);

    expect(fetchMock.mock.calls[0][0]).toBe("http://host:7700/channels/3/messages?before=42");
  });
});
