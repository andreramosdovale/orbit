import { QueryClient, QueryClientProvider, useMutation, useQuery } from "@tanstack/react-query";
import { type FormEvent, useEffect, useState } from "react";
import { ApiError } from "../../api/http";
import type {
  ChatMessage,
  MessageRejectReason,
  ServerMessage,
  TextChannel,
  UserPresence,
} from "../../bindings";
import type { Session } from "../auth/session";
import { createChannel, listChannels } from "./api";
import { ChannelMessages } from "./ChannelMessages";
import { Composer } from "./Composer";
import { MemberList, withPresence } from "./MemberList";

interface ChatLayoutProps {
  server: string;
  token: string;
  session: Session;
  onLogout: () => void;
  /** O servidor recusou o token em uma chamada REST. */
  onSessionExpired: () => void;
}

const REJECT_MESSAGES: Record<MessageRejectReason, string> = {
  invalid_content: "A mensagem deve ter de 1 a 2000 caracteres.",
  unknown_channel: "Esse canal não existe mais.",
  internal: "Não foi possível enviar a mensagem. Tente novamente.",
};

const NO_MESSAGES: ChatMessage[] = [];

/**
 * Cache da conexão atual (ADR 0012). Os dados não envelhecem, porque o WebSocket
 * entrega o que há de novo, e um erro aparece na hora, com **Tentar de novo**.
 */
function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: Number.POSITIVE_INFINITY,
        gcTime: Number.POSITIVE_INFINITY,
        retry: false,
      },
      mutations: { retry: false },
    },
  });
}

function errorMessage(err: unknown): string {
  return err instanceof ApiError ? err.message : "Erro inesperado.";
}

function isUnauthorized(err: unknown): boolean {
  return err instanceof ApiError && err.code === "unauthorized";
}

/** Acrescenta os canais que ainda não estão na lista. */
function mergeChannels(channels: TextChannel[], extra: TextChannel[]): TextChannel[] {
  const known = new Set(channels.map((c) => c.id));
  return [...channels, ...extra.filter((c) => !known.has(c.id))];
}

/** Tela depois de entrar. Ela existe enquanto a conexão durar, e o cache junto com ela. */
export function ChatLayout(props: ChatLayoutProps) {
  const [queryClient] = useState(createQueryClient);
  return (
    <QueryClientProvider client={queryClient}>
      <ChatScreen {...props} />
    </QueryClientProvider>
  );
}

/** Canais na lateral, conversa do canal selecionado no centro. */
function ChatScreen({ server, token, session, onLogout, onSessionExpired }: ChatLayoutProps) {
  const channelsQuery = useQuery({
    queryKey: ["channels"],
    queryFn: () => listChannels(server, token),
  });
  // Criados ou anunciados nesta conexão: a resposta da lista pode já não incluí-los.
  const [extraChannels, setExtraChannels] = useState<TextChannel[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  // Só o que chegou pelo WebSocket. O histórico fica no cache (ADR 0012).
  const [liveMessages, setLiveMessages] = useState<Record<number, ChatMessage[]>>({});
  // Vem só do WebSocket, então também fica fora do cache.
  const [members, setMembers] = useState<UserPresence[] | null>(null);
  const [newName, setNewName] = useState("");
  const [sendError, setSendError] = useState<string | null>(null);

  const create = useMutation({
    mutationFn: (name: string) => createChannel(server, token, { name }),
    onSuccess: (channel) => {
      setExtraChannels((current) => mergeChannels(current, [channel]));
      setSelectedId(channel.id);
      setNewName("");
    },
    onError: (err) => {
      if (isUnauthorized(err)) onSessionExpired();
    },
  });

  useEffect(() => {
    if (isUnauthorized(channelsQuery.error)) onSessionExpired();
  }, [channelsQuery.error, onSessionExpired]);

  useEffect(() => {
    const onServerMessage = (msg: ServerMessage) => {
      switch (msg.type) {
        case "new_message": {
          const { message } = msg;
          setLiveMessages((current) => ({
            ...current,
            [message.channel_id]: [...(current[message.channel_id] ?? []), message],
          }));
          break;
        }
        case "channel_created":
          setExtraChannels((current) => mergeChannels(current, [msg.channel]));
          break;
        case "message_rejected":
          setSendError(REJECT_MESSAGES[msg.reason]);
          break;
        case "presence_snapshot":
          setMembers(msg.users);
          break;
        case "user_online":
        case "user_offline": {
          const online = msg.type === "user_online";
          setMembers((current) => current && withPresence(current, msg.user, online));
          break;
        }
        default:
          break;
      }
    };
    return session.subscribe(onServerMessage);
  }, [session]);

  function onCreate(event: FormEvent) {
    event.preventDefault();
    create.mutate(newName);
  }

  function selectChannel(channelId: number) {
    setSendError(null);
    setSelectedId(channelId);
  }

  const channels = channelsQuery.data ? mergeChannels(channelsQuery.data, extraChannels) : null;
  const selected = channels?.find((c) => c.id === selectedId) ?? channels?.[0];
  const createError = create.error && !isUnauthorized(create.error) ? create.error : null;

  return (
    <div className="chat-layout">
      <aside className="chat-sidebar">
        <p role="status">
          Conectado como <strong>{session.user.username}</strong>
        </p>

        {!channels && channelsQuery.isFetching && <p>Carregando canais...</p>}
        {!channels && !channelsQuery.isFetching && channelsQuery.error && (
          <>
            <p className="error" role="alert">
              {errorMessage(channelsQuery.error)}
            </p>
            <button type="button" onClick={() => channelsQuery.refetch()}>
              Tentar de novo
            </button>
          </>
        )}
        {channels && (
          <>
            <ul className="channels">
              {channels.map((channel) => (
                <li key={channel.id}>
                  <button
                    type="button"
                    className="channel"
                    aria-current={channel.id === selected?.id ? "true" : undefined}
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
              <button type="submit" disabled={create.isPending}>
                {create.isPending ? "Criando..." : "Criar"}
              </button>
              {createError && (
                <p className="error" role="alert">
                  {errorMessage(createError)}
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
            <ChannelMessages
              key={selected.id}
              server={server}
              token={token}
              channelId={selected.id}
              currentUserId={session.user.id}
              liveMessages={liveMessages[selected.id] ?? NO_MESSAGES}
              onSessionExpired={onSessionExpired}
            />
            <Composer
              key={`composer-${selected.id}`}
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

      <MemberList members={members} />
    </div>
  );
}
