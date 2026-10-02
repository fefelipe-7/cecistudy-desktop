import assert from "node:assert/strict";
import { describe, it } from "node:test";

import {
  addDaysInTz,
  addMinutes,
  endOfDay,
  fromPlainDate,
  ianaTimeZone,
  instant,
  isPlainDate,
  minutes,
  minutesBetween,
  now,
  plainDate,
  startOfDay,
  toPlainDate,
  type Clock,
  type IanaTimeZone,
  type Instant,
} from "./time.ts";

const SAO_PAULO = ianaTimeZone("America/Sao_Paulo");
const NEW_YORK = ianaTimeZone("America/New_York");
const UTC = ianaTimeZone("UTC");

/** Relógio fixo: nenhum teste pode ler o relógio da máquina. */
function fixedClock(value: Instant): Clock {
  return () => value;
}

function atLocalTime(date: string, hour: number, minute: number, zone: IanaTimeZone): Instant {
  return addMinutes(fromPlainDate(plainDate(date), zone), minutes(hour * 60 + minute));
}

function toUTCDate(value: Instant): string {
  return new Date(value).toISOString().slice(0, 10);
}

describe("D9 — o relógio vem injetado", () => {
  it("now() devolve exatamente o que o Clock devolve", () => {
    const frozen = instant(1_757_000_000_000);
    assert.equal(now(fixedClock(frozen)), frozen);
  });

  it("dois Clocks diferentes produzem instantes diferentes", () => {
    const early = instant(1_757_000_000_000);
    const late = instant(1_757_000_060_000);
    assert.notEqual(now(fixedClock(early)), now(fixedClock(late)));
  });
});

describe("toPlainDate", () => {
  it("23:30 em São Paulo continua no dia anterior ao dia UTC", () => {
    const value = atLocalTime("2026-09-29", 23, 30, SAO_PAULO);

    assert.equal(toUTCDate(value), "2026-09-30");
    assert.equal(toPlainDate(value, SAO_PAULO), plainDate("2026-09-29"));
  });

  it("21:30 em São Paulo já é o dia seguinte em UTC", () => {
    const value = atLocalTime("2026-09-29", 21, 30, SAO_PAULO);

    assert.equal(toUTCDate(value), "2026-09-30");
    assert.equal(toPlainDate(value, SAO_PAULO), plainDate("2026-09-29"));
  });

  it("20:00 em Nova York já é o dia seguinte em UTC", () => {
    const value = atLocalTime("2026-09-29", 20, 0, NEW_YORK);

    assert.equal(toUTCDate(value), "2026-09-30");
    assert.equal(toPlainDate(value, NEW_YORK), plainDate("2026-09-29"));
  });

  it("meio-dia em São Paulo e em Nova York caem no mesmo dia civil", () => {
    const value = atLocalTime("2026-09-29", 12, 0, SAO_PAULO);

    assert.equal(toPlainDate(value, SAO_PAULO), plainDate("2026-09-29"));
    assert.equal(toPlainDate(value, NEW_YORK), plainDate("2026-09-29"));
    assert.equal(toPlainDate(value, UTC), plainDate("2026-09-29"));
  });

  it("o fuso da máquina não interfere: o mesmo instante é lido pelo fuso pedido", () => {
    const value = instant(Date.UTC(2026, 8, 30, 2, 30, 0));

    assert.equal(toPlainDate(value, SAO_PAULO), plainDate("2026-09-29"));
    assert.equal(toPlainDate(value, UTC), plainDate("2026-09-30"));
    assert.equal(toPlainDate(value, NEW_YORK), plainDate("2026-09-29"));
  });
});

describe("fromPlainDate", () => {
  it("meia-noite local é o instante da meia-noite no fuso", () => {
    const value = fromPlainDate(plainDate("2026-09-29"), SAO_PAULO);

    assert.equal(toPlainDate(value, SAO_PAULO), plainDate("2026-09-29"));
    assert.equal(new Date(value).toISOString(), "2026-09-29T03:00:00.000Z");
  });

  it("funciona em fuso sem offset (UTC)", () => {
    const value = fromPlainDate(plainDate("2026-01-01"), UTC);

    assert.equal(new Date(value).toISOString(), "2026-01-01T00:00:00.000Z");
  });

  it("ida e volta preservam a data civil", () => {
    for (const date of ["2026-01-01", "2026-02-28", "2026-12-31", "2028-02-29"]) {
      assert.equal(toPlainDate(fromPlainDate(plainDate(date), SAO_PAULO), SAO_PAULO), date);
    }
  });
});

describe("startOfDay e endOfDay", () => {
  it("startOfDay volta ao começo do dia local", () => {
    const value = atLocalTime("2026-09-29", 23, 30, SAO_PAULO);
    const start = startOfDay(value, SAO_PAULO);

    assert.equal(toPlainDate(start, SAO_PAULO), plainDate("2026-09-29"));
    assert.ok(start < value);
    assert.equal(new Date(start).toISOString(), "2026-09-29T03:00:00.000Z");
  });

  it("endOfDay é a meia-noite do dia seguinte, exclusive", () => {
    const value = atLocalTime("2026-09-29", 12, 0, SAO_PAULO);
    const end = endOfDay(value, SAO_PAULO);

    assert.equal(toPlainDate(end, SAO_PAULO), plainDate("2026-09-30"));
    assert.equal(end - startOfDay(value, SAO_PAULO), 24 * 60 * 60 * 1000);
  });

  it("startOfDay sobrevive à transição de horário de verão de Nova York", () => {
    const value = atLocalTime("2026-03-08", 14, 0, NEW_YORK);
    const start = startOfDay(value, NEW_YORK);

    assert.equal(toPlainDate(start, NEW_YORK), plainDate("2026-03-08"));
    assert.ok(start <= value);
    assert.equal(new Date(start).toISOString(), "2026-03-08T05:00:00.000Z");
  });

  it("o dia da mudança de horário ainda tem 23 horas em Nova York", () => {
    const value = atLocalTime("2026-03-08", 0, 0, NEW_YORK);
    const end = endOfDay(value, NEW_YORK);

    assert.equal((end - startOfDay(value, NEW_YORK)) / 3_600_000, 23);
  });
});

