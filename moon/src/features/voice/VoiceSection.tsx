import { useMutation, useQuery } from "@tanstack/react-query";
import { type FormEvent, useEffect, useState } from "react";
import { ApiError } from "../../api/http";
import type { VoiceChannel } from "../../bindings";
import { Alert, Button, NavItem, SectionTitle, TextField } from "../../ds";
import { createVoiceChannel, listVoiceChannels } from "./api";
import type { VoiceControls } from "./useVoiceRoom";
import { VoicePanel } from "./VoicePanel";

interface VoiceSectionProps {
  server: string;
  token: string;
  /** Salas anunciadas pelo WebSocket nesta tela. */
  announced: VoiceChannel[];
  /** A sala atual: fica no `ChatLayout`, porque a área principal mostra a transmissão. */
  voice: VoiceControls;
  onSessionExpired: () => void;
}

function isUnauthorized(err: unknown): boolean {
  return err instanceof ApiError && err.code === "unauthorized";
}

function errorMessage(err: unknown): string {
  return err instanceof ApiError ? err.message : "Erro inesperado.";
}

/** Acrescenta as salas que ainda não estão na lista. */
function mergeChannels(channels: VoiceChannel[], extra: VoiceChannel[]): VoiceChannel[] {
  const known = new Set(channels.map((c) => c.id));
  return [...channels, ...extra.filter((c) => !known.has(c.id))];
}

/** Salas de voz na lateral e a sala atual. Continua aberta ao trocar de canal de texto. */
export function VoiceSection({
  server,
  token,
  announced,
  voice,
  onSessionExpired,
}: VoiceSectionProps) {
  const channelsQuery = useQuery({
    queryKey: ["voice-channels"],
    queryFn: () => listVoiceChannels(server, token),
  });
  const [created, setCreated] = useState<VoiceChannel[]>([]);
  const [newName, setNewName] = useState("");
  const { state, sharing, join, leave, toggleMute, toggleShare, watch } = voice;

  const create = useMutation({
    mutationFn: (name: string) => createVoiceChannel(server, token, { name }),
    onSuccess: (channel) => {
      setCreated((current) => mergeChannels(current, [channel]));
      setNewName("");
    },
    onError: (err) => {
      if (isUnauthorized(err)) onSessionExpired();
    },
  });

  useEffect(() => {
    if (isUnauthorized(channelsQuery.error)) onSessionExpired();
  }, [channelsQuery.error, onSessionExpired]);

  function onCreate(event: FormEvent) {
    event.preventDefault();
    create.mutate(newName);
  }

  const channels = mergeChannels(mergeChannels(channelsQuery.data ?? [], created), announced);
  const current = channels.find((c) => c.id === state.channelId);
  const createError = create.error && !isUnauthorized(create.error) ? create.error : null;

  return (
    <section className="sidebar-section" aria-label="Salas de voz">
      <SectionTitle>Salas de voz</SectionTitle>
      {channelsQuery.error && !channelsQuery.data && !channelsQuery.isFetching && (
        <>
          <Alert>{errorMessage(channelsQuery.error)}</Alert>
          <Button size="sm" onClick={() => channelsQuery.refetch()}>
            Recarregar salas
          </Button>
        </>
      )}
      <ul className="voice-channels">
        {channels.map((channel) => (
          <li key={channel.id}>
            <NavItem
              aria-label={`Entrar na sala ${channel.name}`}
              aria-current={channel.id === state.channelId ? "true" : undefined}
              onClick={() => {
                if (channel.id !== state.channelId) join(channel.id);
              }}
            >
              🔊 {channel.name}
            </NavItem>
          </li>
        ))}
      </ul>

      {state.error && <Alert>{state.error}</Alert>}
      {state.channelId !== null && (
        <VoicePanel
          channelName={current?.name ?? ""}
          state={state}
          sharing={sharing}
          onToggleMute={toggleMute}
          onToggleShare={toggleShare}
          onWatch={watch}
          onLeave={leave}
        />
      )}

      <form className="new-channel" onSubmit={onCreate}>
        <TextField
          label="Nova sala"
          required
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
        />
        <Button type="submit" disabled={create.isPending}>
          {create.isPending ? "Criando..." : "Criar sala"}
        </Button>
        {createError && <Alert>{errorMessage(createError)}</Alert>}
      </form>
    </section>
  );
}
