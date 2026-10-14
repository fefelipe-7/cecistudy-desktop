/**
 * Testes do índice dia → ocorrência.
 *
 * O caso que justifica o arquivo é o item que atravessa a meia-noite: ele é
 * buscado pelo SQL, indexado, pago na leitura — e, indexado pelo dia em que
 * começou, some da tela.
 */

import assert from "node:assert/strict";
import { test } from "node:test";

import { indexByDay, itemsOn, spansDay } from "./day-index.ts";
import { addDaysInTz, fromPlainDate, instant, plainDate, toPlainDate } from "./time.ts";
import { CALENDAR_ZONE } from "./zone.ts";

const ZONE = CALENDAR_ZONE;

/** `2026-03-10T23:30` → o instante correspondente no fuso do Calendário. */
function at(iso: string) {
  const base = fromPlainDate(plainDate(iso.slice(0, 10)), ZONE);
  return instant(base + Number(iso.slice(11, 13)) * 3_600_000 + Number(iso.slice(14, 16)) * 60_000);
}

const day = fromPlainDate(plainDate("2026-03-10"), ZONE);
const nextDay = addDaysInTz(day, 1, ZONE);

const window = { from: addDaysInTz(day, -1, ZONE), to: addDaysInTz(day, 8, ZONE) };

test("spansDay aceita o item que está dentro e recusa o de fora", () => {
  const inside = { startsAt: at("2026-03-10T09:00"), endsAt: at("2026-03-10T10:00") };
  const before = { startsAt: at("2026-03-08T09:00"), endsAt: at("2026-03-08T10:00") };
  const after = { startsAt: at("2026-03-12T09:00"), endsAt: at("2026-03-12T10:00") };

  assert.equal(spansDay(inside, day, ZONE), true);
  assert.equal(spansDay(before, day, ZONE), false);
  assert.equal(spansDay(after, day, ZONE), false);
});

test("um item que termina exatamente à meia-noite pertence só ao dia em que começa", () => {
  // `addDaysInTz` preserva a hora de parede, então a meia-noite do dia seguinte
  // se obtém do início do dia — não adicionando um dia às 23:00.
  const midnight = instant(addDaysInTz(fromPlainDate(plainDate("2026-03-11"), ZONE), 0, ZONE));
  const item = { startsAt: at("2026-03-10T23:00"), endsAt: midnight };

  assert.equal(spansDay(item, day, ZONE), true);
  assert.equal(spansDay(item, nextDay, ZONE), false);
});

test("um item que atravessa a meia-noite aparece nos DOIS dias", () => {
  const item = { id: "x", startsAt: at("2026-03-10T23:00"), endsAt: at("2026-03-11T07:00") };
  const index = indexByDay([item], window, ZONE);

  assert.deepEqual(itemsOn(index, plainDate("2026-03-10")), [item]);
  assert.deepEqual(itemsOn(index, plainDate("2026-03-11")), [item]);
});

test("um item que começa antes da janela aparece no dia em que se sobrepõe a ela", () => {
  // Começou três dias antes e atravessa a meia-noite: indexado por `startsAt`, ele
  // ficaria na chave de um dia que não existe na tela e sumiria inteiro.
  const item = { id: "y", startsAt: at("2026-03-07T23:00"), endsAt: at("2026-03-10T02:00") };
  const index = indexByDay([item], window, ZONE);

  // A janela abre em 09/03, então os dias 07 e 08 ficam de fora — mas 09/03 tem a
  // parte das 23h do dia anterior, e por isso o item também pertence a ele.
  assert.deepEqual(itemsOn(index, plainDate("2026-03-07")), []);
  assert.deepEqual(itemsOn(index, plainDate("2026-03-08")), []);
  assert.deepEqual(itemsOn(index, plainDate("2026-03-09")), [item]);
  assert.deepEqual(itemsOn(index, plainDate("2026-03-10")), [item]);
});

test("o índice é recortado na janela, nas duas pontas", () => {
  // A janela é [ontem, dia+8]; um item de março a abril aparece só nesses dias.
  const item = { id: "z", startsAt: at("2026-03-01T09:00"), endsAt: at("2026-04-30T09:00") };
  const index = indexByDay([item], window, ZONE);

  const keys = [...index.keys()].sort();
  assert.deepEqual(keys, [
    "2026-03-09",
    "2026-03-10",
    "2026-03-11",
    "2026-03-12",
    "2026-03-13",
    "2026-03-14",
    "2026-03-15",
    "2026-03-16",
    "2026-03-17",
    "2026-03-18",
  ]);
});

test("um item que não toca a janela não cria chave nenhuma", () => {
  const index = indexByDay(
    [{ startsAt: at("2026-03-08T09:00"), endsAt: at("2026-03-08T10:00") }],
    window,
    ZONE,
  );
  assert.equal(index.size, 0);
});

test("itemsOn devolve a lista vazia constante, para o memo não errar", () => {
  const index = indexByDay([], window, ZONE);
  const first = itemsOn(index, plainDate("2026-03-10"));
  const second = itemsOn(index, plainDate("2026-03-11"));
  // Identidade, não igualdade: dois `[]` novos a cada chamada fariam o `memo` da
  // coluna errar em todo render mesmo sem nenhuma mudança.
  assert.equal(first, second);
});

test("a chave do índice é a data civil no fuso, não a da máquina", () => {
  // 23:30 no Brasil é 02:30 UTC do dia seguinte. Indexar pela data UTC colocaria
  // o item na coluna errada sempre que a leitura viesse de madrugada.
  const item = { startsAt: at("2026-03-10T23:30"), endsAt: at("2026-03-10T23:45") };
  const index = indexByDay([item], window, ZONE);
  assert.ok(index.has(plainDate("2026-03-10")));
  assert.equal(toPlainDate(item.startsAt, ZONE), plainDate("2026-03-10"));
});

test("o índice sobrevive a item degenerado sem laço infinito", () => {
  // Fim antes do início, e item de duração zero: nenhuma chave pode aparecer, e
  // sobretudo o `while` tem de terminar.
  const inverted = indexByDay(
    [{ startsAt: at("2026-03-10T15:00"), endsAt: at("2026-03-10T09:00") }],
    window,
    ZONE,
  );
  const zero = indexByDay(
    [{ startsAt: at("2026-03-10T09:00"), endsAt: at("2026-03-10T09:00") }],
    window,
    ZONE,
  );

  assert.equal(inverted.size, 0);
  assert.equal(zero.size, 0);
});

test("instant usado só como tipo, sem quebrar a marca", () => {
  // Guarda contra alguém passar um número cru: o `Spans` exige `Instant`.
  const raw: Parameters<typeof spansDay>[0] = {
    startsAt: instant(at("2026-03-10T09:00")),
    endsAt: instant(at("2026-03-10T10:00")),
  };
  assert.equal(spansDay(raw, day, ZONE), true);
});
