import { describe, expect, it, vi } from "vitest";
import { ApiError } from "../../api/http";
import { login } from "./api";

const REQ = { username: "fulano", password: "senha-boa-123" };

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

describe("login", () => {
  it("envia POST /auth/login e resolve com LoginResponse", async () => {
    const body = { token: "a.b.c", user: { id: 1, username: "fulano" } };
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, body));
    vi.stubGlobal("fetch", fetchMock);

    await expect(login("http://host:7700/", REQ)).resolves.toEqual(body);

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("http://host:7700/auth/login");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body)).toEqual(REQ);
  });

  it("um 401 rejeita com ApiError e o message do servidor", async () => {
    const body = { code: "invalid_credentials", message: "Usuário ou senha incorretos." };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse(401, body)));

    const err = await login("http://host:7700", REQ).catch((e) => e);

    expect(err).toBeInstanceOf(ApiError);
    expect(err.code).toBe("invalid_credentials");
    expect(err.message).toBe("Usuário ou senha incorretos.");
  });

  it("falha de rede rejeita com ApiError network", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new TypeError("failed to fetch")));

    const err = await login("http://host:7700", REQ).catch((e) => e);

    expect(err).toBeInstanceOf(ApiError);
    expect(err.code).toBe("network");
  });
});
