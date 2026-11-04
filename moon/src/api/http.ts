import type { ApiErrorBody, ApiErrorCode } from "../bindings";

/** Erro de uma chamada ao `sun`. `network` = não foi possível falar com o servidor. */
export class ApiError extends Error {
  readonly code: ApiErrorCode | "network";

  constructor(code: ApiErrorCode | "network", message: string) {
    super(message);
    this.name = "ApiError";
    this.code = code;
  }
}

/** Remove espaços e barras finais: `http://host:7700/` → `http://host:7700`. */
export function normalizeServerUrl(server: string): string {
  return server.trim().replace(/\/+$/, "");
}

interface RequestOptions {
  method: "GET" | "POST";
  /** Vai como JSON. */
  body?: unknown;
  /** JWT da sessão, para as rotas protegidas. */
  token?: string;
}

/** Chama o `sun`. Resolve com o JSON da resposta ou rejeita com `ApiError`. */
export async function request<T>(
  server: string,
  path: string,
  { method, body, token }: RequestOptions,
): Promise<T> {
  const headers: Record<string, string> = {};
  if (body !== undefined) headers["Content-Type"] = "application/json";
  if (token !== undefined) headers.Authorization = `Bearer ${token}`;

  let res: Response;
  try {
    res = await fetch(`${normalizeServerUrl(server)}${path}`, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  } catch {
    throw new ApiError("network", "Não foi possível conectar ao servidor.");
  }

  if (res.ok) {
    return (await res.json()) as T;
  }
  throw await toApiError(res);
}

async function toApiError(res: Response): Promise<ApiError> {
  try {
    const body = (await res.json()) as ApiErrorBody;
    return new ApiError(body.code, body.message);
  } catch {
    return new ApiError("internal", `O servidor respondeu com erro ${res.status}.`);
  }
}
