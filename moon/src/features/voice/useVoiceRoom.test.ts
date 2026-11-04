import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CaptureTarget, EndReason } from "../../bindings";
import type { NativeShare } from "./nativeShare";
import { type VoiceEvents, type VoiceParticipant, type VoiceRoom, VoiceRoomError } from "./room";
import { useVoiceRoom } from "./useVoiceRoom";

const SERVER = "http://host:7700";
const ANA: VoiceParticipant = {
  id: "1",
  name: "ana",
  isLocal: true,
  speaking: false,
  muted: false,
  sharing: false,
};
const BETO: VoiceParticipant = { ...ANA, id: "2", name: "beto", isLocal: false, sharing: true };

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), { status });
}

function fakeRoom() {
  return {
    setMuted: vi.fn().mockResolvedValue(undefined),
    watchScreen: vi.fn(),
    stopWatchingScreen: vi.fn(),
    setScreenVideo: vi.fn(),
    disconnect: vi.fn().mockResolvedValue(undefined),
  };
}

const GAME: CaptureTarget = {
  kind: "window",
  id: "10",
  title: "Jogo",
  app: "jogo.exe",
  process_id: 5,
};

function fakeShare() {
  let ended: (reason: EndReason) => void = () => {};
  const unlisten = vi.fn();
  return {
    listTargets: vi.fn().mockResolvedValue([GAME]),
    start: vi.fn().mockResolvedValue(undefined),
    stop: vi.fn().mockResolvedValue(undefined),
    onEnded: vi.fn(async (listener: (reason: EndReason) => void) => {
      ended = listener;
      return unlisten;
    }),
    unlisten,
    end: (reason: EndReason) => ended(reason),
  } satisfies NativeShare & Record<string, unknown>;
}

function setup(
  connect = vi.fn(async (): Promise<VoiceRoom> => fakeRoom()),
  share: NativeShare = fakeShare(),
) {
  const onSessionExpired = vi.fn();
  const view = renderHook(() =>
    useVoiceRoom({ server: SERVER, token: "a.b.c", onSessionExpired, connect, share }),
  );
  return { ...view, connect, onSessionExpired };
}

beforeEach(() => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => jsonResponse(200, { token: "x.y.z", livekit_port: 7999 })),
  );
});

