import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { DICA_PADRAO, pickTip } from "./tips.ts";

const AGORA = new Date("2026-10-10T12:00:00");

describe("pickTip", () => {
  it("é determinística para o mesmo contexto", () => {
    const ctx = { dueCards: 3 };
    assert.equal(pickTip(ctx, AGORA), pickTip(ctx, AGORA));
  });

  it("a candidata mais específica vence (prova próxima vence cartões)", () => {
    const dica = pickTip({ dueCards: 10, nextExamDate: "2026-10-12" }, AGORA);
    assert.match(dica, /prova/i);
  });

  it("sem contexto, devolve a dica de foco (a mais específica aplicável), sem metáfora cozy", () => {
    // Com tudo zerado, `focusMinutesToday === 0` vence o fallback — mesma
    // semântica do mobile. A dica padrão só aparece quando há foco e nenhuma
    // outra condição (ex.: minutos de foco > 0 e nada pendente).
    assert.equal(pickTip({ focusMinutesToday: 30, streakDays: 0 }), DICA_PADRAO);
  });

  it("nenhuma dica usa a metáfora cozy (D4)", () => {
    for (const ctx of [{}, { focusMinutesToday: 30 }, { dueCards: 2 }, { pendingTasks: 1 }]) {
      assert.ok(!pickTip(ctx).includes("cantinho"), `dica com "cantinho": ${pickTip(ctx)}`);
    }
  });

  it("respeita o contexto de cartões vencidos", () => {
    assert.match(pickTip({ dueCards: 3 }, AGORA), /3 cartões/);
    assert.match(pickTip({ dueCards: 1 }, AGORA), /1 cartão /);
  });
});
