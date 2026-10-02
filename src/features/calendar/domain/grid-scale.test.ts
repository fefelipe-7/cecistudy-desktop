/**
 * Testes da geometria da grade (D15).
 *
 * O caso que mais importa aqui é o inverso: `timeToY` e `yToTime` precisam ser
 * inversas **uma da outra**. Se deixarem de ser, o item aparece numa hora e é
 * gravado em outra, e o bug só aparece em tela.
 */

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  DEFAULT_SCALE,
  clampY,
  columnize,
  placeInDay,
  snapTime,
  snapY,
  timeToY,
  visibleHeight,
  yToTime,
} from "./grid-scale.ts";
import {
  addDaysInTz,
  fromPlainDate,
  ianaTimeZone,
  instant,
  minutes,
  plainDate,
  startOfDay,
  toPlainDate,
} from "./time.ts";

const ZONE = ianaTimeZone("America/Sao_Paulo");
const scale = { ...DEFAULT_SCALE, timeZone: ZONE };

/** 2026-03-10 00:00 no fuso do evento. */
const day = fromPlainDate(plainDate("2026-03-10"), ZONE);

function at(hour: number, minute = 0): ReturnType<typeof instant> {
  const base = fromPlainDate(plainDate("2026-03-10"), ZONE);
  return instant(base + (hour * 60 + minute) * 60_000) as never;
}

test("timeToY posiciona pela hora do fuso, não pela hora UTC", () => {
  // Com dayStartHour 6, 08:00 fica a 2 horas do topo.
  assert.equal(timeToY(at(8), scale), 2 * 56);
  assert.equal(timeToY(at(6), scale), 0);
  // Antes da primeira hora visível dá negativo: o chamador recorta, e um valor
  // negativo "colado" no topo seria pior do que o clip explícito.
  assert.ok(timeToY(at(3), scale) < 0);
});

test("timeToY e yToTime são inversas", () => {
  for (const hour of [6, 7, 9, 13, 18, 23]) {
    const y = timeToY(at(hour, 30), scale);
    const back = yToTime(y, day, scale);
    assert.equal(toWallClockOf(back), toWallClockOf(at(hour, 30)), `hora ${hour}:30`);
  }
});

function toWallClockOf(value: number): number {
  // Compara pela hora local, que é o que a grade enxerga.
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone: ZONE,
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).formatToParts(new Date(value));
  const hour = parts.find((part) => part.type === "hour")?.value ?? "0";
  const minute = parts.find((part) => part.type === "minute")?.value ?? "0";
  return Number(hour) * 60 + Number(minute);
}

test("o deslocamento de fuso não move o item: 09:00 continua 09:00", () => {
  // Brasil não observa horário de verão, então o caso é o inverso: a data em que
  // o país está em horário de verão (1999-02-21) tinha DST, e foi aí que a
  // aritmética em UTC colocava o item uma hora fora.
  const dstDay = fromPlainDate(plainDate("1999-02-21"), ZONE);
  const nine = instant(fromPlainDate(plainDate("1999-02-21"), ZONE) + 9 * 3_600_000) as never;
  assert.equal(toWallClockOf(nine), 540);
  const y = timeToY(nine, scale);
  const back = yToTime(y, dstDay, scale);
  assert.equal(toWallClockOf(back), 540);
});

test("snapTime encaixa em 15 minutos", () => {
  // A fronteira é 7min30, não a hora cheia: 8:07 cai em 8:00 e 8:08 cai em 8:15.
  assert.equal(toWallClockOf(snapTime(at(8, 7), scale)), 8 * 60);
  assert.equal(toWallClockOf(snapTime(at(8, 8), scale)), 8 * 60 + 15);
  assert.equal(toWallClockOf(snapTime(at(8, 22), scale)), 8 * 60 + 15);
  assert.equal(toWallClockOf(snapTime(at(8, 23), scale)), 8 * 60 + 30);
});

