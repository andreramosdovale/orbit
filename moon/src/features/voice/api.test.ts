import { describe, expect, it, vi } from "vitest";
import { createVoiceChannel, listVoiceChannels, livekitUrl, requestVoiceToken } from "./api";

const SERVER = "http://host:7700";

function stubFetch(status: number, body: unknown) {
  const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(body), { status }));
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

describe("salas de voz", () => {
  it("listVoiceChannels envia o token", async () => {
    const channels = [{ id: 1, name: "geral" }];
    const fetchMock = stubFetch(200, channels);

    await expect(listVoiceChannels(SERVER, "a.b.c")).resolves.toEqual(channels);

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("http://host:7700/voice-channels");
    expect(init.method).toBe("GET");
    expect(init.headers.Authorization).toBe("Bearer a.b.c");
  });

  it("createVoiceChannel envia o token e o corpo", async () => {
    const fetchMock = stubFetch(201, { id: 2, name: "jogos" });

    await createVoiceChannel(SERVER, "a.b.c", { name: "jogos" });

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("http://host:7700/voice-channels");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body)).toEqual({ name: "jogos" });
  });

  it("requestVoiceToken pede o token da sala", async () => {
    const join = { token: "x.y.z", livekit_port: 7880 };
    const fetchMock = stubFetch(200, join);

    await expect(requestVoiceToken(SERVER, "a.b.c", 3)).resolves.toEqual(join);

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("http://host:7700/voice-channels/3/token");
    expect(init.method).toBe("POST");
    expect(init.headers.Authorization).toBe("Bearer a.b.c");
  });
});

describe("livekitUrl", () => {
  it("troca a porta e o esquema, mantendo o host", () => {
    expect(livekitUrl("http://100.1.2.3:7700", 7880)).toBe("ws://100.1.2.3:7880");
    expect(livekitUrl(" http://host:7700/ ", 7999)).toBe("ws://host:7999");
    expect(livekitUrl("https://host:7700", 7880)).toBe("wss://host:7880");
    expect(livekitUrl("http://[fd7a::1]:7700", 7880)).toBe("ws://[fd7a::1]:7880");
  });
});
