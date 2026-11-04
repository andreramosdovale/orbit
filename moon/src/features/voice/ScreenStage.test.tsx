import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ScreenStage } from "./ScreenStage";

function setup() {
  const videoRef = vi.fn();
  const onStop = vi.fn();
  const view = render(<ScreenStage name="ana" videoRef={videoRef} onStop={onStop} />);
  return { ...view, videoRef, onStop };
}

describe("ScreenStage", () => {
  it("mostra quem transmite e entrega um vídeo sem som", () => {
    const { container, videoRef } = setup();

    expect(screen.getByRole("region", { name: "Transmissão de ana" })).toBeTruthy();
    const video = container.querySelector("video") as HTMLVideoElement;
    expect(video.muted).toBe(true);
    expect(videoRef).toHaveBeenCalledWith(video);
  });

  it("parar de assistir avisa quem usa", () => {
    const { onStop } = setup();

    fireEvent.click(screen.getByRole("button", { name: "Parar de assistir" }));

    expect(onStop).toHaveBeenCalledOnce();
  });

  it("alterna a tela cheia", () => {
    setup();
    const stage = screen.getByRole("region");
    const request = vi.fn();
    stage.requestFullscreen = request;

    fireEvent.click(screen.getByRole("button", { name: "Tela cheia" }));
    expect(request).toHaveBeenCalledOnce();

    Object.defineProperty(document, "fullscreenElement", { configurable: true, value: stage });
    fireEvent(document, new Event("fullscreenchange"));
    expect(screen.getByRole("button", { name: "Sair da tela cheia" })).toBeTruthy();

    Object.defineProperty(document, "fullscreenElement", { configurable: true, value: null });
    fireEvent(document, new Event("fullscreenchange"));
    expect(screen.getByRole("button", { name: "Tela cheia" })).toBeTruthy();
  });

  it("sair da página solta o vídeo", () => {
    const { unmount, videoRef } = setup();

    unmount();

    expect(videoRef.mock.lastCall?.[0]).toBeNull();
  });
});
