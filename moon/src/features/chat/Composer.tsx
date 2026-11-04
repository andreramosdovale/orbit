import { type KeyboardEvent, useState } from "react";

/** O mesmo limite do `sun`. Conta caracteres, não unidades UTF-16. */
const MAX_CONTENT_CHARS = 2000;

interface ComposerProps {
  channelName: string;
  /** Erro da última mensagem enviada. */
  error: string | null;
  /** Conexão caída: o campo continua editável, mas Enter não envia nem apaga o texto. */
  offline?: boolean;
  onSend: (content: string) => void;
}

/** Campo de mensagem: Enter envia, Shift+Enter quebra a linha. */
export function Composer({ channelName, error, offline = false, onSend }: ComposerProps) {
  const [text, setText] = useState("");
  const [tooLong, setTooLong] = useState(false);

  function onKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) return;
    event.preventDefault();
    if (offline) return;

    const content = text.trim();
    if (content === "") return;
    if ([...content].length > MAX_CONTENT_CHARS) {
      setTooLong(true);
      return;
    }
    setTooLong(false);
    setText("");
    onSend(content);
  }

  return (
    <div className="composer">
      <textarea
        aria-label={`Mensagem para #${channelName}`}
        placeholder={`Mensagem para #${channelName}`}
        rows={2}
        value={text}
        onChange={(e) => {
          setText(e.target.value);
          setTooLong(false);
        }}
        onKeyDown={onKeyDown}
      />
      {tooLong && (
        <p className="error" role="alert">
          A mensagem passa do limite de {MAX_CONTENT_CHARS} caracteres.
        </p>
      )}
      {!tooLong && error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
