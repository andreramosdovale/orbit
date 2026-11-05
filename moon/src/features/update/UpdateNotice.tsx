import { useEffect, useState } from "react";
import { commands } from "../../bindings";
import { Alert, Button } from "../../ds";

type Install = { kind: "idle" } | { kind: "installing" } | { kind: "failed"; message: string };

/** Avisa quando há uma versão nova. O app fecha sozinho para instalar (ADR 0019). */
export function UpdateNotice() {
  const [version, setVersion] = useState<string | null>(null);
  const [install, setInstall] = useState<Install>({ kind: "idle" });

  useEffect(() => {
    let active = true;
    commands
      .checkForUpdate()
      .then((result) => {
        if (result.status === "error") {
          console.warn(result.error);
        } else if (active && result.data) {
          setVersion(result.data.version);
        }
      })
      .catch((err: unknown) => console.warn(err));
    return () => {
      active = false;
    };
  }, []);

  async function update() {
    setInstall({ kind: "installing" });
    try {
      const result = await commands.installUpdate();
      if (result.status === "error") setInstall({ kind: "failed", message: result.error });
    } catch (err) {
      setInstall({ kind: "failed", message: String(err) });
    }
  }

  if (!version) return null;
  if (install.kind === "installing") {
    return (
      <Alert tone="info" className="update-notice">
        Atualizando…
      </Alert>
    );
  }

  const failed = install.kind === "failed";
  return (
    <Alert
      tone={failed ? "danger" : "info"}
      className="update-notice"
      action={
        <Button variant="primary" size="sm" onClick={update}>
          Atualizar
        </Button>
      }
    >
      {failed ? install.message : `Versão ${version} disponível`}
    </Alert>
  );
}
