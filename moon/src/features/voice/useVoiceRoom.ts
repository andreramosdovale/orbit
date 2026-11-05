import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError } from "../../api/http";
import type { CaptureTarget, EndReason, VideoEncoderKind } from "../../bindings";
import { livekitUrl, requestScreenToken, requestVoiceToken } from "./api";
import { type NativeShare, nativeShare } from "./nativeShare";
import {
  type ConnectVoiceRoom,
  connectVoiceRoom,
  type VoiceParticipant,
  type VoiceRoom,
  VoiceRoomError,
} from "./room";

export type VoiceStatus = "idle" | "connecting" | "connected" | "reconnecting";

export interface VoiceState {
  status: VoiceStatus;
  channelId: number | null;
  participants: VoiceParticipant[];
  muted: boolean;
  /** A transmissão está mudando de estado. */
  sharePending: boolean;
  /** Você está transmitindo. */
  sharing: boolean;
  /** Quem codifica a transmissão no ar. */
  shareEncoder: VideoEncoderKind | null;
  /** O que oferecer no seletor de transmissão, com ele aberto. */
  shareTargets: CaptureTarget[] | null;
  /** Id de quem está sendo assistido. */
  watching: string | null;
  /** Aviso que não é um erro, como o fim da transmissão assistida. */
  notice: string | null;
  error: string | null;
}

export type VoiceControls = ReturnType<typeof useVoiceRoom>;

const IDLE: VoiceState = {
  status: "idle",
  channelId: null,
  participants: [],
  muted: false,
  sharePending: false,
  sharing: false,
  shareEncoder: null,
  shareTargets: null,
  watching: null,
  notice: null,
  error: null,
};

interface UseVoiceRoomOptions {
  server: string;
  token: string;
  onSessionExpired: () => void;
  /** Troca a conexão real nos testes. */
  connect?: ConnectVoiceRoom;
  /** Troca o core Rust nos testes. */
  share?: NativeShare;
}

const SHARE_ENDED: Record<EndReason, { notice?: string; error?: string }> = {
  target_closed: { notice: "A janela transmitida foi fechada." },
  connection_lost: { error: "A transmissão caiu." },
  failed: { error: "A transmissão parou por um erro." },
};

function errorMessage(err: unknown): string {
  if (err instanceof ApiError || err instanceof VoiceRoomError) return err.message;
  return "Não foi possível entrar na sala de voz.";
}

