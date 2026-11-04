import { cx } from "./cx";
import { StatusDot } from "./StatusDot";
import "./Avatar.css";

export interface AvatarProps {
  name: string;
  size?: "md" | "sm";
  status?: "online" | "offline";
}

/** Inicial da pessoa em um círculo. Só decora: o nome precisa estar no texto ao lado. */
export function Avatar({ name, size = "md", status }: AvatarProps) {
  const initial = [...name][0]?.toLocaleUpperCase("pt-BR") ?? "";
  return (
    <span
      className={cx("ds-avatar", size === "sm" && "ds-avatar-sm")}
      data-initial={initial}
      aria-hidden="true"
    >
      {status && <StatusDot status={status} />}
    </span>
  );
}
