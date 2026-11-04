import type { ReactNode } from "react";
import { cx } from "./cx";
import "./Alert.css";

export interface AlertProps {
  tone?: "danger" | "success" | "info";
  /** Botão ao lado da mensagem, como "Tentar agora". */
  action?: ReactNode;
  className?: string;
  children: ReactNode;
}

/** Um erro é anunciado na hora (`alert`); um aviso, quando der (`status`). */
export function Alert({ tone = "danger", action, className, children }: AlertProps) {
  return (
    <div
      role={tone === "danger" ? "alert" : "status"}
      className={cx("ds-alert", `ds-alert-${tone}`, className)}
    >
      <span>{children}</span>
      {action}
    </div>
  );
}
