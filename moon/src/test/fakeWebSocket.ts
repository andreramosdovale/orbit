import { vi } from "vitest";
import type { ServerMessage } from "../bindings";

/** `WebSocket` simulado: o teste decide quando abre, o que chega e quando fecha. */
export class FakeWebSocket {
  static instances: FakeWebSocket[] = [];

  /** A conexão mais recente. Falha o teste se nenhuma foi aberta. */
  static get last(): FakeWebSocket {
    const socket = FakeWebSocket.instances[FakeWebSocket.instances.length - 1];
    if (!socket) throw new Error("nenhum WebSocket foi aberto");
    return socket;
  }

  readonly url: string;
  readonly sent: string[] = [];
  closed = false;
  onopen: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  onerror: (() => void) | null = null;
  onclose: (() => void) | null = null;

  constructor(url: string) {
    this.url = url;
    FakeWebSocket.instances.push(this);
  }

  send(data: string) {
    this.sent.push(data);
  }

  close() {
    if (this.closed) return;
    this.closed = true;
    this.onclose?.();
  }

  // Comandos do teste: simulam o que vem do servidor.
  open() {
    this.onopen?.();
  }

  receive(msg: ServerMessage) {
    this.onmessage?.({ data: JSON.stringify(msg) });
  }

  fail() {
    this.onerror?.();
    this.close();
  }

  dropConnection() {
    this.close();
  }
}

/** Troca o `WebSocket` global pelo simulado. O `unstubGlobals` da config desfaz. */
export function stubWebSocket() {
  FakeWebSocket.instances = [];
  vi.stubGlobal("WebSocket", FakeWebSocket);
}
