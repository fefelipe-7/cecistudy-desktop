#!/usr/bin/env node
/**
 * Gate de paridade do contrato entre Rust, TypeScript e o manifest (`SPEC-D-013`
 * `D85`).
 *
 * Sem dependências, roda em segundos, sem build e **sem toolchain Rust** — o que
 * importa, porque é o único jeito de o gate rodar na máquina de quem escreve
 * TypeScript.
 *
 * O problema que ele existe para resolver está no código e é verificável hoje:
 * `src-tauri/src/lib.rs:37-39` afirma que os handlers são "os mesmos 20 nomes
 * que `data/bridge.ts` invoca". São 21 registrados e 19 wrappers — e dois dos
 * registrados, `get_setting` e `set_setting`, não têm nenhum consumidor
 * TypeScript. O comentário está errado porque ninguém comparou.
 *
 * As regras saem da `SPEC-D-013`:
 *
 * 1. **Três conjuntos iguais** (`I5`). Manifest, `invoke_handler` do Rust e
 *    funções que chamam `invoke`. Divergir em qualquer sentido falha.
 * 2. **Nenhum comando escreve em dado de outro dono** (`I6`).
 * 3. **Nenhum módulo escreve em identidade ou preferência** (`I7`), porque `D87`
 *    diz que só Configurações escreve.
 * 4. **Todo comando declara `escreve` explicitamente** — mesmo quando é `[]`.
 *    Comando sem `escreve` declarado não é verificável, e o padrão é não declarar.
 */

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname;
const SRC = join(ROOT, "src");
const RUST_LIB = join(ROOT, "src-tauri", "src", "lib.rs");
const CONTRACTS = join(ROOT, "contracts");

/** Dados que `D87` reserva ao módulo Configurações. */
const DE_USUARIO = new Set(["identidade", "preferencia"]);

function exists(p) {
  try {
    statSync(p);
    return true;
  } catch {
    return false;
  }
}

function walk(dir, out = []) {
  for (const entry of readdirSync(dir)) {
    if (entry === "node_modules" || entry === "dist" || entry.startsWith(".")) continue;
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) walk(full, out);
    else if (/\.(ts|tsx)$/.test(full)) out.push(full);
  }
  return out;
}

function stripComments(text) {
  return text
    .replace(/\/\*[\s\S]*?\*\//g, (m) => m.replace(/[^\n]/g, " "))
    .replace(/(^|[^:])\/\/[^\n]*/g, (_m, prefix) => prefix);
}

const failures = [];
const rel = (f) => relative(ROOT, f);

// ------------------------------------------------------- conjuntos do Rust

function comandosRegistrados() {
  const texto = stripComments(readFileSync(RUST_LIB, "utf8"));
  // `tauri::generate_handler![...]` é a lista que vale. Um `#[tauri::command]`
  // que não está aqui é órfão pelo outro motivo.
  const bloco = texto.match(/generate_handler!\s*\[([\s\S]*?)\]/);
  if (!bloco) {
    console.error("Gate de contrato INDISPONÍVEL: generate_handler! não encontrado em lib.rs");
    process.exit(1);
  }
  return [...bloco[1].matchAll(/(\w+)\s*(?:::|,|\s*\n\s*\})/g)]
    .map((m) => m[1])
    .filter((n) => n !== "commands");
}

const rust = new Set(comandosRegistrados());

// ------------------------------------------- conjuntos do TypeScript (invoke)

const ponteiros = walk(SRC).filter((f) => /[\\/]data[\\/][^\\/]+\.ts$/.test(f));