describe("useVoiceRoom", () => {
  it("entrar monta o endereço do LiveKit a partir do servidor e da porta", async () => {
    const { result, connect } = setup();

    await act(() => result.current.join(3));

    const [url, token] = connect.mock.calls[0] as unknown as [string, string];
    expect(url).toBe("ws://host:7999");
    expect(token).toBe("x.y.z");
    expect(result.current.state.status).toBe("connected");
    expect(result.current.state.channelId).toBe(3);
  });

  it("entrar em outra sala desconecta da atual", async () => {
    const first = fakeRoom();
    const second = fakeRoom();
    const connect = vi.fn<() => Promise<VoiceRoom>>();
    connect.mockResolvedValueOnce(first).mockResolvedValueOnce(second);
    const { result } = setup(connect);

    await act(() => result.current.join(1));
    await act(() => result.current.join(2));

    expect(first.disconnect).toHaveBeenCalledOnce();
    expect(second.disconnect).not.toHaveBeenCalled();
    expect(result.current.state.channelId).toBe(2);
  });

  it("uma sala que termina de abrir depois de sair é desconectada", async () => {
    const late = fakeRoom();
    let finish: (room: VoiceRoom) => void = () => {};
    const connect = vi.fn(
      () =>
        new Promise<VoiceRoom>((resolve) => {
          finish = resolve;
        }),
    );
    const { result } = setup(connect);

    act(() => {
      result.current.join(1);
    });
    await waitFor(() => expect(connect).toHaveBeenCalled());
    act(() => result.current.leave());
    await act(async () => finish(late));

    expect(late.disconnect).toHaveBeenCalledOnce();
    expect(result.current.state.status).toBe("idle");
  });

  it("mute chama setMuted e alterna o estado", async () => {
    const room = fakeRoom();
    const { result } = setup(vi.fn(async () => room));
    await act(() => result.current.join(1));

    await act(() => result.current.toggleMute());
    expect(room.setMuted).toHaveBeenLastCalledWith(true);
    expect(result.current.state.muted).toBe(true);

    await act(() => result.current.toggleMute());
    expect(room.setMuted).toHaveBeenLastCalledWith(false);
    expect(result.current.state.muted).toBe(false);
  });

  it("participantes e estado da conexão seguem os eventos", async () => {
    let events: VoiceEvents | undefined;
    const connect = vi.fn(async (_url: string, _token: string, given: VoiceEvents) => {
      events = given;
      return fakeRoom();
    });
    const { result } = setup(connect as never);
    await act(() => result.current.join(1));

    act(() => events?.onParticipants([ANA]));
    expect(result.current.state.participants).toEqual([ANA]);

    act(() => events?.onConnectionState("reconnecting"));
    expect(result.current.state.status).toBe("reconnecting");

    act(() => events?.onConnectionState("connected"));
    expect(result.current.state.status).toBe("connected");

    act(() => events?.onConnectionState("disconnected"));
    expect(result.current.state.status).toBe("idle");
    expect(result.current.state.error).toBe("A conexão de voz caiu.");
  });

  it("eventos de uma sala antiga não mexem no estado", async () => {
    let oldEvents: VoiceEvents | undefined;
    const connect = vi.fn(async (_url: string, _token: string, given: VoiceEvents) => {
      oldEvents ??= given;
      return fakeRoom();
    });
    const { result } = setup(connect as never);
    await act(() => result.current.join(1));
    await act(() => result.current.join(2));

    act(() => oldEvents?.onConnectionState("disconnected"));

    expect(result.current.state.status).toBe("connected");
    expect(result.current.state.channelId).toBe(2);
  });

  it("um erro da sala aparece como texto", async () => {
    const message = "O Windows não liberou o microfone.";
    const { result } = setup(vi.fn().mockRejectedValue(new VoiceRoomError(message)));

    await act(() => result.current.join(1));

    expect(result.current.state.status).toBe("idle");
    expect(result.current.state.error).toBe(message);
  });

  it("voice_unavailable mostra a mensagem do servidor", async () => {
    const message = "A voz não está configurada neste servidor.";
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => jsonResponse(503, { code: "voice_unavailable", message })),
    );
    const { result, connect } = setup();

    await act(() => result.current.join(1));

    expect(result.current.state.error).toBe(message);
    expect(connect).not.toHaveBeenCalled();
  });

  it("unauthorized encerra a sessão", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => jsonResponse(401, { code: "unauthorized", message: "expirou" })),
    );
    const { result, onSessionExpired } = setup();

    await act(() => result.current.join(1));

    expect(onSessionExpired).toHaveBeenCalledOnce();
  });

  it("sair desconecta", async () => {
    const room = fakeRoom();
    const { result } = setup(vi.fn(async () => room));
    await act(() => result.current.join(1));

    act(() => result.current.leave());

    expect(room.disconnect).toHaveBeenCalledOnce();
    expect(result.current.state.status).toBe("idle");
  });

  describe("transmissão de tela", () => {
    /** Entra na sala e guarda os eventos, para simular o que vem do LiveKit. */
    async function joined(room = fakeRoom(), share: NativeShare = fakeShare()) {
      let events = {} as VoiceEvents;
      const connect = vi.fn(async (_url: string, _token: string, received: VoiceEvents) => {
        events = received;
        return room;
      });
      const view = setup(connect as never, share);
      await act(() => view.result.current.join(1));
      return { ...view, room, emit: () => events };
    }

    it("transmitir abre o seletor, e escolher começa com o token próprio", async () => {
      const share = fakeShare();
      const { result } = await joined(fakeRoom(), share);

      await act(() => result.current.toggleShare());
      expect(result.current.state.shareTargets).toEqual([GAME]);
      expect(share.start).not.toHaveBeenCalled();

      await act(() => result.current.startShare(GAME));

      expect(share.start).toHaveBeenCalledWith(GAME, "ws://host:7999", "x.y.z");
      const [url] = vi.mocked(fetch).mock.lastCall as [string];
      expect(url).toBe("http://host:7700/voice-channels/1/screen-token");
      expect(result.current.sharing).toBe(true);
      expect(result.current.state.shareTargets).toBeNull();
    });

    it("cancelar o seletor não transmite nem mostra erro", async () => {
      const share = fakeShare();
      const { result } = await joined(fakeRoom(), share);

      await act(() => result.current.toggleShare());
      act(() => result.current.cancelShare());

      expect(result.current.state.shareTargets).toBeNull();
      expect(result.current.state.error).toBeNull();
      expect(share.start).not.toHaveBeenCalled();
    });

    it("parar encerra no core e para de ouvir", async () => {
      const share = fakeShare();
      const { result } = await joined(fakeRoom(), share);
      await act(() => result.current.startShare(GAME));

      await act(() => result.current.toggleShare());

      expect(share.stop).toHaveBeenCalledOnce();
      expect(share.unlisten).toHaveBeenCalledOnce();
      expect(result.current.sharing).toBe(false);
    });

    it("uma falha ao transmitir mostra a mensagem do core", async () => {
      const share = fakeShare();
      share.start.mockRejectedValue(new Error("O servidor recusou a transmissão."));
      const { result } = await joined(fakeRoom(), share);

      await act(() => result.current.startShare(GAME));

      expect(result.current.state.error).toBe("O servidor recusou a transmissão.");
      expect(result.current.sharing).toBe(false);
      expect(result.current.state.sharePending).toBe(false);
    });

    it("listar as janelas com erro mostra a mensagem", async () => {
      const share = fakeShare();
      share.listTargets.mockRejectedValue(new Error("x"));
      const { result } = await joined(fakeRoom(), share);

      await act(() => result.current.toggleShare());

      expect(result.current.state.error).toBe("Não foi possível listar as janelas.");
    });

    it("um token recusado encerra a sessão", async () => {
      const { result, onSessionExpired } = await joined();
      vi.mocked(fetch).mockResolvedValueOnce(
        jsonResponse(401, { code: "unauthorized", message: "Token inválido." }),
      );

      await act(() => result.current.startShare(GAME));

      expect(onSessionExpired).toHaveBeenCalledOnce();
    });

    it.each([
      ["target_closed", null, "A janela transmitida foi fechada."],
      ["connection_lost", "A transmissão caiu.", null],
      ["failed", "A transmissão parou por um erro.", null],
    ] as const)("o fim por %s avisa", async (reason, error, notice) => {
      const share = fakeShare();
      const { result } = await joined(fakeRoom(), share);
      await act(() => result.current.startShare(GAME));

      act(() => share.end(reason));

      expect(result.current.sharing).toBe(false);
      expect(result.current.state.error).toBe(error);
      expect(result.current.state.notice).toBe(notice);
      expect(share.stop).not.toHaveBeenCalled();
    });

    it("sair da sala encerra a transmissão", async () => {
      const share = fakeShare();
      const { result } = await joined(fakeRoom(), share);
      await act(() => result.current.startShare(GAME));

      act(() => result.current.leave());

      expect(share.stop).toHaveBeenCalledOnce();
      expect(result.current.sharing).toBe(false);
    });

    it("trocar de sala encerra a transmissão", async () => {
      const share = fakeShare();
      const { result } = await joined(fakeRoom(), share);
      await act(() => result.current.startShare(GAME));

      await act(() => result.current.join(2));

      expect(share.stop).toHaveBeenCalledOnce();
      expect(result.current.sharing).toBe(false);
    });

    it("desmontar encerra a transmissão", async () => {
      const share = fakeShare();
      const { result, unmount } = await joined(fakeRoom(), share);
      await act(() => result.current.startShare(GAME));

      unmount();

      expect(share.stop).toHaveBeenCalledOnce();
    });

    it("sair da sala enquanto a transmissão começa também a encerra", async () => {
      const share = fakeShare();
      let finish = () => {};
      share.start.mockReturnValue(
        new Promise<void>((resolve) => {
          finish = resolve;
        }),
      );
      const { result } = await joined(fakeRoom(), share);
      let starting = Promise.resolve();
      await act(async () => {
        starting = result.current.startShare(GAME);
        await Promise.resolve();
        await new Promise((resolve) => setTimeout(resolve));
      });

      act(() => result.current.leave());
      await act(async () => {
        finish();
        await starting;
      });

      expect(share.stop).toHaveBeenCalled();
      expect(result.current.sharing).toBe(false);
      expect(result.current.state.status).toBe("idle");
    });

    it("assistir e parar de assistir chamam a sala", async () => {
      const { result, room } = await joined();

      act(() => result.current.watch("2"));
      expect(room.watchScreen).toHaveBeenCalledWith("2");
      expect(result.current.state.watching).toBe("2");

      act(() => result.current.stopWatching());
      expect(room.stopWatchingScreen).toHaveBeenCalledOnce();
      expect(result.current.state.watching).toBeNull();
    });

    it("o fim da transmissão assistida fecha o vídeo e avisa", async () => {
      const { result, emit } = await joined();
      act(() => emit().onParticipants([ANA, BETO]));
      act(() => result.current.watch("2"));

      act(() => emit().onWatchedScreenEnded());

      expect(result.current.state.watching).toBeNull();
      expect(result.current.state.notice).toBe("A transmissão terminou.");

      act(() => result.current.dismissNotice());
      expect(result.current.state.notice).toBeNull();
    });

    it("assistir de novo apaga o aviso", async () => {
      const { result, emit } = await joined();
      act(() => emit().onWatchedScreenEnded());

      act(() => result.current.watch("2"));

      expect(result.current.state.notice).toBeNull();
    });

    it("trocar de sala e sair limpam o que estava sendo assistido", async () => {
      const { result } = await joined();
      act(() => result.current.watch("2"));

      await act(() => result.current.join(2));
      expect(result.current.state.watching).toBeNull();

      act(() => result.current.watch("2"));
      act(() => result.current.leave());
      expect(result.current.state.watching).toBeNull();
    });

    it("a queda da voz limpa o que estava sendo assistido", async () => {
      const { result, emit } = await joined();
      act(() => result.current.watch("2"));

      act(() => emit().onConnectionState("disconnected"));

      expect(result.current.state.watching).toBeNull();
    });

    it("entrega o elemento de vídeo à sala", async () => {
      const { result, room } = await joined();
      const video = document.createElement("video");

      act(() => result.current.setScreenVideo(video));

      expect(room.setScreenVideo).toHaveBeenCalledWith(video);
    });
  });

  it("desmontar desconecta", async () => {
    const room = fakeRoom();
    const { result, unmount } = setup(vi.fn(async () => room));
    await act(() => result.current.join(1));

    unmount();

    expect(room.disconnect).toHaveBeenCalledOnce();
  });
});
