import type { ComponentProps } from "react";
import { cx } from "./cx";
import "./field.css";

export function TextArea({ className, ...props }: ComponentProps<"textarea">) {
  return <textarea className={cx("ds-input", className)} {...props} />;
}
