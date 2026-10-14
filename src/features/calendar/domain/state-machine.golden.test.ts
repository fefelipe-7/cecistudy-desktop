/**
 * O golden é o contrato entre os dois lados da regra (D61 da SPEC-D-005).
 *
 * Este arquivo lê o **mesmo** `golden/state-machine.jsonl` que o teste Rust em
 * `src-tauri/src/commands.rs` lê. Divergência entre os dois lados é falha, e a
 * falha nomeia o caso.
 *
 * Rust é a autoridade (ADR-006, corolário 2). Este lado é espelho: existe para
 * a render e para o gate de divergência, e é o lado errado quando os dois
 * discordam.
 */

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { transitionAllowed } from "./state-machine.ts";

const GOLDEN = fileURLToPath(new URL("../../../../golden/state-machine.jsonl", import.meta.url));

interface Caso {
  readonly caso: string;
  readonly entrada: { readonly from: string; readonly to: string };
  readonly saida: { readonly aceito: boolean };
}

function casos(): readonly Caso[] {
  return readFileSync(GOLDEN, "utf8")
    .split(/\r?\n/)
    .map((linha) => linha.trim())
    .filter((linha) => linha.length > 0 && !linha.startsWith("#"))
    .map((linha) => JSON.parse(linha) as Caso);
}

test("o espelho TypeScript concorda com o golden em todos os casos", () => {
  const lista = casos();
  for (const caso of lista) {
    assert.equal(
      transitionAllowed(caso.entrada.from as never, caso.entrada.to as never),
      caso.saida.aceito,
      `caso \`${caso.caso}\`: o espelho TypeScript discorda do golden`,
    );
  }
});

test("o golden é completo: 7 estados × 7 destinos", () => {
  assert.equal(casos().length, 49);
});
