import { type FormEvent, useEffect, useRef, useState } from "react";
import { ApiError } from "../../api/http";
import type { ChatMessage, MessageRejectReason, ServerMessage, TextChannel } from "../../bindings";
import type { Session } from "../auth/session";
import { createChannel, listChannels } from "./api";
import { Composer } from "./Composer";
import { MessageList } from "./MessageList";

interface ChatLayoutProps {
  server: string;
  token: string;
  session: Session;
  onLogout: () => void;
  /** O servidor recusou o token em uma chamada REST. */
  onSessionExpired: () => void;
}

type Channels =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "ready"; channels: TextChannel[]; selectedId: number | null };

const REJECT_MESSAGES: Record<MessageRejectReason, string> = {
  invalid_content: "A mensagem deve ter de 1 a 2000 caracteres.",
  unknown_channel: "Esse canal não existe mais.",
  internal: "Não foi possível enviar a mensagem. Tente novamente.",
};

function errorMessage(err: unknown): string {
  return err instanceof ApiError ? err.message : "Erro inesperado.";
}

/** Acrescenta os canais que ainda não estão na lista. */
function mergeChannels(channels: TextChannel[], extra: TextChannel[]): TextChannel[] {
  const known = new Set(channels.map((c) => c.id));
  return [...channels, ...extra.filter((c) => !known.has(c.id))];
}

/** Tela depois de entrar: canais na lateral, conversa do canal selecionado no centro. */
export function ChatLayout({
  server,
  token,
  session,
  onLogout,
  onSessionExpired,
}: ChatLayoutProps) {
  const [state, setState] = useState<Channels>({ kind: "loading" });
  const [messages, setMessages] = useState<Record<number, ChatMessage[]>>({});
  const [newName, setNewName] = useState("");
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const [sendError, setSendError] = useState<string | null>(null);
  // Canais anunciados enquanto a lista carrega: a resposta pode já não incluí-los.
  const announced = useRef<TextChannel[]>([]);
  const loading = state.kind === "loading";

  useEffect(() => {
    if (!loading) return;
    let cancelled = false;
    listChannels(server, token)
      .then((channels) => {
        if (cancelled) return;
        const merged = mergeChannels(channels, announced.current);
        setState({ kind: "ready", channels: merged, selectedId: merged[0]?.id ?? null });
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

  useEffect(() => {
    const onServerMessage = (msg: ServerMessage) => {
      switch (msg.type) {
        case "new_message": {
          const { message } = msg;
          setMessages((current) => ({
            ...current,
            [message.channel_id]: [...(current[message.channel_id] ?? []), message],
          }));
          break;
        }
        case "channel_created":
          announced.current = mergeChannels(announced.current, [msg.channel]);
          setState((current) =>
            current.kind === "ready"
              ? {
                  ...current,
                  channels: mergeChannels(current.channels, [msg.channel]),
                  selectedId: current.selectedId ?? msg.channel.id,
                }
              : current,
          );
          break;
        case "message_rejected":
          setSendError(REJECT_MESSAGES[msg.reason]);
          break;
        default:
          break;
      }
    };
    return session.subscribe(onServerMessage);
  }, [session]);

  async function onCreate(event: FormEvent) {
    event.preventDefault();
    setCreating(true);
    setCreateError(null);
    try {
      const channel = await createChannel(server, token, { name: newName });
      setState((current) =>
        current.kind === "ready"
          ? {
              kind: "ready",
              channels: mergeChannels(current.channels, [channel]),
              selectedId: channel.id,
            }
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

  function selectChannel(channelId: number) {
    if (!ready) return;
    setSendError(null);
    setState({ ...ready, selectedId: channelId });
  }

  return (
    <div className="chat-layout">
      <aside className="chat-sidebar">
        <p role="status">
          Conectado como <strong>{session.user.username}</strong>
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
                    onClick={() => selectChannel(channel.id)}
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
            <MessageList messages={messages[selected.id] ?? []} />
            <Composer
              key={selected.id}
              channelName={selected.name}
              error={sendError}
              onSend={(content) => {
                setSendError(null);
                session.send({ type: "send_message", channel_id: selected.id, content });
              }}
            />
          </>
        )}
      </section>
    </div>
  );
}