describe("addDaysInTz", () => {
  it("atravessa a virada de mês mantendo a hora local", () => {
    const value = atLocalTime("2026-09-30", 18, 45, SAO_PAULO);
    const next = addDaysInTz(value, 1, SAO_PAULO);

    assert.equal(toPlainDate(next, SAO_PAULO), plainDate("2026-10-01"));
    assert.equal(wallTimeOf(next, SAO_PAULO), "18:45");
  });

  it("atravessa a virada de ano", () => {
    const value = atLocalTime("2026-12-31", 9, 0, SAO_PAULO);

    assert.equal(toPlainDate(addDaysInTz(value, 1, SAO_PAULO), SAO_PAULO), plainDate("2027-01-01"));
  });

  it("voltar um dia é o inverso de avançar um dia", () => {
    const value = atLocalTime("2026-03-10", 7, 30, SAO_PAULO);

    assert.equal(
      toPlainDate(addDaysInTz(addDaysInTz(value, 1, SAO_PAULO), -1, SAO_PAULO), SAO_PAULO),
      plainDate("2026-03-10"),
    );
  });

  it("preserva a hora local no dia em que Nova York adianta o relógio", () => {
    const value = atLocalTime("2026-03-07", 10, 0, NEW_YORK);
    const next = addDaysInTz(value, 1, NEW_YORK);

    assert.equal(toPlainDate(next, NEW_YORK), plainDate("2026-03-08"));
    assert.equal(wallTimeOf(next, NEW_YORK), "10:00");
  });
});

function wallTimeOf(value: Instant, zone: IanaTimeZone): string {
  const parts = new Intl.DateTimeFormat("en-GB", {
    timeZone: zone,
    hourCycle: "h23",
    hour: "2-digit",
    minute: "2-digit",
  }).formatToParts(new Date(value));
  const read = (type: Intl.DateTimeFormatPartTypes): string =>
    parts.find((part) => part.type === type)?.value ?? "?";
  return `${read("hour")}:${read("minute")}`;
}

describe("minutesBetween e addMinutes", () => {
  it("minutesBetween tem sinal: mais_tarde contra mais_cedo dá positivo", () => {
    const eight = atLocalTime("2026-09-29", 8, 0, SAO_PAULO);
    const ten = atLocalTime("2026-09-29", 10, 0, SAO_PAULO);

    assert.equal(minutesBetween(ten, eight), 120);
    assert.equal(minutesBetween(eight, ten), -120);
  });

  it("minutesBetween do mesmo instante é zero, não null", () => {
    const eight = atLocalTime("2026-09-29", 8, 0, SAO_PAULO);

    assert.equal(minutesBetween(eight, eight), 0);
  });

  it("addMinutes é o inverso de minutesBetween", () => {
    const eight = atLocalTime("2026-09-29", 8, 0, SAO_PAULO);

    assert.equal(addMinutes(eight, minutes(120)), atLocalTime("2026-09-29", 10, 0, SAO_PAULO));
    assert.equal(minutesBetween(addMinutes(eight, minutes(90)), eight), 90);
  });

  it("aceita durações negativas, que é como se representa um atraso", () => {
    const eight = atLocalTime("2026-09-29", 8, 0, SAO_PAULO);

    assert.equal(addMinutes(eight, minutes(-30)), atLocalTime("2026-09-29", 7, 30, SAO_PAULO));
  });
});

describe("guardas de construção", () => {
  it("plainDate recusa formatos fora de YYYY-MM-DD", () => {
    assert.ok(isPlainDate("2026-09-29"));
    assert.ok(!isPlainDate("2026-9-29"));
    assert.ok(!isPlainDate("29/09/2026"));
    assert.ok(!isPlainDate(""));
    assert.throws(() => plainDate("2026-9-29"), TypeError);
  });

  it("plainDate recusa data que não existe no calendário", () => {
    assert.ok(!isPlainDate("2026-02-30"));
    assert.ok(!isPlainDate("2026-13-01"));
    assert.ok(isPlainDate("2028-02-29"));
    assert.throws(() => plainDate("2026-02-30"), TypeError);
  });

  it("ianaTimeZone recusa fuso que o Intl não conhece", () => {
    assert.equal(ianaTimeZone("America/Sao_Paulo"), "America/Sao_Paulo");
    assert.throws(() => ianaTimeZone("São_Paulo"), TypeError);
  });

  it("instant e minutes recusam valores não finitos", () => {
    assert.throws(() => instant(Number.NaN), TypeError);
    assert.throws(() => minutes(Number.POSITIVE_INFINITY), TypeError);
  });
});