test("snapTime usa a escala, e não um passo fixo de 15 min", () => {
  const coarse = { ...scale, snapMinutes: minutes(60) };
  assert.equal(toWallClockOf(snapTime(at(8, 40), coarse)), 9 * 60);
});

// ---------------------------------------------------------------------------
// `snapY` — o encaixe em pixel
//
// Este bloco existe porque `snapY` esteve **errado** e nada pegou: ele assumia
// 4 pixels por minuto, o que só acerta com `snapMinutes: 15` **e**
// `hourHeight: 60`. Com o 56px de D16 o passo real é 14px e o código usava
// 60px — 4,3× longe. Sem arraste, a função não era chamada por ninguém, então
// não havia sintoma; só explodiria no primeiro gesto.
// ---------------------------------------------------------------------------

test("snapY encaixa no passo que a escala define, não num literal", () => {
  // 15 min a 56px/h = 14px. 6px e 7px ficam dentro da meia-banda do primeiro
  // passo; 8px já passou dela e cai em 14px.
  assert.equal(snapY(0, scale), 0);
  assert.equal(snapY(6, scale), 0);
  assert.equal(snapY(8, scale), 14);
  assert.equal(snapY(20, scale), 14);
  assert.equal(snapY(21, scale), 28);
  assert.equal(snapY(42, scale), 42);
});

test("snapY respeita hourHeight: 84px muda o passo, não o horário", () => {
  const wide = { ...scale, hourHeight: 84 };
  // 15 min a 84px/h = 21px.
  assert.equal(snapY(10, wide), 0);
  assert.equal(snapY(11, wide), 21);
  assert.equal(snapY(42, wide), 42);
});

test("snapY é o mesmo passo do snapTime quando o pixel volta a ser horário", () => {
  // A razão de `snapY` existir é não divergir de `snapTime`. Se as duas
  // divergirem, o arraste grava uma hora que não é a que a grade mostra.
  for (const y of [0, 8, 33, 100, 420, 700]) {
    const snappedY = snapY(y, scale);
    const viaPixel = yToTime(snappedY, day, scale);
    const viaInstant = snapTime(yToTime(y, day, scale), scale);
    assert.equal(toWallClockOf(viaPixel), toWallClockOf(viaInstant), `y=${y}`);
  }
});

test("snapY é monotônico", () => {
  let previous = -Infinity;
  for (let y = -200; y <= 1400; y += 3) {
    const snapped = snapY(y, scale);
    assert.ok(snapped >= previous, `snapY(${y}) = ${snapped} regrediu de ${previous}`);
    previous = snapped;
  }
});

test("snapY sobrevive a passo zero, sem devolver NaN", () => {
  const degenerate = { ...scale, snapMinutes: minutes(0) };
  const snapped = snapY(30, degenerate);
  assert.ok(Number.isFinite(snapped));
});

test("clampY recorta o ponteiro na faixa visível, sem virar dia seguinte", () => {
  const ceiling = visibleHeight(scale);
  assert.equal(clampY(-500, scale), 0);
  assert.equal(clampY(0, scale), 0);
  assert.equal(clampY(ceiling, scale), ceiling);
  assert.equal(clampY(99_999, scale), ceiling);
  // O topo é `dayStartHour` (06:00) e o piso é o **fim** do dia pedido — a
  // meia-noite que encerra 10/03, que é a última faixa visível da coluna. Não é
  // 01:00 do dia seguinte: um `y` sem recorte passaria direto por ela.
  assert.equal(toWallClockOf(yToTime(clampY(-1, scale), day, scale)), 6 * 60);
  assert.equal(toPlainDate(yToTime(clampY(ceiling + 900, scale), day, scale), ZONE), "2026-03-11");
  assert.equal(toWallClockOf(yToTime(clampY(ceiling + 900, scale), day, scale)), 0);
});

test("placeInDay dá a posição e a altura de um item no dia", () => {
  const placed = placeInDay(at(9), at(10, 30), day, scale);
  assert.equal(placed.length, 1);
  assert.equal(placed[0]?.top, 3 * 56);
  // 1h30 × 56px, e não 90: a altura vem da escala, não do pixel.
  assert.equal(placed[0]?.height, 84);
  assert.equal(placed[0]?.continued, false);
  assert.equal(placed[0]?.continues, false);
});