const wrappers = new Set();
for (const arquivo of ponteiros) {
  const texto = readFileSync(arquivo, "utf8");
  // O parâmetro de tipo é opcional: `invoke("nome", {...})` também é a fronteira
  // legada, e deixar de fora as chamadas sem tipo é como o gate passa verde com
  // metade do contrato fora dele.
  for (const m of texto.matchAll(/invoke(?:<[^>]*>)?\s*\(\s*"(\w+)"/g)) wrappers.add(m[1]);
}

// ------------------------------------------------------ conjuntos do manifest

if (!exists(CONTRACTS)) {
  console.error(`Gate de contrato VIOLADO:\n  contracts/ não existe (D85)`);
  process.exit(1);
}

const manifestPorNome = new Map();
const modulosManifest = [];

for (const arquivo of readdirSync(CONTRACTS).filter((f) => f.endsWith(".json"))) {
  const caminho = join(CONTRACTS, arquivo);
  let json;
  try {
    json = JSON.parse(readFileSync(caminho, "utf8"));
  } catch (erro) {
    console.error(`Gate de contrato VIOLADO:\n  ${rel(caminho)}  JSON inválido: ${erro.message}`);
    process.exit(1);
  }
  modulosManifest.push(json.modulo);
  if (!Array.isArray(json.comandos) || json.comandos.length === 0) {
    failures.push(`  ${rel(caminho)}  declara módulo sem comando algum`);
    continue;
  }
  for (const comando of json.comandos) {
    if (manifestPorNome.has(comando.nome)) {
      failures.push(
        `  ${rel(caminho)}  comando "${comando.nome}" já declarado em outro manifest (${manifestPorNome.get(comando.nome).arquivo})`,
      );
    }
    manifestPorNome.set(comando.nome, { ...comando, modulo: json.modulo, arquivo: rel(caminho) });

    // Regra 4 — todo comando declara o que escreve, mesmo quando nada.
    if (!Array.isArray(comando.escreve)) {
      failures.push(`  ${rel(caminho)}  "${comando.nome}" não declara "escreve"`);
    }
    if (!Array.isArray(comando.le)) {
      failures.push(`  ${rel(caminho)}  "${comando.nome}" não declara "le"`);
    }
  }
}

// ------------------------------------------------------------------ regra 1

for (const nome of rust) {
  if (!manifestPorNome.has(nome)) {
    failures.push(
      `  src-tauri/src/lib.rs  comando "${nome}" está registrado e não está em nenhum manifest`,
    );
  }
}
for (const nome of wrappers) {
  if (!manifestPorNome.has(nome)) {
    failures.push(`  TypeScript  invoke("${nome}") não está em nenhum manifest`);
  }
}
for (const nome of manifestPorNome.keys()) {
  if (!rust.has(nome)) {
    const c = manifestPorNome.get(nome);
    failures.push(
      `  ${c.arquivo}  "${nome}" está no manifest e não está registrado em src-tauri/src/lib.rs`,
    );
  }
  if (!wrappers.has(nome)) {
    const c = manifestPorNome.get(nome);
    failures.push(
      `  ${c.arquivo}  "${nome}" está no manifest e não tem wrapper em src/features/*/data/ — comando órfão (D85)`,
    );
  }
}

// ------------------------------------------------------------------ regra 2 e 3

for (const [nome, comando] of manifestPorNome) {
  if (!Array.isArray(comando.escreve)) continue;
  for (const dono of comando.escreve) {
    // Regra 3 — só Configurações escreve identidade e preferência.
    if (DE_USUARIO.has(dono) && comando.modulo !== "usuario") {
      failures.push(
        `  ${comando.arquivo}  "${nome}" (módulo ${comando.modulo}) escreve "${dono}", que D87 reserva a Configurações`,
      );
    }
  }
}

// ------------------------------------------------------------------- saída

if (failures.length > 0) {
  console.error(`Contrato Rust/TypeScript VIOLADO (${failures.length}):\n`);
  console.error(failures.join("\n"));
  console.error(
    "\nVer contracts/*.json e SPEC-D-013 D85. Três conjuntos têm de ser iguais:\n" +
      "  manifest · generate_handler! em src-tauri/src/lib.rs · invoke em src/features/*/data/",
  );
  process.exit(1);
}

console.log(
  `OK: contrato em paridade (${manifestPorNome.size} comandos em ${modulosManifest.length} manifests, ` +
    `${ponteiros.length} pontes verificadas).`,
);
