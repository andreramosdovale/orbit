import type { ComponentProps } from "react";
import { cx } from "./cx";
import "./SectionTitle.css";

export function SectionTitle({ className, ...props }: ComponentProps<"h3">) {
  return <h3 className={cx("ds-section-title", className)} {...props} />;
}
