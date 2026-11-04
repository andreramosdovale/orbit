import type { ComponentProps } from "react";
import { cx } from "./cx";
import "./NavItem.css";

/** Item de uma lista de navegação. O atual é marcado com `aria-current`. */
export function NavItem({ className, ...props }: ComponentProps<"button">) {
  return <button type="button" className={cx("ds-nav-item", className)} {...props} />;
}
