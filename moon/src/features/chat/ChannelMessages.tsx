import { useInfiniteQuery } from "@tanstack/react-query";
import { useEffect, useMemo } from "react";
import { ApiError } from "../../api/http";
import type { ChatMessage, MessagePage } from "../../bindings";
import { listMessages } from "./api";
import { MessageList } from "./MessageList";

interface ChannelMessagesProps {
  server: string;
  token: string;
  channelId: number;
  currentUserId: number;
  /** O que chegou pelo WebSocket nesta conexão. */
  liveMessages: ChatMessage[];
  onSessionExpired: () => void;
}

/**
 * Junta as páginas do histórico e as mensagens do WebSocket, sem repetir e em ordem
 * crescente de `id`: uma mensagem pode chegar pelos dois caminhos durante uma busca.
 */
export function mergeMessages(pages: MessagePage[], live: ChatMessage[]): ChatMessage[] {
  const byId = new Map<number, ChatMessage>();
  for (const page of pages) {
    for (const message of page.messages) byId.set(message.id, message);
  }
  for (const message of live) byId.set(message.id, message);
  return [...byId.values()].sort((a, b) => a.id - b.id);
}

/** Conversa de um canal: histórico em páginas + o que chega ao vivo. */
export function ChannelMessages({
  server,
  token,
  channelId,
  currentUserId,
  liveMessages,
  onSessionExpired,
}: ChannelMessagesProps) {
  const query = useInfiniteQuery({
    queryKey: ["messages", channelId],
    queryFn: ({ pageParam }) => listMessages(server, token, channelId, pageParam),
    initialPageParam: undefined as number | undefined,
    // A página seguinte é a de mensagens mais antigas.
    getNextPageParam: (last) => (last.has_more ? last.messages[0]?.id : undefined),
  });
  const { error, isFetching } = query;

  useEffect(() => {
    if (error instanceof ApiError && error.code === "unauthorized") onSessionExpired();
  }, [error, onSessionExpired]);

  const pages = query.data?.pages;
  const messages = useMemo(() => mergeMessages(pages ?? [], liveMessages), [pages, liveMessages]);

  let errorText: string | null = null;
  if (error && !isFetching) {
    errorText = error instanceof ApiError ? error.message : "Erro inesperado.";
  }

  return (
    <MessageList
      messages={messages}
      currentUserId={currentUserId}
      loading={!pages && isFetching}
      error={errorText}
      hasMore={query.hasNextPage}
      loadingOlder={query.isFetchingNextPage}
      onLoadOlder={() => query.fetchNextPage()}
      onRetry={() => (query.isFetchNextPageError ? query.fetchNextPage() : query.refetch())}
    />
  );
}
