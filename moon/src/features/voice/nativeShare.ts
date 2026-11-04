import { type CaptureTarget, commands, type EndReason, events } from "../../bindings";

/** A transmissão no core Rust. Esta interface deixa os testes sem depender do Tauri. */
export interface NativeShare {
  listTargets: () => Promise<CaptureTarget[]>;
  start: (target: CaptureTarget, url: string, token: string) => Promise<void>;
  stop: () => Promise<void>;
  /** Avisa quando a transmissão acaba sozinha. Devolve como parar de ouvir. */
  onEnded: (listener: (reason: EndReason) => void) => Promise<() => void>;
}

export const nativeShare: NativeShare = {
  listTargets: () => commands.listCaptureTargets(),
  start: async (target, url, token) => {
    const result = await commands.startScreenShare(target, url, token);
    if (result.status === "error") throw new Error(result.error);
  },
  stop: async () => {
    await commands.stopScreenShare();
  },
  onEnded: (listener) => events.screenShareEnded.listen((event) => listener(event.payload.reason)),
};