/** Estado da sala de voz atual. A pessoa fica em no máximo uma sala por vez. */
export function useVoiceRoom({
  server,
  token,
  onSessionExpired,
  connect = connectVoiceRoom,
  share = nativeShare,
}: UseVoiceRoomOptions) {
  const [state, setState] = useState<VoiceState>(IDLE);
  const room = useRef<VoiceRoom | null>(null);
  // Cada entrada ou saída invalida a anterior: uma resposta atrasada não pode mexer no estado.
  const attempt = useRef(0);

  // Há uma transmissão no ar, ou a caminho: quem sai da sala precisa encerrá-la.
  const shareActive = useRef(false);
  // Cada transmissão invalida a anterior, como `attempt` faz com a sala.
  const shareAttempt = useRef(0);
  const stopListening = useRef<(() => void) | null>(null);

  const endShare = useCallback(() => {
    shareAttempt.current += 1;
    stopListening.current?.();
    stopListening.current = null;
    if (shareActive.current) {
      shareActive.current = false;
      share.stop().catch(() => {});
    }
  }, [share]);

  const closeCurrent = useCallback(() => {
    endShare();
    room.current?.disconnect();
    room.current = null;
  }, [endShare]);

  const join = useCallback(
    async (channelId: number) => {
      const current = ++attempt.current;
      closeCurrent();
      setState({ ...IDLE, status: "connecting", channelId });
      try {
        const voice = await requestVoiceToken(server, token, channelId);
        if (current !== attempt.current) return;
        const opened = await connect(livekitUrl(server, voice.livekit_port), voice.token, {
          onParticipants: (participants) => {
            if (current !== attempt.current) return;
            setState((s) => ({ ...s, participants }));
          },
          onConnectionState: (connection) => {
            if (current !== attempt.current) return;
            if (connection === "disconnected") {
              room.current = null;
              setState({ ...IDLE, error: "A conexão de voz caiu." });
            } else {
              setState((s) => ({ ...s, status: connection }));
            }
          },
          onWatchedScreenEnded: () => {
            if (current !== attempt.current) return;
            setState((s) => ({ ...s, watching: null, notice: "A transmissão terminou." }));
          },
        });
        if (current !== attempt.current) {
          opened.disconnect();
          return;
        }
        room.current = opened;
        setState((s) => ({ ...s, status: "connected" }));
      } catch (err) {
        if (current !== attempt.current) return;
        if (err instanceof ApiError && err.code === "unauthorized") {
          onSessionExpired();
          return;
        }
        setState({ ...IDLE, error: errorMessage(err) });
      }
    },
    [server, token, connect, closeCurrent, onSessionExpired],
  );

  const leave = useCallback(() => {
    attempt.current += 1;
    closeCurrent();
    setState(IDLE);
  }, [closeCurrent]);

  const toggleMute = useCallback(async () => {
    const opened = room.current;
    if (!opened) return;
    const muted = !state.muted;
    try {
      await opened.setMuted(muted);
      setState((s) => ({ ...s, muted, error: null }));
    } catch {
      setState((s) => ({ ...s, error: "Não foi possível mudar o microfone." }));
    }
  }, [state.muted]);

  const toggleShare = useCallback(async () => {
    if (state.sharing) {
      endShare();
      setState((s) => ({ ...s, sharing: false, shareEncoder: null, sharePending: false }));
      return;
    }
    setState((s) => ({ ...s, sharePending: true, error: null }));
    const current = attempt.current;
    try {
      const shareTargets = await share.listTargets();
      if (current !== attempt.current) return;
      setState((s) => ({ ...s, sharePending: false, shareTargets }));
    } catch {
      if (current !== attempt.current) return;
      setState((s) => ({
        ...s,
        sharePending: false,
        error: "Não foi possível listar as janelas.",
      }));
    }
  }, [state.sharing, share, endShare]);

  const cancelShare = useCallback(() => {
    setState((s) => ({ ...s, shareTargets: null }));
  }, []);

  const startShare = useCallback(
    async (target: CaptureTarget) => {
      const channelId = state.channelId;
      if (channelId === null) return;
      endShare();
      const current = ++shareAttempt.current;
      shareActive.current = true;
      setState((s) => ({
        ...s,
        shareTargets: null,
        sharePending: true,
        error: null,
        notice: null,
      }));
      try {
        const screen = await requestScreenToken(server, token, channelId);
        if (current !== shareAttempt.current) return;
        // Ouve antes de começar, para não perder um fim imediato.
        const unlisten = await share.onEnded((reason) => {
          if (current !== shareAttempt.current) return;
          shareActive.current = false;
          stopListening.current?.();
          stopListening.current = null;
          setState((s) => ({
            ...s,
            sharing: false,
            shareEncoder: null,
            sharePending: false,
            ...SHARE_ENDED[reason],
          }));
        });
        if (current !== shareAttempt.current) {
          unlisten();
          return;
        }
        stopListening.current = unlisten;
        const shareEncoder = await share.start(
          target,
          livekitUrl(server, screen.livekit_port),
          screen.token,
        );
        if (current !== shareAttempt.current) {
          // Saiu da sala enquanto ela começava.
          share.stop().catch(() => {});
          return;
        }
        setState((s) => ({ ...s, sharing: true, shareEncoder, sharePending: false }));
      } catch (err) {
        if (current !== shareAttempt.current) return;
        shareActive.current = false;
        stopListening.current?.();
        stopListening.current = null;
        if (err instanceof ApiError && err.code === "unauthorized") {
          onSessionExpired();
          return;
        }
        const error = err instanceof Error ? err.message : "Não foi possível transmitir.";
        setState((s) => ({ ...s, sharePending: false, error }));
      }
    },
    [state.channelId, server, token, share, endShare, onSessionExpired],
  );

  const watch = useCallback((participantId: string) => {
    room.current?.watchScreen(participantId);
    setState((s) => ({ ...s, watching: participantId, notice: null }));
  }, []);

  const stopWatching = useCallback(() => {
    room.current?.stopWatchingScreen();
    setState((s) => ({ ...s, watching: null }));
  }, []);

  const setScreenVideo = useCallback((element: HTMLVideoElement | null) => {
    room.current?.setScreenVideo(element);
  }, []);

  const dismissNotice = useCallback(() => {
    setState((s) => ({ ...s, notice: null }));
  }, []);

  // Sair da conta ou fechar a tela encerra a sala.
  useEffect(
    () => () => {
      attempt.current += 1;
      closeCurrent();
    },
    [closeCurrent],
  );

  return {
    state,
    sharing: state.sharing,
    join,
    leave,
    toggleMute,
    toggleShare,
    cancelShare,
    startShare,
    watch,
    stopWatching,
    setScreenVideo,
    dismissNotice,
  };
}
