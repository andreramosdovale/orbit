import type { Participant, RemoteParticipant, RemoteTrackPublication, Room } from "livekit-client";

/** Uma pessoa na sala, já sem os tipos do SDK. */
export interface VoiceParticipant {
  id: string;
  name: string;
  isLocal: boolean;
  speaking: boolean;
  muted: boolean;
  /** Está transmitindo a tela. */
  sharing: boolean;
}

export type VoiceConnectionState = "connected" | "reconnecting" | "disconnected";

export interface VoiceEvents {
  onParticipants: (participants: VoiceParticipant[]) => void;
  onConnectionState: (state: VoiceConnectionState) => void;
  /** A transmissão que estava sendo assistida terminou. */
  onWatchedScreenEnded: () => void;
}

/** Sala conectada. Esta interface isola o SDK, para os testes não dependerem de WebRTC. */
export interface VoiceRoom {
  setMuted: (muted: boolean) => Promise<void>;
  /** Abre o seletor e transmite. Devolve `false` se a pessoa cancelou o seletor. */
  startScreenShare: () => Promise<boolean>;
  stopScreenShare: () => Promise<void>;
  /** Passa a baixar a tela da pessoa. Só uma por vez: troca a que estiver sendo assistida. */
  watchScreen: (participantId: string) => void;
  stopWatchingScreen: () => void;
  /** O elemento que mostra a tela assistida, ou `null` quando ele sai da página. */
  setScreenVideo: (element: HTMLVideoElement | null) => void;
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

/** Limite de banda da transmissão, por espectador (spec 012). */
const SCREEN_MAX_BITRATE = 4_000_000;
const SCREEN_MAX_FRAMERATE = 30;

/**
 * Conecta à sala e publica o microfone. O SDK só é carregado aqui, ao entrar na primeira
 * sala: ele é grande, e quem só usa o chat não precisa dele.
 */
export const connectVoiceRoom: ConnectVoiceRoom = async (url, token, events) => {
  const {
    Room: LiveKitRoom,
    RoomEvent,
    ScreenSharePresets,
    Track,
  } = await import("livekit-client");

  const room: Room = new LiveKitRoom();
  const audioElements = new Set<HTMLMediaElement>();
  let watchedId: string | null = null;
  let watchedTrack: import("livekit-client").RemoteTrack | null = null;
  let screenVideo: HTMLVideoElement | null = null;

  const screenPublication = (participantId: string): RemoteTrackPublication | undefined =>
    room.remoteParticipants.get(participantId)?.getTrackPublication(Track.Source.ScreenShare);

  const showWatchedTrack = () => {
    if (watchedTrack && screenVideo) watchedTrack.attach(screenVideo);
  };
  const stopWatching = () => {
    if (watchedId !== null) screenPublication(watchedId)?.setSubscribed(false);
    if (watchedTrack && screenVideo) watchedTrack.detach(screenVideo);
    watchedId = null;
    watchedTrack = null;
  };
  const endWatching = () => {
    watchedId = null;
    watchedTrack = null;
    events.onWatchedScreenEnded();
  };

  // O vídeo só é baixado por quem pede, para poupar o upload do host. O áudio é sempre ouvido.
  const subscribeMicrophone = (participant: RemoteParticipant) => {
    participant.getTrackPublication(Track.Source.Microphone)?.setSubscribed(true);
  };
  const subscribeMicrophones = () => {
    for (const participant of room.remoteParticipants.values()) subscribeMicrophone(participant);
  };

  const describe = (participant: Participant, isLocal: boolean): VoiceParticipant => {
    const microphone = participant.getTrackPublication(Track.Source.Microphone);
    const screen = participant.getTrackPublication(Track.Source.ScreenShare);
    return {
      id: participant.identity,
      name: participant.name || participant.identity,
      isLocal,
      speaking: participant.isSpeaking,
      muted: !microphone || microphone.isMuted,
      sharing: !!screen && !screen.isMuted,
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

  room.on(RoomEvent.TrackPublished, (publication, participant) => {
    if (publication.source === Track.Source.Microphone) publication.setSubscribed(true);
    if (publication.source === Track.Source.ScreenShare && participant.identity === watchedId) {
      publication.setSubscribed(true);
    }
  });
  room.on(RoomEvent.TrackUnpublished, (publication, participant) => {
    if (publication.source === Track.Source.ScreenShare && participant.identity === watchedId) {
      endWatching();
    }
  });
  room.on(RoomEvent.ParticipantDisconnected, (participant) => {
    if (participant.identity === watchedId) endWatching();
  });
  room.on(RoomEvent.TrackSubscribed, (track, publication, participant) => {
    if (track.kind === Track.Kind.Video) {
      if (publication.source === Track.Source.ScreenShare && participant.identity === watchedId) {
        watchedTrack = track;
        showWatchedTrack();
      }
      return;
    }
    const element = track.attach();
    audioElements.add(element);
    document.body.appendChild(element);
  });
  room.on(RoomEvent.TrackUnsubscribed, (track) => {
    // O elemento de vídeo é da tela, que cuida dele. Só os de áudio são criados aqui.
    if (track.kind === Track.Kind.Video) return;
    for (const element of track.detach()) {
      audioElements.delete(element);
      element.remove();
    }
  });

  room.on(RoomEvent.Reconnecting, () => events.onConnectionState("reconnecting"));
  room.on(RoomEvent.Reconnected, () => {
    subscribeMicrophones();
    events.onConnectionState("connected");
  });
  room.on(RoomEvent.Disconnected, () => {
    for (const element of audioElements) element.remove();
    audioElements.clear();
    events.onConnectionState("disconnected");
  });

  try {
    await room.connect(url, token, { autoSubscribe: false });
  } catch {
    throw new VoiceRoomError("Não foi possível conectar à sala de voz.");
  }
  try {
    await room.localParticipant.setMicrophoneEnabled(true);
  } catch (err) {
    await room.disconnect();
    throw microphoneError(err);
  }
  subscribeMicrophones();
  publish();

  return {
    setMuted: async (muted) => {
      await room.localParticipant.setMicrophoneEnabled(!muted);
      publish();
    },
    startScreenShare: async () => {
      try {
        await room.localParticipant.setScreenShareEnabled(
          true,
          { audio: false, resolution: ScreenSharePresets.h1080fps30.resolution },
          {
            simulcast: false,
            screenShareEncoding: {
              maxBitrate: SCREEN_MAX_BITRATE,
              maxFramerate: SCREEN_MAX_FRAMERATE,
            },
          },
        );
      } catch (err) {
        if (err instanceof Error && err.name === "NotAllowedError") return false;
        throw new VoiceRoomError("Não foi possível transmitir a tela.");
      }
      publish();
      return true;
    },
    stopScreenShare: async () => {
      await room.localParticipant.setScreenShareEnabled(false);
      publish();
    },
    watchScreen: (participantId) => {
      stopWatching();
      watchedId = participantId;
      screenPublication(participantId)?.setSubscribed(true);
    },
    stopWatchingScreen: stopWatching,
    setScreenVideo: (element) => {
      if (watchedTrack && screenVideo) watchedTrack.detach(screenVideo);
      screenVideo = element;
      showWatchedTrack();
    },
    disconnect: () => room.disconnect(),
  };
};
