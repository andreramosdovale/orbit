import { normalizeServerUrl, request } from "../../api/http";
import type { CreateVoiceChannelRequest, VoiceChannel, VoiceJoin } from "../../bindings";

/** `GET /voice-channels`: todas as salas, na ordem de criação. */
export function listVoiceChannels(server: string, token: string): Promise<VoiceChannel[]> {
  return request<VoiceChannel[]>(server, "/voice-channels", { method: "GET", token });
}

/** `POST /voice-channels`. Resolve com a sala criada ou rejeita com `ApiError`. */
export function createVoiceChannel(
  server: string,
  token: string,
  req: CreateVoiceChannelRequest,
): Promise<VoiceChannel> {
  return request<VoiceChannel>(server, "/voice-channels", { method: "POST", token, body: req });
}

/** `POST /voice-channels/{id}/token`: o token para entrar na sala pelo LiveKit. */
export function requestVoiceToken(
  server: string,
  token: string,
  channelId: number,
): Promise<VoiceJoin> {
  return request<VoiceJoin>(server, `/voice-channels/${channelId}/token`, {
    method: "POST",
    token,
  });
}

/** `POST /voice-channels/{id}/screen-token`: o token da transmissão, que o core Rust usa. */
export function requestScreenToken(
  server: string,
  token: string,
  channelId: number,
): Promise<VoiceJoin> {
  return request<VoiceJoin>(server, `/voice-channels/${channelId}/screen-token`, {
    method: "POST",
    token,
  });
}

/** O LiveKit roda no mesmo host do `sun`: `http://host:7700` + 7880 → `ws://host:7880`. */
export function livekitUrl(server: string, port: number): string {
  const { protocol, hostname } = new URL(normalizeServerUrl(server));
  return `${protocol === "https:" ? "wss" : "ws"}://${hostname}:${port}`;
}
