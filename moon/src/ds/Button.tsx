import type { ComponentProps } from "react";
import { cx } from "./cx";
import "./Button.css";

export interface ButtonProps extends ComponentProps<"button"> {
  variant?: "primary" | "secondary" | "ghost" | "link";
  size?: "md" | "sm";
}

export function Button({
  variant = "secondary",
  size = "md",
  type = "button",
  className,
  ...props
}: ButtonProps) {
  return (
    <button
      type={type}
      className={cx(
        "ds-button",
        `ds-button-${variant}`,
        size === "sm" && "ds-button-sm",
        className,
      )}
      {...props}
    />
  );
}
