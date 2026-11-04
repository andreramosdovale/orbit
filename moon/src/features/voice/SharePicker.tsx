import type { CaptureTarget } from "../../bindings";
import { Button, NavItem, SectionTitle, Surface } from "../../ds";

interface SharePickerProps {
  targets: CaptureTarget[];
  onPick: (target: CaptureTarget) => void;
  onCancel: () => void;
}

type WindowTarget = Extract<CaptureTarget, { kind: "window" }>;
type ScreenTarget = Extract<CaptureTarget, { kind: "screen" }>;

/** O que transmitir: uma janela, com o áudio do aplicativo, ou uma tela, sem áudio. */
export function SharePicker({ targets, onPick, onCancel }: SharePickerProps) {
  const windows = targets.filter((t): t is WindowTarget => t.kind === "window");
  const screens = targets.filter((t): t is ScreenTarget => t.kind === "screen");

  return (
    <Surface as="section" className="share-picker" aria-label="Escolher o que transmitir">
      <div className="share-picker-section">
        <SectionTitle>Janelas, com o áudio do aplicativo</SectionTitle>
        {windows.length === 0 ? (
          <p className="share-picker-empty">Nenhuma janela disponível.</p>
        ) : (
          <ul className="share-picker-list">
            {windows.map((target) => (
              <li key={target.id}>
                <NavItem onClick={() => onPick(target)}>
                  {target.title}
                  {target.app && <span className="share-picker-app"> · {target.app}</span>}
                </NavItem>
              </li>
            ))}
          </ul>
        )}
      </div>
      <div className="share-picker-section">
        <SectionTitle>Telas, sem áudio</SectionTitle>
        <ul className="share-picker-list">
          {screens.map((target) => (
            <li key={target.index}>
              <NavItem onClick={() => onPick(target)}>{target.name}</NavItem>
            </li>
          ))}
        </ul>
      </div>
      <Button size="sm" className="share-picker-cancel" onClick={onCancel}>
        Cancelar
      </Button>
    </Surface>
  );
}
