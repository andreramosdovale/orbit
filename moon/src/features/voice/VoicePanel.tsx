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
    <section className="voice-panel" aria-label="Sala de voz atual">
      <h3>Em {channelName}</h3>
      {statusText && <p className="voice-status">{statusText}</p>}
      <ul>
        {state.participants.map((participant) => (
          <li
            key={participant.id}
            className={participant.speaking ? "voice-participant speaking" : "voice-participant"}
          >
            {participant.name}
            {participant.isLocal && " (você)"}
            {participant.muted && (
              <span className="voice-muted" aria-hidden="true">
                {" "}
                🔇
              </span>
            )}
            {participant.speaking && <span className="visually-hidden"> (falando)</span>}
            {participant.muted && <span className="visually-hidden"> (silenciado)</span>}
          </li>
        ))}
      </ul>
      <div className="voice-actions">
        <button type="button" disabled={state.status !== "connected"} onClick={onToggleMute}>
          {state.muted ? "Ativar microfone" : "Silenciar"}
        </button>
        <button type="button" onClick={onLeave}>
          Sair da sala
        </button>
      </div>
    </section>
  );
}
