import { type FormEvent, useState } from "react";
import { ApiError } from "../../api/http";
import { Alert, Button, Surface, TextField } from "../../ds";
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
    <Surface as="form" className="auth-form" onSubmit={onSubmit}>
      <h1>Entrar</h1>

      {notice && <Alert tone="success">{notice}</Alert>}

      <TextField
        label="Servidor"
        type="url"
        required
        placeholder="http://100.x.y.z:7700"
        value={server}
        onChange={(e) => setServer(e.target.value)}
      />

      <TextField
        label="Usuário"
        required
        autoComplete="username"
        value={username}
        onChange={(e) => setUsername(e.target.value)}
      />

      <TextField
        label="Senha"
        type="password"
        required
        autoComplete="current-password"
        value={password}
        onChange={(e) => setPassword(e.target.value)}
      />

      <Button type="submit" variant="primary" disabled={sending}>
        {sending ? "Entrando..." : "Entrar"}
      </Button>

      {status.kind === "error" && <Alert>{status.message}</Alert>}

      <Button variant="link" onClick={onCreateAccount}>
        Criar conta
      </Button>
    </Surface>
  );
}
