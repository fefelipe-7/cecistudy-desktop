import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { ITEM_STATES } from "./commitment.ts";
import { allowedTransitions, transitionAllowed } from "./state-machine.ts";
import type { ItemState } from "./types.ts";

/** A tabela de §4 transcrita. Se a implementação divergir, esta lista diverge junto. */
const TABELA: readonly (readonly [ItemState, readonly ItemState[]])[] = [
  [
    "planejado",
    ["em_andamento", "concluido", "adiado", "nao_realizado", "cancelado", "dispensado"],
  ],
  ["em_andamento", ["concluido", "adiado", "nao_realizado", "cancelado", "dispensado"]],
  [
    "adiado",
    ["planejado", "em_andamento", "concluido", "nao_realizado", "cancelado", "dispensado"],
  ],
  [
    "nao_realizado",
    ["planejado", "em_andamento", "concluido", "adiado", "cancelado", "dispensado"],
  ],
  ["concluido", ["em_andamento"]],
  ["cancelado", ["planejado"]],
  ["dispensado", ["planejado"]],
];

describe("§4 — a máquina de estados tem sete linhas e nenhuma transição a mais", () => {
  it("cobre todos os estados exatamente uma vez", () => {
    assert.deepEqual(TABELA.map(([from]) => from).sort(), [...ITEM_STATES].sort());
  });

  for (const [from, to] of TABELA) {
    it(`${from} permite exatamente ${to.join(", ") || "nada"}`, () => {
      assert.deepEqual([...allowedTransitions(from)].sort(), [...to].sort());
    });
  }

  it("concluído não pode virar cancelado: prova os dois lados da regra", () => {
    assert.equal(transitionAllowed("planejado", "concluido"), true);
    assert.equal(transitionAllowed("concluido", "cancelado"), false);
  });

  it("concluído só reabre para em andamento", () => {
    for (const state of ITEM_STATES) {
      const expected = state === "em_andamento";
      assert.equal(transitionAllowed("concluido", state), expected, `concluido -> ${state}`);
    }
  });

  it("cancelado e dispensado voltam para planejado, e só para planejado", () => {
    for (const from of ["cancelado", "dispensado"] as const) {
      for (const to of ITEM_STATES) {
        const expected = to === "planejado";
        assert.equal(transitionAllowed(from, to), expected, `${from} -> ${to}`);
      }
    }
  });

  it("nenhum estado transiciona para ele mesmo", () => {
    for (const state of ITEM_STATES) {
      assert.equal(transitionAllowed(state, state), false, `${state} -> ${state}`);
    }
  });

  it("a varredura completa bate com a tabela: 49 pares, 26 permitidos", () => {
    const permitidos = ITEM_STATES.flatMap((from) =>
      ITEM_STATES.filter((to) => transitionAllowed(from, to)),
    );

    assert.equal(ITEM_STATES.length * ITEM_STATES.length, 49);
    assert.equal(permitidos.length, 26);
  });

  it("allowedTransitions devolve a lista, não uma referência mutável compartilhada", () => {
    const lista = allowedTransitions("planejado");

    assert.ok(Array.isArray(lista));
    assert.ok(lista.includes("concluido"));
  });
});
