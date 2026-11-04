import { request } from "../../api/http";
import type { CreateChannelRequest, TextChannel } from "../../bindings";

/** `GET /channels`: todos os canais, na ordem de criação. */
export function listChannels(server: string, token: string): Promise<TextChannel[]> {
  return request<TextChannel[]>(server, "/channels", { method: "GET", token });
}

/** `POST /channels`. Resolve com o canal criado ou rejeita com `ApiError`. */
export function createChannel(
  server: string,
  token: string,
  req: CreateChannelRequest,
): Promise<TextChannel> {
  return request<TextChannel>(server, "/channels", { method: "POST", token, body: req });
}
