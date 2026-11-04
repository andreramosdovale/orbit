import type { User, UserPresence } from "../../bindings";

interface MemberListProps {
  /** `null` enquanto o `presence_snapshot` não chega. */
  members: UserPresence[] | null;
}

/** Atualiza o estado de `user`, acrescentando quem ainda não está na lista. */
export function withPresence(members: UserPresence[], user: User, online: boolean): UserPresence[] {
  const others = members.filter((member) => member.user.id !== user.id);
  return [...others, { user, online }];
}

function byName(a: UserPresence, b: UserPresence): number {
  return a.user.username.localeCompare(b.user.username, "pt-BR");
}

function Section({ title, members }: { title: string; members: UserPresence[] }) {
  if (members.length === 0) return null;
  return (
    <section>
      <h3>
        {title} — {members.length}
      </h3>
      <ul>
        {members.map(({ user, online }) => (
          <li key={user.id} className={online ? "member online" : "member offline"}>
            <span className="presence-dot" aria-hidden="true" />
            {user.username}
            <span className="visually-hidden"> ({online ? "online" : "offline"})</span>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** Quem do grupo está online e quem está offline. */
export function MemberList({ members }: MemberListProps) {
  return (
    <aside className="member-list" aria-label="Pessoas">
      {members === null ? (
        <p>Carregando...</p>
      ) : (
        <>
          <Section title="Online" members={members.filter((m) => m.online).sort(byName)} />
          <Section title="Offline" members={members.filter((m) => !m.online).sort(byName)} />
        </>
      )}
    </aside>
  );
}
