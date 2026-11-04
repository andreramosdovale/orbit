import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ChatMessage } from "../../bindings";
import { MessageList } from "./MessageList";

const ME = 1;
const OTHER = 2;

function message(id: number, authorId = OTHER): ChatMessage {
  return {
    id,
    channel_id: 1,
    author: { id: authorId, username: authorId === ME ? "fulano" : "ciclano" },
    content: `msg ${id}`,
    created_at: "2026-11-03T12:00:00.000Z",
  };
}

function range(from: number, to: number): ChatMessage[] {
  return Array.from({ length: to - from + 1 }, (_, i) => message(from + i));
}

/**
 * O jsdom não faz layout: as medidas de rolagem são simuladas. A altura depende do que
 * está no DOM, então só muda quando a atualização é aplicada, como em um navegador.
 */
const ITEM_HEIGHT = 40;
const CLIENT_HEIGHT = 500;

function setup(messages: ChatMessage[], overrides = {}) {
  const onLoadOlder = vi.fn();
  const onRetry = vi.fn();
  const props = {
    currentUserId: ME,
    loading: false,
    error: null,
    hasMore: true,
    loadingOlder: false,
    onLoadOlder,
    onRetry,
    ...overrides,
  };
  const view = render(<MessageList messages={messages} {...props} />);
  const update = (next: ChatMessage[], nextOverrides = {}) =>
    view.rerender(<MessageList messages={next} {...props} {...nextOverrides} />);
  const list = view.container.querySelector(".messages") as HTMLElement;
  return { list, update, onLoadOlder, onRetry };
}

describe("MessageList", () => {
  beforeEach(() => {
    vi.spyOn(HTMLElement.prototype, "scrollHeight", "get").mockImplementation(function (
      this: HTMLElement,
    ) {
      return this.querySelectorAll("li").length * ITEM_HEIGHT;
    });
    vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(CLIENT_HEIGHT);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("mostra autor, dia e horário de cada mensagem", () => {
    const createdAt = new Date(2026, 10, 3, 14, 32).toISOString();
    setup([{ ...message(1), created_at: createdAt }]);

    expect(screen.getByText("ciclano")).toBeTruthy();
    expect(screen.getByText("03/11/2026 14:32")).toBeTruthy();
    expect(screen.getByText("msg 1")).toBeTruthy();
  });

  it("abre rolado até o fim", () => {
    const { list } = setup(range(1, 50));

    expect(list.scrollTop).toBe(2000);
  });

  it("mensagem nova rola até o fim quando se está perto do fim", () => {
    const { list, update } = setup(range(1, 50));
    list.scrollTop = 1500 - 40;

    update(range(1, 51));

    expect(list.scrollTop).toBe(2040);
  });

  it("mensagem nova de outra pessoa não rola quando se está lendo acima", () => {
    const { list, update } = setup(range(1, 50));
    list.scrollTop = 300;

    update(range(1, 51));

    expect(list.scrollTop).toBe(300);
  });

  it("mensagem nova do próprio usuário sempre rola", () => {
    const { list, update } = setup(range(1, 50));
    list.scrollTop = 300;

    update([...range(1, 50), message(51, ME)]);

    expect(list.scrollTop).toBe(2040);
  });

  it("carregar mensagens anteriores mantém a posição das mensagens na tela", () => {
    const { list, update } = setup(range(51, 100));
    list.scrollTop = 30;

    update(range(1, 100));

    expect(list.scrollTop).toBe(2030);
  });

  it("chegar perto do topo pede a página anterior", () => {
    const { list, onLoadOlder } = setup(range(1, 50));

    list.scrollTop = 80;
    fireEvent.scroll(list);

    expect(onLoadOlder).toHaveBeenCalledOnce();
  });

  it("longe do topo não pede a página anterior", () => {
    const { list, onLoadOlder } = setup(range(1, 50));

    list.scrollTop = 81;
    fireEvent.scroll(list);

    expect(onLoadOlder).not.toHaveBeenCalled();
  });

  it("não pede de novo durante um carregamento, depois de um erro nem sem mais páginas", () => {
    for (const overrides of [{ loadingOlder: true }, { error: "falhou" }, { hasMore: false }]) {
      const { list, onLoadOlder } = setup(range(1, 50), overrides);

      list.scrollTop = 0;
      fireEvent.scroll(list);

      expect(onLoadOlder).not.toHaveBeenCalled();
    }
  });

  it("o botão pede a página anterior e fica desativado durante o carregamento", () => {
    const { update, onLoadOlder } = setup(range(1, 50));

    fireEvent.click(screen.getByRole("button", { name: "Carregar mensagens anteriores" }));
    expect(onLoadOlder).toHaveBeenCalledOnce();

    update(range(1, 50), { loadingOlder: true });
    expect(
      (screen.getByRole("button", { name: "Carregando..." }) as HTMLButtonElement).disabled,
    ).toBe(true);
  });

  it("sem mais páginas o botão não aparece", () => {
    setup(range(1, 50), { hasMore: false });

    expect(screen.queryByRole("button")).toBeNull();
  });

  it("mostra o carregamento e, depois dele, o aviso de canal vazio", () => {
    const { update } = setup([], { loading: true, hasMore: false });
    expect(screen.getByText("Carregando mensagens...")).toBeTruthy();

    update([], { loading: false, hasMore: false });
    expect(screen.getByText("Nenhuma mensagem ainda.")).toBeTruthy();
  });

  it("um erro mostra a mensagem e Tentar de novo", () => {
    const { onRetry } = setup([], { error: "Erro no servidor.", hasMore: false });

    expect(screen.getByRole("alert").textContent).toBe("Erro no servidor.");
    fireEvent.click(screen.getByRole("button", { name: "Tentar de novo" }));
    expect(onRetry).toHaveBeenCalledOnce();
  });
});
