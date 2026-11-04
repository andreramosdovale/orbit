import { request } from "../../api/http";
import type { LoginRequest, LoginResponse, RegisterRequest, User } from "../../bindings";

/** `POST /auth/register`. Resolve com o usuário criado ou rejeita com `ApiError`. */
export function register(server: string, req: RegisterRequest): Promise<User> {
  return request<User>(server, "/auth/register", { method: "POST", body: req });
}

/** `POST /auth/login`. Resolve com o token e o usuário ou rejeita com `ApiError`. */
export function login(server: string, req: LoginRequest): Promise<LoginResponse> {
  return request<LoginResponse>(server, "/auth/login", { method: "POST", body: req });
}
