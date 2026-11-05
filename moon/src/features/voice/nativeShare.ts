import {
  type CaptureTarget,
  commands,
  type EndReason,
  events,
  type VideoEncoderKind,
} from "../../bindings";

/** A transmissão no core Rust. Esta interface deixa os testes sem depender do Tauri. */
export interface NativeShare {
  listTargets: () => Promise<CaptureTarget[]>;
  /** Devolve quem está codificando o vídeo. */
  start: (target: CaptureTarget, url: string, token: string) => Promise<VideoEncoderKind>;
  stop: () => Promise<void>;
  /** Avisa quando a transmissão acaba sozinha. Devolve como parar de ouvir. */
  onEnded: (listener: (reason: EndReason) => void) => Promise<() => void>;
}

export const nativeShare: NativeShare = {
  listTargets: () => commands.listCaptureTargets(),
  start: async (target, url, token) => {
    const result = await commands.startScreenShare(target, url, token);
    if (result.status === "error") throw new Error(result.error);
    return result.data;
  },
  stop: async () => {
    await commands.stopScreenShare();
  },
  onEnded: (listener) => events.screenShareEnded.listen((event) => listener(event.payload.reason)),
};
