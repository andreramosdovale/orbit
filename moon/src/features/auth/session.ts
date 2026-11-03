import type { ClientMessage, ServerMessage, User } from "../../bindings";
import { normalizeServerUrl } from "./api";

/**
 * Tempo máximo esperando a resposta da autenticação. É maior que os 10 segundos
 * do `sun` (`WS_AUTH_TIMEOUT`), para uma recusa dele chegar primeiro.
 */
export const CONNECT_TIMEOUT_MS = 15_000;

/** `rejected` = o servidor recusou o token. `unavailable` = não deu para falar com ele. */
export type SessionErrorKind = "rejected" | "unavailable";

export class SessionError extends Error {
  readonly kind: SessionErrorKind;

  constructor(kind: SessionErrorKind) {
    super(kind === "rejected" ? "Token recusado pelo servidor." : "Servidor indisponível.");
    this.name = "SessionError";
    this.kind = kind;
  }
}

export interface Session {
  user: User;
  /** Fecha a conexão a pedido do app. Não dispara `onClose`. */
  close(): void;
}

/** `http://host:7700/` → `ws://host:7700/ws`; `https` vira `wss`. */
export function wsUrl(server: string): string {
  return `${normalizeServerUrl(server).replace(/^http/i, "ws")}/ws`;
}

/**
 * Abre o `/ws` e autentica com `token` (ADR 0009).
 *
 * @param onClose chamado se a conexão cair depois de autenticada.
 * @param signal cancela a tentativa enquanto ela está em andamento.
 */
export function connectSession(
  server: string,
  token: string,
  onClose: () => void,
  signal?: AbortSignal,
): Promise<Session> {
  return new Promise((resolve, reject) => {
    let socket: WebSocket;
    try {
      socket = new WebSocket(wsUrl(server));
    } catch {
      // Endereço que não forma uma URL de WebSocket válida.
      reject(new SessionError("unavailable"));
      return;
    }

    let settled = false;
    let closedByApp = false;

    const fail = (kind: SessionErrorKind) => {
      if (settled) return;
      settled = true;
      closedByApp = true;
      clearTimeout(timer);
      socket.close();
      reject(new SessionError(kind));
    };

    const timer = setTimeout(() => fail("unavailable"), CONNECT_TIMEOUT_MS);
    signal?.addEventListener("abort", () => fail("unavailable"), { once: true });

    socket.onopen = () => {
      const auth: ClientMessage = { type: "auth", token };
      socket.send(JSON.stringify(auth));
    };

    socket.onmessage = (event) => {
      if (settled) return;
      const msg = parseServerMessage(event.data);
      if (msg?.type === "auth_ok") {
        settled = true;
        clearTimeout(timer);
        resolve({
          user: msg.user,
          close() {
            closedByApp = true;
            socket.close();
          },
        });
      } else if (msg?.type === "auth_error") {
        fail("rejected");
      }
    };

    socket.onerror = () => fail("unavailable");

    socket.onclose = () => {
      if (!settled) {
        fail("unavailable");
      } else if (!closedByApp) {
        onClose();
      }
    };
  });
}

function parseServerMessage(data: unknown): ServerMessage | null {
  try {
    return JSON.parse(String(data)) as ServerMessage;
  } catch {
    return null;
  }
}
