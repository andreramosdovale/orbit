import { type FormEvent, useState } from "react";
import { ApiError } from "../../api/http";
import { login } from "./api";
import { loadServer, saveServer } from "./storage";

type Status = { kind: "idle" } | { kind: "sending" } | { kind: "error"; message: string };

interface LoginProps {
  /** Usuário já preenchido, vindo do cadastro. */
  initialUsername?: string;
  /** Aviso mostrado acima do formulário (sessão expirada, conta criada). */
  notice?: string;
  onLoggedIn: (token: string) => void;
  onCreateAccount: () => void;
}

/** Formulário de login. Ver `docs/specs/004-tela-de-login.md`. */
export function Login({ initialUsername = "", notice, onLoggedIn, onCreateAccount }: LoginProps) {
  const [server, setServer] = useState(loadServer);
  const [username, setUsername] = useState(initialUsername);
  const [password, setPassword] = useState("");
  const [status, setStatus] = useState<Status>({ kind: "idle" });

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setStatus({ kind: "sending" });
    saveServer(server);
    try {
      const { token } = await login(server, { username, password });
      setPassword("");
      onLoggedIn(token);
    } catch (err) {
      const message = err instanceof ApiError ? err.message : "Erro inesperado.";
      setStatus({ kind: "error", message });
    }
  }

  const sending = status.kind === "sending";

  return (
    <form className="auth-form" onSubmit={onSubmit}>
      <h1>Entrar</h1>

      {notice && (
        <p className="notice" role="status">
          {notice}
        </p>
      )}

      <label>
        Servidor
        <input
          type="url"
          required
          placeholder="http://100.x.y.z:7700"
          value={server}
          onChange={(e) => setServer(e.target.value)}
        />
      </label>

      <label>
        Usuário
        <input
          required
          autoComplete="username"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
        />
      </label>

      <label>
        Senha
        <input
          type="password"
          required
          autoComplete="current-password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
        />
      </label>

      <button type="submit" disabled={sending}>
        {sending ? "Entrando..." : "Entrar"}
      </button>

      {status.kind === "error" && (
        <p className="error" role="alert">
          {status.message}
        </p>
      )}

      <button type="button" className="link" onClick={onCreateAccount}>
        Criar conta
      </button>
    </form>
  );
}
