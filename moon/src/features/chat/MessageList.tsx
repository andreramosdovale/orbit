import { useLayoutEffect, useRef } from "react";
import type { ChatMessage } from "../../bindings";

interface MessageListProps {
  messages: ChatMessage[];
  currentUserId: number;
  /** Primeira página a caminho. */
  loading: boolean;
  error: string | null;
  /** Existem mensagens mais antigas no servidor. */
  hasMore: boolean;
  loadingOlder: boolean;
  onLoadOlder: () => void;
  onRetry: () => void;
}

/** Distância das pontas da lista, em px, em que o usuário ainda conta como "lá". */
const NEAR_PX = 80;

/** `03/11/2026 14:32`, no horário local. */
export function formatDateTime(createdAt: string): string {
  const date = new Date(createdAt);
  const day = date.toLocaleDateString("pt-BR", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  });
  const time = date.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
  return `${day} ${time}`;
}

/**
 * Conversa de um canal. Chegar perto do topo carrega as mensagens anteriores, e o botão
 * faz o mesmo quando não há o que rolar. Quem está lendo o histórico não é levado para
 * o fim por uma mensagem nova, a não ser que seja dele.
 */
export function MessageList({
  messages,
  currentUserId,
  loading,
  error,
  hasMore,
  loadingOlder,
  onLoadOlder,
  onRetry,
}: MessageListProps) {
  const list = useRef<HTMLDivElement>(null);
  const shown = useRef<{ firstId?: number; lastId?: number } | null>(null);
  const first = messages[0];
  const last = messages[messages.length - 1];

  // Medidas de antes desta atualização: o DOM só muda quando ela for aplicada.
  const element = list.current;
  const before = element && {
    scrollTop: element.scrollTop,
    scrollHeight: element.scrollHeight,
    nearEnd: element.scrollHeight - element.scrollTop - element.clientHeight <= NEAR_PX,
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: só as pontas da lista são gatilho; as medidas são as desta renderização.
  useLayoutEffect(() => {
    const current = list.current;
    if (!current) return;
    const previous = shown.current;
    shown.current = { firstId: first?.id, lastId: last?.id };

    if (!previous || !before) {
      current.scrollTop = current.scrollHeight;
      return;
    }
    const newAtEnd = last?.id !== previous.lastId;
    const olderAtStart = first?.id !== previous.firstId;
    if (newAtEnd && (before.nearEnd || last?.author.id === currentUserId)) {
      current.scrollTop = current.scrollHeight;
    } else if (olderAtStart) {
      // O que entrou acima empurra a lista: compensa para o que está na tela não sair do lugar.
      current.scrollTop = before.scrollTop + current.scrollHeight - before.scrollHeight;
    }
  }, [first?.id, last?.id]);

  function onScroll() {
    const current = list.current;
    if (!current || current.scrollTop > NEAR_PX) return;
    // Depois de um erro, só o botão tenta de novo: rolar não pode repetir a chamada.
    if (hasMore && !loadingOlder && !error) onLoadOlder();
  }

  if (messages.length === 0 && !error) {
    return (
      <p className="no-messages">
        {loading ? "Carregando mensagens..." : "Nenhuma mensagem ainda."}
      </p>
    );
  }

  return (
    <div className="messages" ref={list} onScroll={onScroll}>
      {error && (
        <div className="messages-error">
          <p className="error" role="alert">
            {error}
          </p>
          <button type="button" onClick={onRetry}>
            Tentar de novo
          </button>
        </div>
      )}
      {!error && hasMore && (
        <button type="button" className="load-older" disabled={loadingOlder} onClick={onLoadOlder}>
          {loadingOlder ? "Carregando..." : "Carregar mensagens anteriores"}
        </button>
      )}
      <ul>
        {messages.map((message) => (
          <li key={message.id} className="message">
            <header>
              <strong>{message.author.username}</strong>{" "}
              <time dateTime={message.created_at}>{formatDateTime(message.created_at)}</time>
            </header>
            <p>{message.content}</p>
          </li>
        ))}
      </ul>
    </div>
  );
}
