import type { Participant, Room } from "livekit-client";

/** Uma pessoa na sala, já sem os tipos do SDK. */
export interface VoiceParticipant {
  id: string;
  name: string;
  isLocal: boolean;
  speaking: boolean;
  muted: boolean;
}

export type VoiceConnectionState = "connected" | "reconnecting" | "disconnected";

export interface VoiceEvents {
  onParticipants: (participants: VoiceParticipant[]) => void;
  onConnectionState: (state: VoiceConnectionState) => void;
}

/** Sala conectada. Esta interface isola o SDK, para os testes não dependerem de WebRTC. */
export interface VoiceRoom {
  setMuted: (muted: boolean) => Promise<void>;
  disconnect: () => Promise<void>;
}

export type ConnectVoiceRoom = (
  url: string,
  token: string,
  events: VoiceEvents,
) => Promise<VoiceRoom>;

/** Erro já com o texto para mostrar ao usuário. */
export class VoiceRoomError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "VoiceRoomError";
  }
}

function microphoneError(err: unknown): VoiceRoomError {
  const name = err instanceof Error ? err.name : "";
  if (name === "NotAllowedError") {
    return new VoiceRoomError(
      "O Windows não liberou o microfone. Veja em Configurações > Privacidade e segurança > Microfone.",
    );
  }
  if (name === "NotFoundError") {
    return new VoiceRoomError("Nenhum microfone encontrado.");
  }
  return new VoiceRoomError("Não foi possível usar o microfone.");
}

/**
 * Conecta à sala e publica o microfone. O SDK só é carregado aqui, ao entrar na primeira
 * sala: ele é grande, e quem só usa o chat não precisa dele.
 */
export const connectVoiceRoom: ConnectVoiceRoom = async (url, token, events) => {
  const { Room: LiveKitRoom, RoomEvent, Track } = await import("livekit-client");

  const room: Room = new LiveKitRoom();
  const audioElements = new Set<HTMLMediaElement>();

  const describe = (participant: Participant, isLocal: boolean): VoiceParticipant => {
    const microphone = participant.getTrackPublication(Track.Source.Microphone);
    return {
      id: participant.identity,
      name: participant.name || participant.identity,
      isLocal,
      speaking: participant.isSpeaking,
      muted: !microphone || microphone.isMuted,
    };
  };
  const publish = () => {
    events.onParticipants([
      describe(room.localParticipant, true),
      ...[...room.remoteParticipants.values()].map((p) => describe(p, false)),
    ]);
  };

  for (const event of [
    RoomEvent.ParticipantConnected,
    RoomEvent.ParticipantDisconnected,
    RoomEvent.ActiveSpeakersChanged,
    RoomEvent.TrackMuted,
    RoomEvent.TrackUnmuted,
    RoomEvent.TrackPublished,
    RoomEvent.TrackUnpublished,
    RoomEvent.LocalTrackPublished,
    RoomEvent.LocalTrackUnpublished,
  ]) {
    room.on(event, publish);
  }

  room.on(RoomEvent.TrackSubscribed, (track) => {
    if (track.kind !== Track.Kind.Audio) return;
    const element = track.attach();
    audioElements.add(element);
    document.body.appendChild(element);
  });
  room.on(RoomEvent.TrackUnsubscribed, (track) => {
    for (const element of track.detach()) {
      audioElements.delete(element);
      element.remove();
    }
  });

  room.on(RoomEvent.Reconnecting, () => events.onConnectionState("reconnecting"));
  room.on(RoomEvent.Reconnected, () => events.onConnectionState("connected"));
  room.on(RoomEvent.Disconnected, () => {
    for (const element of audioElements) element.remove();
    audioElements.clear();
    events.onConnectionState("disconnected");
  });

  try {
    await room.connect(url, token);
  } catch {
    throw new VoiceRoomError("Não foi possível conectar à sala de voz.");
  }
  try {
    await room.localParticipant.setMicrophoneEnabled(true);
  } catch (err) {
    await room.disconnect();
    throw microphoneError(err);
  }
  publish();

  return {
    setMuted: async (muted) => {
      await room.localParticipant.setMicrophoneEnabled(!muted);
      publish();
    },
    disconnect: () => room.disconnect(),
  };
};
