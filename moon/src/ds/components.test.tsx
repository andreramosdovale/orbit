import { fireEvent, render, screen } from "@testing-library/react";
import { createRef } from "react";
import { describe, expect, it, vi } from "vitest";
import {
  Alert,
  Avatar,
  Button,
  NavItem,
  SectionTitle,
  StatusDot,
  Surface,
  TextArea,
  TextField,
  VisuallyHidden,
} from ".";

describe("Button", () => {
  it("é secundário e não envia formulário por padrão", () => {
    render(<Button>Ok</Button>);
    const button = screen.getByRole("button", { name: "Ok" }) as HTMLButtonElement;
    expect(button.type).toBe("button");
    expect(button.classList.contains("ds-button-secondary")).toBe(true);
  });

  it.each(["primary", "secondary", "ghost", "link"] as const)("aplica a variação %s", (variant) => {
    render(<Button variant={variant}>Ok</Button>);
    expect(screen.getByRole("button").classList.contains(`ds-button-${variant}`)).toBe(true);
  });

  it("repassa atributos, classe e ref", () => {
    const ref = createRef<HTMLButtonElement>();
    const onClick = vi.fn();
    render(
      <Button ref={ref} type="submit" size="sm" className="extra" aria-label="x" onClick={onClick}>
        Ok
      </Button>,
    );
    const button = screen.getByRole("button", { name: "x" }) as HTMLButtonElement;
    expect(ref.current).toBe(button);
    expect(button.type).toBe("submit");
    expect(button.classList.contains("extra")).toBe(true);
    expect(button.classList.contains("ds-button-sm")).toBe(true);
    fireEvent.click(button);
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("desativado não dispara o clique", () => {
    const onClick = vi.fn();
    render(
      <Button disabled onClick={onClick}>
        Ok
      </Button>,
    );
    fireEvent.click(screen.getByRole("button"));
    expect(onClick).not.toHaveBeenCalled();
  });
});

describe("NavItem", () => {
  it("expõe o item atual e repassa a ref", () => {
    const ref = createRef<HTMLButtonElement>();
    render(
      <NavItem ref={ref} aria-current="true">
        # geral
      </NavItem>,
    );
    const item = screen.getByRole("button", { name: "# geral" }) as HTMLButtonElement;
    expect(item.getAttribute("aria-current")).toBe("true");
    expect(item.type).toBe("button");
    expect(ref.current).toBe(item);
  });
});

describe("TextField", () => {
  it("o rótulo nomeia o campo, e a dica o descreve", () => {
    render(<TextField label="Usuário" hint="Letras ou números" />);
    const input = screen.getByRole("textbox", { name: /Usuário/ });
    const hint = screen.getByText("Letras ou números");
    expect(input.getAttribute("aria-describedby")).toBe(hint.id);
  });

  it("sem dica, não descreve nada", () => {
    render(<TextField label="Usuário" />);
    expect(screen.getByRole("textbox").hasAttribute("aria-describedby")).toBe(false);
  });

  it("repassa atributos, estado desativado e ref", () => {
    const ref = createRef<HTMLInputElement>();
    render(<TextField ref={ref} label="Usuário" disabled required maxLength={5} />);
    const input = screen.getByRole("textbox") as HTMLInputElement;
    expect(ref.current).toBe(input);
    expect(input.disabled).toBe(true);
    expect(input.required).toBe(true);
    expect(input.maxLength).toBe(5);
  });
});

describe("TextArea", () => {
  it("repassa atributos, estado desativado e ref", () => {
    const ref = createRef<HTMLTextAreaElement>();
    render(<TextArea ref={ref} aria-label="Mensagem" disabled rows={2} />);
    const area = screen.getByRole("textbox", { name: "Mensagem" }) as HTMLTextAreaElement;
    expect(ref.current).toBe(area);
    expect(area.disabled).toBe(true);
    expect(area.rows).toBe(2);
  });
});

describe("Alert", () => {
  it("um erro usa o papel alert, e mostra a ação", () => {
    render(<Alert action={<Button>Tentar</Button>}>Caiu</Alert>);
    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain("Caiu");
    expect(alert.classList.contains("ds-alert-danger")).toBe(true);
    expect(screen.getByRole("button", { name: "Tentar" })).toBeTruthy();
  });

  it("uma informação neutra usa o papel status", () => {
    render(<Alert tone="info">Terminou</Alert>);
    expect(screen.getByRole("status").classList.contains("ds-alert-info")).toBe(true);
  });

  it("um aviso usa o papel status", () => {
    render(<Alert tone="success">Conta criada</Alert>);
    expect(screen.getByRole("status").classList.contains("ds-alert-success")).toBe(true);
  });
});

describe("Surface", () => {
  it("escolhe o elemento e a variação, e repassa atributos", () => {
    render(
      <Surface as="section" tone="overlay" aria-label="Painel">
        x
      </Surface>,
    );
    const surface = screen.getByRole("region", { name: "Painel" });
    expect(surface.tagName).toBe("SECTION");
    expect(surface.classList.contains("ds-surface-overlay")).toBe(true);
  });
});

describe("SectionTitle", () => {
  it("é um título de nível 3", () => {
    render(<SectionTitle>Online</SectionTitle>);
    expect(screen.getByRole("heading", { level: 3, name: "Online" })).toBeTruthy();
  });
});

describe("StatusDot", () => {
  it.each(["online", "offline"] as const)(
    "marca %s sem aparecer para leitores de tela",
    (status) => {
      const { container } = render(<StatusDot status={status} />);
      const dot = container.firstElementChild as HTMLElement;
      expect(dot.getAttribute("aria-hidden")).toBe("true");
      expect(dot.classList.contains(`ds-status-dot-${status}`)).toBe(true);
    },
  );
});

describe("Avatar", () => {
  it("mostra a inicial sem acrescentar texto à página", () => {
    const { container } = render(<Avatar name="ana" />);
    const avatar = container.firstElementChild as HTMLElement;
    expect(avatar.dataset.initial).toBe("A");
    expect(avatar.textContent).toBe("");
    expect(avatar.getAttribute("aria-hidden")).toBe("true");
  });

  it("não corta um emoji no meio", () => {
    const { container } = render(<Avatar name="😀ana" />);
    expect((container.firstElementChild as HTMLElement).dataset.initial).toBe("😀");
  });

  it("mostra o estado e o tamanho pedidos", () => {
    const { container } = render(<Avatar name="ana" size="sm" status="online" />);
    const avatar = container.firstElementChild as HTMLElement;
    expect(avatar.classList.contains("ds-avatar-sm")).toBe(true);
    expect(avatar.querySelector(".ds-status-dot-online")).toBeTruthy();
  });
});

describe("VisuallyHidden", () => {
  it("mantém o texto acessível", () => {
    render(<VisuallyHidden>(online)</VisuallyHidden>);
    expect(screen.getByText("(online)").classList.contains("ds-visually-hidden")).toBe(true);
  });
});
