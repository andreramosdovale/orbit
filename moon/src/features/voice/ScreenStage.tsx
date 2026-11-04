import { useEffect, useRef, useState } from "react";
import { Button, SectionTitle } from "../../ds";

interface ScreenStageProps {
  /** Quem está transmitindo. */
  name: string;
  /** Recebe o elemento de vídeo, e `null` quando ele sai da página. */
  videoRef: (element: HTMLVideoElement | null) => void;
  onStop: () => void;
}

/** A transmissão assistida. O vídeo não tem som: a voz segue pelo áudio da sala. */
export function ScreenStage({ name, videoRef, onStop }: ScreenStageProps) {
  const stage = useRef<HTMLElement>(null);
  const [fullscreen, setFullscreen] = useState(false);

  useEffect(() => {
    const onChange = () => setFullscreen(document.fullscreenElement === stage.current);
    document.addEventListener("fullscreenchange", onChange);
    return () => document.removeEventListener("fullscreenchange", onChange);
  }, []);

  function toggleFullscreen() {
    if (fullscreen) document.exitFullscreen?.();
    else stage.current?.requestFullscreen?.();
  }

  return (
    <section ref={stage} className="screen-stage" aria-label={`Transmissão de ${name}`}>
      <div className="screen-stage-bar">
        <SectionTitle>Transmissão de {name}</SectionTitle>
        <div className="screen-stage-actions">
          <Button size="sm" onClick={toggleFullscreen}>
            {fullscreen ? "Sair da tela cheia" : "Tela cheia"}
          </Button>
          <Button size="sm" onClick={onStop}>
            Parar de assistir
          </Button>
        </div>
      </div>
      <video ref={videoRef} autoPlay playsInline muted />
    </section>
  );
}
