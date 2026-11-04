import { type FormEvent, useEffect, useState } from "react";
import { ApiError } from "../../api/http";
import type { TextChannel, User } from "../../bindings";
import { createChannel, listChannels } from "./api";

interface ChatLayoutProps {
  server: string;
  token: string;
  user: User;
  onLogout: () => void;
  /** O servidor recusou o token em uma chamada REST. */
  onSessionExpired: () => void;
}

type Channels =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "ready"; channels: TextChannel[]; selectedId: number | null };

function errorMessage(err: unknown): string {
  return err instanceof ApiError ? err.message : "Erro inesperado.";
}

/** Tela depois de entrar: canais na lateral, canal selecionado no centro. */
export function ChatLayout({ server, token, user, onLogout, onSessionExpired }: ChatLayoutProps) {
  const [state, setState] = useState<Channels>({ kind: "loading" });
  const [newName, setNewName] = useState("");
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const loading = state.kind === "loading";

  useEffect(() => {
    if (!loading) return;
    let cancelled = false;
    listChannels(server, token)
      .then((channels) => {
        if (cancelled) return;
        setState({ kind: "ready", channels, selectedId: channels[0]?.id ?? null });
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        if (err instanceof ApiError && err.code === "unauthorized") {
          onSessionExpired();
        } else {
          setState({ kind: "error", message: errorMessage(err) });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [loading, server, token, onSessionExpired]);

  async function onCreate(event: FormEvent) {
    event.preventDefault();
    setCreating(true);
    setCreateError(null);
    try {
      const channel = await createChannel(server, token, { name: newName });
      setState((current) =>
        current.kind === "ready"
          ? { kind: "ready", channels: [...current.channels, channel], selectedId: channel.id }
          : current,
      );
      setNewName("");
    } catch (err) {
      if (err instanceof ApiError && err.code === "unauthorized") {
        onSessionExpired();
        return;
      }
      setCreateError(errorMessage(err));
    } finally {
      setCreating(false);
    }
  }

  const ready = state.kind === "ready" ? state : null;
  const selected = ready?.channels.find((c) => c.id === ready.selectedId);

  return (
    <div className="chat-layout">
      <aside className="chat-sidebar">
        <p role="status">
          Conectado como <strong>{user.username}</strong>
        </p>

        {state.kind === "loading" && <p>Carregando canais...</p>}
        {state.kind === "error" && (
          <>
            <p className="error" role="alert">
              {state.message}
            </p>
            <button type="button" onClick={() => setState({ kind: "loading" })}>
              Tentar de novo
            </button>
          </>
        )}
        {ready && (
          <>
            <ul className="channels">
              {ready.channels.map((channel) => (
                <li key={channel.id}>
                  <button
                    type="button"
                    className="channel"
                    aria-current={channel.id === ready.selectedId ? "true" : undefined}
                    onClick={() => setState({ ...ready, selectedId: channel.id })}
                  >
                    # {channel.name}
                  </button>
                </li>
              ))}
            </ul>

            <form className="new-channel" onSubmit={onCreate}>
              <label>
                Novo canal
                <input required value={newName} onChange={(e) => setNewName(e.target.value)} />
              </label>
              <button type="submit" disabled={creating}>
                {creating ? "Criando..." : "Criar"}
              </button>
              {createError && (
                <p className="error" role="alert">
                  {createError}
                </p>
              )}
            </form>
          </>
        )}

        <button type="button" className="logout" onClick={onLogout}>
          Sair
        </button>
      </aside>

      <section className="chat-main">
        {selected && (
          <>
            <h2># {selected.name}</h2>
            <p>As mensagens chegam em breve.</p>
          </>
        )}
      </section>
    </div>
  );
}
