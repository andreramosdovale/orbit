import "./StatusDot.css";

/** Só decora: o estado precisa também estar no texto, para leitores de tela. */
export function StatusDot({ status }: { status: "online" | "offline" }) {
  return <span className={`ds-status-dot ds-status-dot-${status}`} aria-hidden="true" />;
}
