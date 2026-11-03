import { type FormEvent, useState } from "react";
import { ApiError, register } from "./api";

const SERVER_KEY = "orbit.server";

function loadServer(): string {
  try {
    return localStorage.getItem(SERVER_KEY) ?? "";
  } catch {
    return "";
  }
}

function saveServer(server: string) {
  try {
    localStorage.setItem(SERVER_KEY, server);
  } catch {
    // Sem localStorage o usuário só precisa redigitar o endereço.
  }
}

type Status =
  | { kind: "idle" }
  | { kind: "sending" }
  | { kind: "success"; username: string }
  | { kind: "error"; message: string };

/** Formulário de auto-cadastro. Ver `docs/specs/001-cadastro-de-usuarios.md`. */
export function Register() {
  const [server, setServer] = useState(loadServer);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [status, setStatus] = useState<Status>({ kind: "idle" });

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    if (password !== confirm) {
      setStatus({ kind: "error", message: "As senhas não conferem." });
      return;
    }

    setStatus({ kind: "sending" });
    saveServer(server);
    try {
      const user = await register(server, { username, password });
      setStatus({ kind: "success", username: user.username });
      setPassword("");
      setConfirm("");
    } catch (err) {
      const message = err instanceof ApiError ? err.message : "Erro inesperado.";
      setStatus({ kind: "error", message });
    }
  }

  const sending = status.kind === "sending";

  return (
    <form className="register" onSubmit={onSubmit}>
      <h1>Criar conta</h1>

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
          minLength={3}
          maxLength={32}
          pattern="[A-Za-z0-9_]+"
          title="Letras, números ou _"
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
          minLength={8}
          maxLength={128}
          autoComplete="new-password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
        />
      </label>

      <label>
        Confirmar senha
        <input
          type="password"
          required
          autoComplete="new-password"
          value={confirm}
          onChange={(e) => setConfirm(e.target.value)}
        />
      </label>

      <button type="submit" disabled={sending}>
        {sending ? "Criando..." : "Criar conta"}
      </button>

      {status.kind === "success" && (
        <p className="success" role="status">
          Conta <strong>{status.username}</strong> criada.
        </p>
      )}
      {status.kind === "error" && (
        <p className="error" role="alert">
          {status.message}
        </p>
      )}
    </form>
  );
}
