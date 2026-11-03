import type { User } from "../../bindings";

interface ConnectedProps {
  user: User;
  onLogout: () => void;
}

/** Tela depois de entrar. O chat da Fase 2 entra aqui. */
export function Connected({ user, onLogout }: ConnectedProps) {
  return (
    <div className="auth-form">
      <p role="status">
        Conectado como <strong>{user.username}</strong>
      </p>
      <button type="button" onClick={onLogout}>
        Sair
      </button>
    </div>
  );
}