test("placeInDay devolve pedaço para o dia seguinte quando o item atravessa a meia-noite", () => {
  // 23:00 até 07:00 do dia seguinte: atravessa e ainda cai dentro da faixa visível
  // do segundo dia, que começa às 06:00.
  const start = at(23);
  const end = instant(start + 8 * 3_600_000) as never;

  const first = placeInDay(start, end, day, scale);
  assert.equal(first.length, 1);
  assert.equal(first[0]?.top, 17 * 56);
  assert.equal(first[0]?.continues, true);
  assert.equal(first[0]?.continued, false);

  const nextDay = addDaysInTz(day, 1, ZONE);
  const second = placeInDay(start, end, nextDay, scale);
  assert.equal(second.length, 1);
  assert.equal(second[0]?.top, 0);
  assert.equal(second[0]?.height, 56);
  assert.equal(second[0]?.continued, true);
});

test("placeInDay não desenha o pedaço que termina fora da faixa visível", () => {
  // Termina às 02:00, antes das 06:00 do dia seguinte: não há coluna para ele.
  const start = at(23);
  const end = instant(start + 3 * 3_600_000) as never;
  const nextDay = addDaysInTz(day, 1, ZONE);
  assert.deepEqual(placeInDay(start, end, nextDay, scale), []);
});

test("um item que termina exatamente à meia-noite não continua", () => {
  // `addDaysInTz` preserva a hora de parede, então a meia-noite do dia seguinte
  // se obtém do início do dia, e não adicionando um dia às 23:00.
  const midnight = addDaysInTz(startOfDay(day, ZONE), 1, ZONE);
  const placed = placeInDay(at(23), midnight, day, scale);
  assert.equal(placed.length, 1);
  assert.equal(placed[0]?.continues, false);
  assert.equal(placed[0]?.top, 17 * 56);
  assert.equal(placed[0]?.height, 56);
});

test("placeInDay devolve vazio para um item de outro dia", () => {
  const other = addDaysInTz(day, 3, ZONE);
  assert.deepEqual(placeInDay(at(9), at(10), other, scale), []);
});

test("um item de poucos minutos continua clicável", () => {
  const placed = placeInDay(at(9), at(9, 10), day, scale);
  assert.ok((placed[0]?.height ?? 0) >= 18);
});

test("columnize separa itens sobrepostos em colunas lado a lado", () => {
  const items = [
    { id: "a", from: 0, to: 100 },
    { id: "b", from: 10, to: 20 },
    { id: "c", from: 200, to: 300 },
  ];
  const out = columnize(items, (item) => ({ from: item.from, to: item.to }));
  const by = new Map(out.map((entry) => [entry.item.id, entry]));

  // "a" e "b" se sobrepõem: duas colunas. "c" não toca nenhum: volta a ser única.
  assert.equal(by.get("a")?.columns, 2);
  assert.equal(by.get("b")?.columns, 2);
  assert.notEqual(by.get("a")?.column, by.get("b")?.column);
  assert.equal(by.get("c")?.columns, 1);
});

test("columnize empilha três itens que se sobrepõem", () => {
  const items = [
    { id: "a", from: 0, to: 100 },
    { id: "b", from: 10, to: 90 },
    { id: "c", from: 20, to: 80 },
  ];
  const out = columnize(items, (item) => ({ from: item.from, to: item.to }));
  assert.equal(out.length, 3);
  for (const entry of out) assert.equal(entry.columns, 3);
  assert.equal(new Set(out.map((entry) => entry.column)).size, 3);
});

test("columnize devolve tudo, inclusive a lista vazia", () => {
  assert.deepEqual(
    columnize([], () => ({ from: 0, to: 0 })),
    [],
  );
});

test("visibleHeight cobre da primeira hora ao fim do dia", () => {
  assert.equal(visibleHeight(scale), 18 * 56);
});
