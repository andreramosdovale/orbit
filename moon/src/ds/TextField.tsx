import { type ComponentProps, type ReactNode, useId } from "react";
import { cx } from "./cx";
import "./field.css";

export interface TextFieldProps extends ComponentProps<"input"> {
  label: ReactNode;
  hint?: ReactNode;
}

export function TextField({ label, hint, className, ...props }: TextFieldProps) {
  const hintId = useId();
  return (
    <label className="ds-field">
      <span className="ds-field-label">{label}</span>
      <input
        className={cx("ds-input", className)}
        aria-describedby={hint ? hintId : undefined}
        {...props}
      />
      {hint && (
        <span id={hintId} className="ds-field-hint">
          {hint}
        </span>
      )}
    </label>
  );
}
