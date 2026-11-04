import { useCallback, useEffect, useRef, useState } from "react";
import { commands } from "./bindings";
import { Login } from "./features/auth/Login";
import { Register } from "./features/auth/Register";
import { connectSession, type Session, SessionError } from "./features/auth/session";
import { clearToken, loadServer, loadToken, saveToken } from "./features/auth/storage";
import { ChatLayout } from "./features/chat/ChatLayout";
import "./App.css";

const SESSION_EXPIRED = "Sua sessão expirou. Entre novamente.";

type View =
  | { kind: "login"; notice?: string; username?: string }
  | { kind: "register" }
  | { kind: "connecting" }
  | { kind: "connected"; session: Session; server: string; token: string }
  | { kind: "unavailable" };

function initialView(): View {
  return loadToken() ? { kind: "connecting" } : { kind: "login" };
}

function App() {
  const [version, setVersion] = useState("");
  const [view, setView] = useState<View>(initialView);
  const session = useRef<Session | null>(null);
  const connecting = view.kind === "connecting";

  useEffect(() => {
    commands.appVersion().then(setVersion);
  }, []);

  /** Fecha a conexão, apaga o token e volta ao login. */
  const endSession = useCallback((notice?: string) => {
    session.current?.close();
    session.current = null;
    clearToken();
    setView({ kind: "login", notice });
  }, []);
  const logout = useCallback(() => endSession(), [endSession]);
  const expireSession = useCallback(() => endSession(SESSION_EXPIRED), [endSession]);

  // Toda tentativa de conexão passa por aqui: abrir o app, login e "Tentar de novo".
  useEffect(() => {
    if (!connecting) return;
    const token = loadToken();
    if (!token) {
      setView({ kind: "login" });
      return;
    }

    const server = loadServer();
    const controller = new AbortController();
    const onDropped = () => {
      session.current = null;
      setView({ kind: "unavailable" });
    };
    connectSession(server, token, onDropped, controller.signal)
      .then((opened) => {
        session.current = opened;
        setView({ kind: "connected", session: opened, server, token });
      })
      .catch((err: unknown) => {
        if (controller.signal.aborted) return;
        if (err instanceof SessionError && err.kind === "rejected") {
          endSession(SESSION_EXPIRED);
        } else {
          setView({ kind: "unavailable" });
        }
      });
    return () => controller.abort();
  }, [connecting, endSession]);

  // Fechar o app derruba a conexão aberta.
  useEffect(() => () => session.current?.close(), []);

  return (
    <main className="container">
      {view.kind === "login" && (
        <Login
          initialUsername={view.username}
          notice={view.notice}
          onLoggedIn={(token) => {
            saveToken(token);
            setView({ kind: "connecting" });
          }}
          onCreateAccount={() => setView({ kind: "register" })}
        />
      )}
      {view.kind === "register" && (
        <Register
          onRegistered={(username) =>
            setView({
              kind: "login",
              username,
              notice: `Conta ${username} criada. Entre com sua senha.`,
            })
          }
          onBack={() => setView({ kind: "login" })}
        />
      )}
      {view.kind === "connecting" && <p role="status">Conectando...</p>}
      {view.kind === "connected" && (
        <ChatLayout
          server={view.server}
          token={view.token}
          session={view.session}
          onLogout={logout}
          onSessionExpired={expireSession}
        />
      )}
      {view.kind === "unavailable" && (
        <div className="auth-form">
          <p className="error" role="alert">
            Não foi possível conectar ao servidor.
          </p>
          <button type="button" onClick={() => setView({ kind: "connecting" })}>
            Tentar de novo
          </button>
          <button type="button" className="link" onClick={logout}>
            Sair
          </button>
        </div>
      )}
      <p className="version">Orbit v{version}</p>
    </main>
  );
}

export default App;
