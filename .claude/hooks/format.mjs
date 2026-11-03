// PostToolUse: formata o arquivo que o Claude acabou de editar.
// .rs → rustfmt | .ts/.tsx/.js/.json/.css dentro de moon/ → Biome.
// Nunca bloqueia: se o formatador falhar (ex.: erro de sintaxe), segue em frente.

import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";

const input = JSON.parse(readFileSync(0, "utf8"));
const file = input.tool_input?.file_path;
if (!file) process.exit(0);

const root = process.env.CLAUDE_PROJECT_DIR ?? process.cwd();
const rel = path.relative(root, path.resolve(file)).split(path.sep).join("/");
const ext = path.extname(file);

if (ext === ".rs") {
  spawnSync("rustfmt", ["--edition", "2024", file], { stdio: "ignore", shell: true });
} else if (
  rel.startsWith("moon/") &&
  !rel.startsWith("moon/src-tauri/") &&
  rel !== "moon/src/bindings.ts" &&
  [".ts", ".tsx", ".js", ".mjs", ".json", ".css"].includes(ext)
) {
  const moon = path.join(root, "moon");
  const biome = path.join(moon, "node_modules", "@biomejs", "biome", "bin", "biome");
  spawnSync(process.execPath, [biome, "check", "--write", file], { cwd: moon, stdio: "ignore" });
}
