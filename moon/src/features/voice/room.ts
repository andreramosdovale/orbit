import type { Participant, RemoteParticipant, RemoteTrackPublication, Room } from "livekit-client";

/** Uma pessoa na sala, já sem os tipos do SDK. */
export interface VoiceParticipant {
  id: string;
  name: string;
  isLocal: boolean;
  speaking: boolean;
  muted: boolean;
  /** Está transmitindo a tela, pela identidade `<id>#screen`. */
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

/** A transmissão chega como outro participante (ADR 0006), que a lista junta à pessoa. */
const SCREEN_SUFFIX = "#screen";

export function isScreenIdentity(identity: string): boolean {
  return identity.endsWith(SCREEN_SUFFIX);
}

export function screenIdentity(participantId: string): string {
  return `${participantId}${SCREEN_SUFFIX}`;
}

/**
 * Conecta à sala e publica o microfone. O SDK só é carregado aqui, ao entrar na primeira
 * sala: ele é grande, e quem só usa o chat não precisa dele.
 */
export const connectVoiceRoom: ConnectVoiceRoom = async (url, token, events) => {
  const { Room: LiveKitRoom, RoomEvent, Track } = await import("livekit-client");

  const room: Room = new LiveKitRoom();
  const audioElements = new Set<HTMLMediaElement>();
  let watchedId: string | null = null;
  let watchedTrack: import("livekit-client").RemoteTrack | null = null;
  let screenVideo: HTMLVideoElement | null = null;

  // O vídeo e o áudio da transmissão andam juntos: só são baixados por quem assiste.
  const screenPublications = (participantId: string): RemoteTrackPublication[] => {
    const screen = room.remoteParticipants.get(screenIdentity(participantId));
    if (!screen) return [];
    return [Track.Source.ScreenShare, Track.Source.ScreenShareAudio].flatMap((source) => {
      const publication = screen.getTrackPublication(source);
      return publication ? [publication] : [];
    });
  };

  const showWatchedTrack = () => {
    if (watchedTrack && screenVideo) watchedTrack.attach(screenVideo);
  };
  const removeAudio = (track: import("livekit-client").Track) => {
    for (const element of track.detach()) {
      audioElements.delete(element);
      element.remove();
    }
  };
  const stopWatching = () => {
    if (watchedId !== null) {
      for (const publication of screenPublications(watchedId)) {
        // O áudio some na hora: o aviso do SDK só chega depois de uma renegociação.
        if (publication.kind === Track.Kind.Audio && publication.track) {
          removeAudio(publication.track);
        }
        publication.setSubscribed(false);
      }
    }
    if (watchedTrack && screenVideo) watchedTrack.detach(screenVideo);
    watchedId = null;
    watchedTrack = null;
  };
  const endWatching = () => {
    watchedId = null;
    watchedTrack = null;
    events.onWatchedScreenEnded();
  };

  // As vozes são sempre ouvidas. A transmissão só por quem pede, para poupar o upload do host.
  const subscribeMicrophone = (participant: RemoteParticipant) => {
    participant.getTrackPublication(Track.Source.Microphone)?.setSubscribed(true);
  };
  const subscribeMicrophones = () => {
    for (const participant of room.remoteParticipants.values()) subscribeMicrophone(participant);
  };

  const describe = (participant: Participant, isLocal: boolean): VoiceParticipant => {
    const microphone = participant.getTrackPublication(Track.Source.Microphone);
    const screen = room.remoteParticipants
      .get(screenIdentity(participant.identity))
      ?.getTrackPublication(Track.Source.ScreenShare);
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
      ...[...room.remoteParticipants.values()]
        .filter((p) => !isScreenIdentity(p.identity))
        .map((p) => describe(p, false)),
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

  const isWatched = (identity: string) =>
    watchedId !== null && identity === screenIdentity(watchedId);

  room.on(RoomEvent.TrackPublished, (publication, participant) => {
    if (publication.source === Track.Source.Microphone || isWatched(participant.identity)) {
      publication.setSubscribed(true);
    }
  });
  room.on(RoomEvent.TrackUnpublished, (publication, participant) => {
    if (publication.source === Track.Source.ScreenShare && isWatched(participant.identity)) {
      endWatching();
    }
  });
  room.on(RoomEvent.ParticipantDisconnected, (participant) => {
    if (isWatched(participant.identity)) endWatching();
  });
  room.on(RoomEvent.TrackSubscribed, (track, publication, participant) => {
    if (track.kind === Track.Kind.Video) {
      if (publication.source === Track.Source.ScreenShare && isWatched(participant.identity)) {
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
    removeAudio(track);
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
    watchScreen: (participantId) => {
      stopWatching();
      watchedId = participantId;
      for (const publication of screenPublications(participantId)) publication.setSubscribed(true);
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
