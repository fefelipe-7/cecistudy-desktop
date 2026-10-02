#!/usr/bin/env node
/**
 * Gate de fronteiras do Campus (débitos C5/C6 do grupo).
 *
 * Sem dependências, roda em segundos, sem build. Cada regra é uma convenção do
 * `AGENTS.md` transformada em comando executável — convenção que não é
 * executada é convenção que não existe.
 *
 * As regras saem do que a `AGENTS.md` já afirma:
 *
 * 1. **Só `src/lib/ipc.ts` fala com o Rust.** Componente nunca chama `invoke`
 *    direto. `data/bridge.ts` é o único consumidor permitido.
 * 2. **`domain/` é puro.** Não importa `ui/` nem `data/`, não importa React e
 *    não toca o relógio — o tempo entra por `Clock` injetado, e é isso que
 *    torna a regra testável sem mock global.
 * 3. **Só o arquivo do IPC importa `@tauri-apps/api`.**
 *
 * ⚠️ Comentários são removidos antes de casar: `domain/grid-scale.ts` cita
 * `invoke` e `Date.now()` no doc comment para dizer que *não* os usa, e um gate
 * que lê comentário obriga o código a mentir no comentário.
 */

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname;
const SRC = join(ROOT, "src");

/** Arquivos autorizados a conter `invoke(` — o resto de `src/` é proibido. */
const IPC_ALLOWED = new Set([
  join(SRC, "lib", "ipc.ts"),
  join(SRC, "features", "calendar", "data", "bridge.ts"),
]);

/** Arquivos autorizados a importar o pacote do Tauri. */
const TAURI_ALLOWED = new Set([join(SRC, "lib", "ipc.ts")]);

const DOMAIN_DIR = join(SRC, "features", "calendar", "domain");

/** Remove comentários preservando as quebras de linha (para não colar linhas). */
function stripComments(text) {
  return text
    .replace(/\/\*[\s\S]*?\*\//g, (m) => m.replace(/[^\n]/g, " "))
    .replace(/(^|[^:])\/\/[^\n]*/g, (_m, prefix) => prefix);
}

function walk(dir, out = []) {
  for (const entry of readdirSync(dir)) {
    if (entry === "node_modules" || entry === "dist" || entry.startsWith(".")) continue;
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) walk(full, out);
    else if (/\.(ts|tsx)$/.test(full) && !/\.test\.tsx?$/.test(full)) out.push(full);
  }
  return out;
}

const failures = [];
const report = (file, line, rule) => failures.push(`  ${relative(ROOT, file)}:${line}  ${rule}`);

const files = walk(SRC);

for (const file of files) {
  const raw = readFileSync(file, "utf8");
  const code = stripComments(raw);
  const lines = code.split("\n");
  const rel = relative(ROOT, file);

  // Regra 1 — invoke só no IPC e na ponte.
  if (!IPC_ALLOWED.has(file)) {
    lines.forEach((line, i) => {
      if (/\binvoke\s*[<(]/.test(line) || /from\s+["']@tauri-apps\/api/.test(line)) {
        report(file, i + 1, "invoke/@tauri-apps/api só em src/lib/ipc.ts e data/bridge.ts");
      }
    });
  }

  // Regra 3 — o pacote do Tauri é do módulo do IPC.
  if (!TAURI_ALLOWED.has(file) && /from\s+["']@tauri-apps\/api/.test(raw)) {
    report(file, 1, "@tauri-apps/api só pode ser importado em src/lib/ipc.ts");
  }

  // Regra 2 — domain/ é puro.
  if (file.startsWith(DOMAIN_DIR)) {
    lines.forEach((line, i) => {
      if (/from\s+["'][^"']*\/(ui|data)\//.test(line)) {
        report(file, i + 1, "domain/ não importa ui/ nem data/");
      }
      if (/from\s+["']react["']/.test(line)) {
        report(file, i + 1, "domain/ não importa React");
      }
      // `Date.now()` e `new Date()` sem argumento são o que quebra a testabilidade:
      // é "que horas são agora", e o relógio tem de entrar por `Clock` injetado.
      // `new Date(<valor>)` **não** é proibido: é conversão de um instante já
      // pronto para um tipo que a biblioteca exige (a `rrule` só aceita `Date`),
      // e não consulta o relógio.
      if (/Date\.now\s*\(/.test(line) || /new\s+Date\s*\(\s*\)/.test(line)) {
        report(file, i + 1, "domain/ não lê o relógio — o tempo entra por Clock injetado");
      }
      if (/from\s+["']\.\.\/data\/bridge/.test(line)) {
        report(file, i + 1, "domain/ não conhece a ponte de dados");
      }
    });
  }
}

const checked = files.length;
if (failures.length > 0) {
  console.error(`Fronteiras VIOLADAS (${failures.length} em ${checked} arquivos):\n`);
  console.error(failures.join("\n"));
  console.error("\nVer src/lib/ipc.ts e src/features/calendar/domain/ para o contrato.");
  process.exit(1);
}

console.log(
  `OK: fronteiras respeitadas (${checked} arquivos, ${IPC_ALLOWED.size} com acesso ao IPC).`,
);
