import { useEffect, useRef } from "react";
import type { ChatMessage } from "../../bindings";

interface MessageListProps {
  messages: ChatMessage[];
}

function formatTime(createdAt: string): string {
  return new Date(createdAt).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
}

/** Conversa de um canal. Rola até o fim quando chega mensagem. */
export function MessageList({ messages }: MessageListProps) {
  const list = useRef<HTMLUListElement>(null);
  const count = messages.length;

  // biome-ignore lint/correctness/useExhaustiveDependencies: `count` é o gatilho para rolar.
  useEffect(() => {
    if (list.current) list.current.scrollTop = list.current.scrollHeight;
  }, [count]);

  if (count === 0) {
    return <p className="no-messages">Nenhuma mensagem ainda.</p>;
  }

  return (
    <ul className="messages" ref={list}>
      {messages.map((message) => (
        <li key={message.id} className="message">
          <header>
            <strong>{message.author.username}</strong>{" "}
            <time dateTime={message.created_at}>{formatTime(message.created_at)}</time>
          </header>
          <p>{message.content}</p>
        </li>
      ))}
    </ul>
  );
}
