// `npm run dev:app`: abre o app em modo dev na primeira porta livre, para rodar
// mais de uma instância ao mesmo tempo. Cada porta é uma origem diferente, então
// cada instância tem o próprio localStorage e pode entrar com outra conta.

import { spawn } from "node:child_process";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";

// A mesma faixa que o `sun` libera no CORS (`sun::DEV_PORTS`).
const FIRST_PORT = 1420;
const LAST_PORT = 1439;

const MOON_DIR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const TAURI_CLI = path.join(MOON_DIR, "node_modules", "@tauri-apps", "cli", "tauri.js");

function isFreeOn(port, host) {
  return new Promise((resolve) => {
    const server = net.createServer();
    server.once("error", () => resolve(false));
    server.listen({ port, host }, () => server.close(() => resolve(true)));
  });
}

async function firstFreePort() {
  for (let port = FIRST_PORT; port <= LAST_PORT; port += 1) {
    // O Vite escuta em `localhost`, que no Windows costuma ser só o IPv6.
    if ((await isFreeOn(port, "::1")) && (await isFreeOn(port, "127.0.0.1"))) return port;
  }
  return null;
}

const port = await firstFreePort();
if (port === null) {
  console.error(`Nenhuma porta livre entre ${FIRST_PORT} e ${LAST_PORT}.`);
  process.exit(1);
}

const env = { ...process.env, ORBIT_DEV_PORT: String(port) };
if (port !== FIRST_PORT && !env.CARGO_TARGET_DIR) {
  // A URL entra no executável, e um moon.exe aberto fica travado: cada porta extra
  // precisa da própria pasta de build. A primeira compilação dela demora mais.
  env.CARGO_TARGET_DIR = path.join(MOON_DIR, "..", "target", `dev-${port}`);
}

console.log(`Abrindo o Orbit na porta ${port}.`);
const config = JSON.stringify({ build: { devUrl: `http://localhost:${port}` } });
const child = spawn(
  process.execPath,
  [TAURI_CLI, "dev", "--config", config, ...process.argv.slice(2)],
  { cwd: MOON_DIR, env, stdio: "inherit" },
);
child.once("error", (err) => {
  console.error(`Não foi possível iniciar o Tauri: ${err.message}`);
  process.exit(1);
});
child.once("exit", (code) => process.exit(code ?? 1));
