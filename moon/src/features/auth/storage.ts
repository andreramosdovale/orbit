//! Dados da sessão guardados neste PC. O token fica aqui pelo ADR 0010.

const SERVER_KEY = "orbit.server";
const TOKEN_KEY = "orbit.token";

function read(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Sem localStorage o usuário só precisa redigitar na próxima vez que abrir o app.
  }
}

export function loadServer(): string {
  return read(SERVER_KEY) ?? "";
}

export function saveServer(server: string) {
  write(SERVER_KEY, server);
}

export function loadToken(): string | null {
  return read(TOKEN_KEY);
}

export function saveToken(token: string) {
  write(TOKEN_KEY, token);
}

export function clearToken() {
  try {
    localStorage.removeItem(TOKEN_KEY);
  } catch {
    // Se o localStorage está indisponível, o token também não foi gravado.
  }
}
