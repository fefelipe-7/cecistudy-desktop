import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { findOverlaps, overlapsPair } from "./overlap.ts";
import { instant, minutes } from "./time.ts";
import { addMinutes } from "./time.ts";

interface Box {
  readonly id: string;
  readonly startsAt: ReturnType<typeof instant>;
  readonly endsAt: ReturnType<typeof instant>;
}

const HOUR = minutes(60);

function box(id: string, startHour: number, endHour: number): Box {
  const startsAt = instant(Date.UTC(2026, 8, 29, startHour, 0, 0));
  return { id, startsAt, endsAt: addMinutes(startsAt, minutes((endHour - startHour) * 60)) };
}

function ids(group: readonly [Box, readonly Box[]]): string[] {
  return [group[0].id, ...group[1].map((item) => item.id)];
}

describe("D7 — sobreposição é derivada, sinalizada e nunca bloqueante", () => {
  it("[a 08–10, b 09–11, c 11–12] devolve um grupo {a,b} e deixa c isolado", () => {
    const a = box("a", 8, 10);
    const b = box("b", 9, 11);
    const c = box("c", 11, 12);

    const groups = findOverlaps([a, b, c]);

    assert.equal(groups.length, 1);
    assert.deepEqual(ids(groups[0]!), ["a", "b"]);
  });

  it("[a 08–10, b 10–12] não devolve grupo nenhum: intervalos que se tocam não conflitam", () => {
    assert.deepEqual(findOverlaps([box("a", 8, 10), box("b", 10, 12)]), []);
  });

  it("uma cadeia a-b-c vira um grupo só", () => {
    const groups = findOverlaps([box("a", 8, 10), box("b", 9, 11), box("c", 10, 12)]);

    assert.equal(groups.length, 1);
    assert.deepEqual(ids(groups[0]!), ["a", "b", "c"]);
  });

  it("item isolado não aparece", () => {
    assert.deepEqual(findOverlaps([box("a", 8, 9), box("b", 10, 11), box("c", 12, 13)]), []);
  });

  it("lista vazia e lista de um só não explodem", () => {
    assert.deepEqual(findOverlaps([]), []);
    assert.deepEqual(findOverlaps([box("a", 8, 10)]), []);
  });

  it("a ordem de entrada não muda o resultado", () => {
    const a = box("a", 8, 10);
    const b = box("b", 9, 11);
    const c = box("c", 11, 12);

    assert.deepEqual(findOverlaps([c, b, a]), findOverlaps([a, b, c]));
  });

  it("um segundo par que só se toca também não vira grupo", () => {
    const groups = findOverlaps([
      box("a", 8, 10),
      box("b", 9, 11),
      box("c", 14, 15),
      box("d", 15, 17),
    ]);

    assert.equal(groups.length, 1);
    assert.deepEqual(ids(groups[0]!), ["a", "b"]);
  });

  it("grupos separados de verdade devolvem dois grupos", () => {
    const groups = findOverlaps([
      box("a", 8, 10),
      box("b", 9, 11),
      box("c", 14, 16),
      box("d", 15, 17),
    ]);

    assert.equal(groups.length, 2);
    assert.deepEqual(ids(groups[0]!), ["a", "b"]);
    assert.deepEqual(ids(groups[1]!), ["c", "d"]);
  });

  it("o conteúdo sobrevive ao agrupamento: a âncora é a primeira caixa do grupo", () => {
    const late = box("late", 9, 11);
    const early = box("early", 8, 10);

    const groups = findOverlaps([late, early]);

    assert.equal(groups[0]![0].id, "early");
  });

  it("um item contido em outro ainda conta como sobreposição", () => {
    const groups = findOverlaps([box("outer", 8, 12), box("inner", 9, 10)]);

    assert.equal(groups.length, 1);
    assert.deepEqual(ids(groups[0]!), ["outer", "inner"]);
  });
});

describe("overlapsPair", () => {
  it("separa sobreposição de simples contato", () => {
    const a = box("a", 8, 10);
    const b = box("b", 9, 11);
    const c = box("c", 10, 12);

    assert.equal(overlapsPair(a, b), true);
    assert.equal(overlapsPair(b, a), true);
    assert.equal(overlapsPair(a, c), false);
  });

  it("zero duração não se sobrepõe a si mesmo", () => {
    const point: Box = { id: "p", startsAt: instant(HOUR), endsAt: instant(HOUR) };

    assert.equal(overlapsPair(point, point), false);
  });
});
