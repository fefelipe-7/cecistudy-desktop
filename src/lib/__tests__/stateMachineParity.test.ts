/**
 * Paridade da tabela de transições entre o Rust (fonte) e o espelho TypeScript.
 *
 * ## Por que este teste existe
 *
 * `domain/state-machine.ts` e `ITEM_TRANSITIONS` em `src-tauri/src/commands.rs`
 * são **a mesma regra em dois lugares** — o Rust é a fonte, o TypeScript é o
 * espelho (ver o doc comment de ambos). Espelho divergindo da fonte é o débito
 * C6 da recorrência: dois motores de RRULE com capacidades diferentes, e
 * ninguém percebendo até um deles virar semanal.
 *
 * Testes separados dos dois lados **não** pegam isso: os dois passam, cada um
 * contra a sua própria cópia. Só comparar as tabelas pega. Este teste lê a
 * constante do Rust direto do fonte e confere linha a linha.
 *
 * Se ele falhar, a correção é no arquivo que a mensagem apontar:
 * `espelho (TS)` ou `fonte (Rust)` — nunca "escolha o que parece certo".
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import assert from "node:assert/strict";
import { describe, it } from "node:test";

import {
  allowedTransitions,
  transitionAllowed,
} from "../../features/calendar/domain/state-machine.ts";
import { ITEM_STATES } from "../../features/calendar/domain/commitment.ts";
import type { ItemState } from "../../features/calendar/domain/types.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const RUST = join(HERE, "../../../src-tauri/src/commands.rs");

/** Lê `ITEM_TRANSITIONS` do fonte Rust e devolve `Map<from, to[]>`. */
function transicoesDoRust(): Map<string, string[]> {
  const fonte = readFileSync(RUST, "utf8");
  const inicio = fonte.indexOf("const ITEM_TRANSITIONS");
  assert.ok(inicio !== -1, "ITEM_TRANSITIONS não existe em commands.rs");

  const fim = fonte.indexOf("\n];", inicio);
  assert.ok(fim !== -1, "ITEM_TRANSITIONS não tem fim — o formato mudou?");
  const corpo = fonte.slice(inicio, fim);

  // Cada entrada é `("from", &[ "to", ... ])`, com quebra de linha livre.
  const entradas = [...corpo.matchAll(/\(\s*"([a-z_]+)",\s*&\[([\s\S]*?)\]/g)];
  assert.ok(entradas.length > 0, "nenhuma transição lida do Rust");

  const tabela = new Map<string, string[]>();
  for (const entrada of entradas) {
    // Os grupos do `matchAll` são `string | undefined` no tipo do TS; o regex
    // garante que existem, e um estado ilegível tem de fazer o teste falhar em
    // vez de virar `undefined` silencioso.
    const from = entrada[1];
    const corpoTo = entrada[2];
    assert.ok(from && corpoTo, `entrada de transição ilegível: ${entrada[0]}`);
    const para = [...corpoTo.matchAll(/"([a-z_]+)"/g)].map((m) => m[1] as string);
    tabela.set(from, para);
  }
  return tabela;
}

describe("paridade da tabela de transições (Rust é a fonte)", () => {
  it("o Rust tem 7 estados, como a spec §3.1", () => {
    const tabela = transicoesDoRust();
    assert.equal(tabela.size, 7);
  });

  it("mesmos estados de origem nos dois lados", () => {
    const rust = [...transicoesDoRust().keys()].sort();
    // A lista canônica do espelho é `ITEM_STATES` — `allowedTransitions` é
    // função, então `Object.keys` dela devolve `[]`.
    const ts = [...ITEM_STATES].sort();
    assert.deepEqual(ts, rust, "espelho (TS) e fonte (Rust) têm estados diferentes");
  });

  it("mesmos destinos, estado a estado", () => {
    const rust = transicoesDoRust();
    for (const [from, paraRust] of rust) {
      const paraTs = [...allowedTransitions(from as ItemState)];
      assert.deepEqual(
        paraTs,
        paraRust,
        `divergência em "${from}": espelho (TS) diz [${paraTs}], fonte (Rust) diz [${paraRust}]`,
      );
    }
  });

  it("transitionAllowed concorda com a tabela do Rust em todo par", () => {
    const rust = transicoesDoRust();
    const todos = [...new Set([...rust.values()].flat())] as ItemState[];
    for (const from of rust.keys() as IterableIterator<ItemState>) {
      for (const to of todos) {
        const esperado = rust.get(from)!.includes(to);
        assert.equal(
          transitionAllowed(from, to),
          esperado,
          `transitionAllowed("${from}", "${to}") discorda do Rust`,
        );
      }
    }
  });

  it("a fonte recusa a identidade: nenhum estado transiciona para si mesmo", () => {
    for (const from of transicoesDoRust().keys()) {
      assert.equal(
        transitionAllowed(from as ItemState, from as ItemState),
        false,
        `espelho (TS) permite "${from}" → "${from}", mas a lista de §3.1 é fechada`,
      );
    }
  });
});
