import type { HTMLAttributes } from "react";
import { cx } from "./cx";
import "./Surface.css";

export interface SurfaceProps extends HTMLAttributes<HTMLElement> {
  as?: "div" | "section" | "aside" | "form";
  tone?: "raised" | "overlay";
}

export function Surface({ as: Tag = "div", tone = "raised", className, ...props }: SurfaceProps) {
  return <Tag className={cx("ds-surface", `ds-surface-${tone}`, className)} {...props} />;
}
