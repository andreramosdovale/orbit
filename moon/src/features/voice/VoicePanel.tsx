import { Avatar, Button, SectionTitle, Surface, VisuallyHidden } from "../../ds";
import type { VoiceState } from "./useVoiceRoom";

interface VoicePanelProps {
  channelName: string;
  state: VoiceState;
  onToggleMute: () => void;
  onLeave: () => void;
}

const STATUS_TEXT: Partial<Record<VoiceState["status"], string>> = {
  connecting: "Conectando...",
  reconnecting: "Reconectando...",
};

/** A sala atual: quem está nela, quem está falando, microfone e saída. */
export function VoicePanel({ channelName, state, onToggleMute, onLeave }: VoicePanelProps) {
  const statusText = STATUS_TEXT[state.status];
  return (
    <Surface as="section" tone="overlay" className="voice-panel" aria-label="Sala de voz atual">
      <SectionTitle>Em {channelName}</SectionTitle>
      {statusText && <p className="voice-status">{statusText}</p>}
      <ul>
        {state.participants.map((participant) => (
          <li
            key={participant.id}
            className={participant.speaking ? "voice-participant speaking" : "voice-participant"}
          >
            <Avatar name={participant.name} size="sm" />
            <span>
              {participant.name}
              {participant.isLocal && " (você)"}
              {participant.muted && (
                <span className="voice-muted" aria-hidden="true">
                  {" "}
                  🔇
                </span>
              )}
            </span>
            {participant.speaking && <VisuallyHidden> (falando)</VisuallyHidden>}
            {participant.muted && <VisuallyHidden> (silenciado)</VisuallyHidden>}
          </li>
        ))}
      </ul>
      <div className="voice-actions">
        <Button size="sm" disabled={state.status !== "connected"} onClick={onToggleMute}>
          {state.muted ? "Ativar microfone" : "Silenciar"}
        </Button>
        <Button size="sm" onClick={onLeave}>
          Sair da sala
        </Button>
      </div>
    </Surface>
  );
}
