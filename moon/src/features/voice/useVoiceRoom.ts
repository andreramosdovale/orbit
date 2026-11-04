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
  error: string | null;
}

const IDLE: VoiceState = {
  status: "idle",
  channelId: null,
  participants: [],
  muted: false,
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

  // Sair da conta ou fechar a tela encerra a sala.
  useEffect(
    () => () => {
      attempt.current += 1;
      closeCurrent();
    },
    [closeCurrent],
  );

  return { state, join, leave, toggleMute };
}
