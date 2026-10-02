import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { COMMITMENTS, ITEM_STATES } from "./commitment.ts";
import { isOverdue } from "./overdue.ts";
import { instant, type Instant } from "./time.ts";
import type { Commitment, ItemState } from "./types.ts";

const NOW = instant(Date.UTC(2026, 8, 29, 12, 0, 0));
const VENCIDO = instant(Date.UTC(2026, 8, 28, 12, 0, 0));
const FUTURO = instant(Date.UTC(2026, 8, 30, 12, 0, 0));
const LIMITE = instant(Date.UTC(2026, 8, 29, 12, 0, 0));

/** O prazo que vence no mesmo instante do `now` ainda não está atrasado. */
const ATRASA: ReadonlySet<ItemState> = new Set<ItemState>([
  "planejado",
  "em_andamento",
  "adiado",
  "nao_realizado",
]);

function esperado(commitment: Commitment, state: ItemState, dueAt: Instant | null): boolean {
  if (dueAt === null) return false;
  if (dueAt >= NOW) return false;
  if (commitment !== "obrigatorio" && commitment !== "importante") return false;
  return ATRASA.has(state);
}

describe("INV-4 — atraso é derivado, com 7 estados × 4 níveis × 2 prazos", () => {
  const combos: Array<[Commitment, ItemState, Instant | null]> = [];
  for (const commitment of COMMITMENTS) {
    for (const state of ITEM_STATES) {
      combos.push([commitment, state, VENCIDO]);
      combos.push([commitment, state, null]);
    }
  }

  it("a tabela tem 56 combinações e nenhuma delas foge da regra", () => {
    assert.equal(combos.length, 56);
    assert.equal(new Set(combos.map(([c, s, d]) => `${c}|${s}|${d}`)).size, 56);
  });

  for (const [commitment, state, dueAt] of combos) {
    it(`${commitment} · ${state} · ${dueAt === null ? "sem prazo" : "vencido"}`, () => {
      assert.equal(
        isOverdue({ commitment, state, dueAt }, NOW),
        esperado(commitment, state, dueAt),
      );
    });
  }
});

describe("INV-4 — as bordas que não podem ser lidas de outra forma", () => {
  it("um prazo que vence exatamente agora não atrasa", () => {
    assert.equal(
      isOverdue({ commitment: "obrigatorio", state: "planejado", dueAt: LIMITE }, NOW),
      false,
    );
  });

  it("um milissegundo antes de agora já atrasa", () => {
    const almost = instant(NOW - 1);
    assert.equal(
      isOverdue({ commitment: "obrigatorio", state: "planejado", dueAt: almost }, NOW),
      true,
    );
  });

  it("sem prazo, nunca há atraso, nem com obrigação e prazo implícito", () => {
    assert.equal(
      isOverdue({ commitment: "obrigatorio", state: "adiado", dueAt: null }, NOW),
      false,
    );
  });

  it("prazo futuro não atrasa", () => {
    assert.equal(
      isOverdue({ commitment: "obrigatorio", state: "planejado", dueAt: FUTURO }, NOW),
      false,
    );
  });

  it("cancelado, dispensado e concluído nunca atrasam, mesmo vencidos", () => {
    for (const state of ["cancelado", "dispensado", "concluido"] as const) {
      assert.equal(
        isOverdue({ commitment: "obrigatorio", state, dueAt: VENCIDO }, NOW),
        false,
        `${state} não pode atrasar`,
      );
    }
  });

  it("recomendado e opcional não atrasam mesmo vencidos e abertos", () => {
    for (const commitment of ["recomendado", "opcional"] as const) {
      for (const state of ITEM_STATES) {
        assert.equal(
          isOverdue({ commitment, state, dueAt: VENCIDO }, NOW),
          false,
          `${commitment}/${state} não pode atrasar`,
        );
      }
    }
  });

  it("adiado e não realizado continuam atrasando: adiar não zera a dívida", () => {
    for (const state of ["adiado", "nao_realizado"] as const) {
      assert.equal(isOverdue({ commitment: "importante", state, dueAt: VENCIDO }, NOW), true);
    }
  });

  it("o mesmo item muda de leitura quando o relógio passa do prazo", () => {
    const item = { commitment: "obrigatorio", state: "planejado", dueAt: FUTURO } as const;

    assert.equal(isOverdue(item, NOW), false);
    assert.equal(isOverdue(item, instant(Date.UTC(2026, 8, 31, 12, 0, 0))), true);
  });

  it("o mesmo item deixa de atrasar quando é concluído, sem mexer no prazo", () => {
    const item = { commitment: "obrigatorio", dueAt: VENCIDO } as const;

    assert.equal(isOverdue({ ...item, state: "planejado" }, NOW), true);
    assert.equal(isOverdue({ ...item, state: "concluido" }, NOW), false);
  });
});
