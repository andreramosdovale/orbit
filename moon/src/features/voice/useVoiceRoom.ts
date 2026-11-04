import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError } from "../../api/http";
import { livekitUrl, requestVoiceToken } from "./api";
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
  /** O seletor de tela está aberto ou a transmissão está mudando de estado. */
  sharePending: boolean;
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
}

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
}: UseVoiceRoomOptions) {
  const [state, setState] = useState<VoiceState>(IDLE);
  const room = useRef<VoiceRoom | null>(null);
  // Cada entrada ou saída invalida a anterior: uma resposta atrasada não pode mexer no estado.
  const attempt = useRef(0);

  const closeCurrent = useCallback(() => {
    room.current?.disconnect();
    room.current = null;
  }, []);

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

  const sharing = state.participants.some((p) => p.isLocal && p.sharing);

  const toggleShare = useCallback(async () => {
    const opened = room.current;
    if (!opened) return;
    setState((s) => ({ ...s, sharePending: true, error: null }));
    let error: string | null = null;
    try {
      if (sharing) await opened.stopScreenShare();
      else await opened.startScreenShare();
    } catch (err) {
      error = err instanceof VoiceRoomError ? err.message : "Não foi possível parar a transmissão.";
    }
    // A pessoa pode ter saído da sala com o seletor aberto.
    if (room.current !== opened) return;
    setState((s) => ({ ...s, sharePending: false, error }));
  }, [sharing]);

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
    sharing,
    join,
    leave,
    toggleMute,
    toggleShare,
    watch,
    stopWatching,
    setScreenVideo,
    dismissNotice,
  };
}
