import type {
  ApiErrorBody,
  ApiErrorCode,
  LoginRequest,
  LoginResponse,
  RegisterRequest,
  User,
} from "../../bindings";

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

/** `POST /auth/register`. Resolve com o usuário criado ou rejeita com `ApiError`. */
export function register(server: string, req: RegisterRequest): Promise<User> {
  return postJson<User>(server, "/auth/register", req);
}

/** `POST /auth/login`. Resolve com o token e o usuário ou rejeita com `ApiError`. */
export function login(server: string, req: LoginRequest): Promise<LoginResponse> {
  return postJson<LoginResponse>(server, "/auth/login", req);
}

async function postJson<T>(server: string, path: string, body: unknown): Promise<T> {
  let res: Response;
  try {
    res = await fetch(`${normalizeServerUrl(server)}${path}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
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
