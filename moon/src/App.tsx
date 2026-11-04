import { useCallback, useEffect, useRef, useState } from "react";
import { commands } from "./bindings";
import { Alert, Button, Surface } from "./ds";
import { Login } from "./features/auth/Login";
import { Register } from "./features/auth/Register";
import { connectSession, retryDelayMs, type Session, SessionError } from "./features/auth/session";
import { clearToken, loadServer, loadToken, saveToken } from "./features/auth/storage";
import { ChatLayout } from "./features/chat/ChatLayout";
import "./App.css";

const SESSION_EXPIRED = "Sua sessão expirou. Entre novamente.";

/** A tela de chat aberta. Continua montada enquanto a conexão estiver caída (ADR 0013). */
interface Chat {
  session: Session;
  server: string;
  token: string;
}

type View =
  | { kind: "login"; notice?: string; username?: string }
  | { kind: "register" }
  | { kind: "online"; chat: Chat }
  | {
      kind: "offline";
      /** Há uma tentativa em andamento. Se não, está na espera até a próxima. */
      trying: boolean;
      /** Quantas tentativas seguidas já falharam. */
      failures: number;
      chat?: Chat;
    };

const CONNECTING: View = { kind: "offline", trying: true, failures: 0 };

function initialView(): View {
  return loadToken() ? CONNECTING : { kind: "login" };
}

function App() {
  const [version, setVersion] = useState("");
  const [view, setView] = useState<View>(initialView);
  const session = useRef<Session | null>(null);
  const trying = view.kind === "offline" && view.trying;
  const waitMs = view.kind === "offline" && !view.trying ? retryDelayMs(view.failures) : null;

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
  const retryNow = useCallback(() => {
    setView((current) => (current.kind === "offline" ? { ...current, trying: true } : current));
  }, []);

  // Toda tentativa de conexão passa por aqui: abrir o app, login e reconexão.
  useEffect(() => {
    if (!trying) return;
    const token = loadToken();
    if (!token) {
      setView({ kind: "login" });
      return;
    }

    const server = loadServer();
    const controller = new AbortController();
    const onDropped = () => {
      session.current = null;
      setView((current) =>
        current.kind === "online"
          ? { kind: "offline", trying: true, failures: 0, chat: current.chat }
          : current,
      );
    };
    connectSession(server, token, onDropped, controller.signal)
      .then((opened) => {
        session.current = opened;
        setView({ kind: "online", chat: { session: opened, server, token } });
      })
      .catch((err: unknown) => {
        if (controller.signal.aborted) return;
        if (err instanceof SessionError && err.kind === "rejected") {
          endSession(SESSION_EXPIRED);
        } else {
          setView((current) =>
            current.kind === "offline"
              ? { ...current, trying: false, failures: current.failures + 1 }
              : current,
          );
        }
      });
    return () => controller.abort();
  }, [trying, endSession]);

  // A espera entre uma tentativa que falhou e a próxima.
  useEffect(() => {
    if (waitMs === null) return;
    const timer = setTimeout(retryNow, waitMs);
    return () => clearTimeout(timer);
  }, [waitMs, retryNow]);

  // Fechar o app derruba a conexão aberta.
  useEffect(() => () => session.current?.close(), []);

  const chat = view.kind === "online" || view.kind === "offline" ? view.chat : undefined;

  return (
    <main className="container">
      {view.kind === "login" && (
        <Login
          initialUsername={view.username}
          notice={view.notice}
          onLoggedIn={(token) => {
            saveToken(token);
            setView(CONNECTING);
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
      {chat && (
        <ChatLayout
          server={chat.server}
          token={chat.token}
          session={chat.session}
          connectionLost={
            view.kind === "offline" ? { retrying: view.trying, onRetry: retryNow } : undefined
          }
          onLogout={logout}
          onSessionExpired={expireSession}
        />
      )}
      {!chat && view.kind === "offline" && view.failures === 0 && (
        <p role="status">Conectando...</p>
      )}
      {!chat && view.kind === "offline" && view.failures > 0 && (
        <Surface className="auth-form">
          <Alert>Não foi possível conectar ao servidor.</Alert>
          <p>O Orbit continua tentando sozinho.</p>
          <Button variant="primary" disabled={view.trying} onClick={retryNow}>
            Tentar de novo
          </Button>
          <Button variant="link" onClick={logout}>
            Sair
          </Button>
        </Surface>
      )}
      <p className="version">Orbit v{version}</p>
    </main>
  );
}

export default App;
