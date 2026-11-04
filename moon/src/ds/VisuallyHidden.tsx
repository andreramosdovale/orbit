import type { ComponentProps } from "react";
import "./VisuallyHidden.css";

/** Texto só para leitores de tela. */
export function VisuallyHidden(props: ComponentProps<"span">) {
  return <span className="ds-visually-hidden" {...props} />;
}
