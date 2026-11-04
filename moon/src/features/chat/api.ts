import { request } from "../../api/http";
import type { CreateChannelRequest, MessagePage, TextChannel } from "../../bindings";

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

/** `GET /channels/{id}/messages`: as mensagens mais recentes, ou as anteriores a `before`. */
export function listMessages(
  server: string,
  token: string,
  channelId: number,
  before?: number,
): Promise<MessagePage> {
  const query = before === undefined ? "" : `?before=${before}`;
  return request<MessagePage>(server, `/channels/${channelId}/messages${query}`, {
    method: "GET",
    token,
  });
}
