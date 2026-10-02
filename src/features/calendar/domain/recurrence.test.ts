import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { ensureOccurrences, expandRule, type EventAnchor } from "./recurrence.ts";
import {
  addMinutes,
  fromPlainDate,
  ianaTimeZone,
  instant,
  minutes,
  plainDate,
  toPlainDate,
} from "./time.ts";
import type { Instant, IanaTimeZone } from "./time.ts";
import { eventId, recurrenceRuleId, type RecurrenceRule, type Window } from "./types.ts";

const SAO_PAULO = ianaTimeZone("America/Sao_Paulo");
const NEW_YORK = ianaTimeZone("America/New_York");
const UTC = ianaTimeZone("UTC");

const EVENT_ID = eventId("evt-1");
const RULE_ID = recurrenceRuleId("rule-1");

function atLocal(date: string, hour: number, minute: number, zone: IanaTimeZone): Instant {
  return addMinutes(fromPlainDate(plainDate(date), zone), minutes(hour * 60 + minute));
}

/** Evento âncora: 10:00 às 11:00 no fuso pedido. 2026-09-01 é uma terça-feira. */
function anchorEvent(date: string, zone: IanaTimeZone = SAO_PAULO, hour = 10): EventAnchor {
  const startsAt = atLocal(date, hour, 0, zone);
  return {
    id: EVENT_ID,
    startsAt,
    endsAt: addMinutes(startsAt, minutes(60)),
  };
}

interface RuleSeed {
  readonly until?: Instant;
  readonly count?: number;
  readonly exdates?: readonly Instant[];
  readonly dtstartTz?: IanaTimeZone;
}

function rule(rrule: string, seed: RuleSeed = {}): RecurrenceRule {
  return {
    id: RULE_ID,
    eventId: EVENT_ID,
    rrule,
    dtstartTz: seed.dtstartTz ?? SAO_PAULO,
    exdates: seed.exdates ?? [],
    createdAt: instant(0),
    updatedAt: instant(0),
    ...(seed.until !== undefined ? { until: seed.until } : {}),
    ...(seed.count !== undefined ? { count: seed.count } : {}),
  };
}

function window(from: string, to: string, zone: IanaTimeZone = SAO_PAULO): Window {
  return {
    from: fromPlainDate(plainDate(from), zone),
    to: fromPlainDate(plainDate(to), zone),
  };
}

function dates(drafts: readonly { startsAt: Instant }[], zone: IanaTimeZone = SAO_PAULO): string[] {
  return drafts.map((draft) => toPlainDate(draft.startsAt, zone));
}

function wallTime(value: Instant, zone: IanaTimeZone): string {
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

describe("Fase 1.3 (a) — semanal por BYDAY em janela parcial", () => {
  const event = anchorEvent("2026-09-01");
  const regra = rule("FREQ=WEEKLY;BYDAY=TU,TH");

  it("uma semana inteira traz terça e quinta", () => {
    assert.deepEqual(dates(expandRule(event, regra, window("2026-09-07", "2026-09-14"))), [
      "2026-09-08",
      "2026-09-10",
    ]);
  });

  it("uma janela que começa no meio do dia traz o que ainda cabe", () => {
    assert.deepEqual(dates(expandRule(event, regra, window("2026-09-08", "2026-09-10"))), [
      "2026-09-08",
    ]);
  });

  it("uma janela que começa depois da hora final não traz nada", () => {
    const meioDoDia = addMinutes(
      fromPlainDate(plainDate("2026-09-08"), SAO_PAULO),
      minutes(12 * 60),
    );
    const ateQuarta = fromPlainDate(plainDate("2026-09-10"), SAO_PAULO);
    assert.deepEqual(dates(expandRule(event, regra, { from: meioDoDia, to: ateQuarta })), []);
  });

  it("a janela é exclusiva no fim: quem começa exatamente no limite não entra", () => {
    assert.deepEqual(dates(expandRule(event, regra, window("2026-09-01", "2026-09-29"))), [
      "2026-09-01",
      "2026-09-03",
      "2026-09-08",
      "2026-09-10",
      "2026-09-15",
      "2026-09-17",
      "2026-09-22",
      "2026-09-24",
    ]);
  });

  it("sem BYDAY a âncora é o próprio dia inicial, e o INTERVAL salta de duas em duas semanas", () => {
    assert.deepEqual(
      dates(
        expandRule(
          anchorEvent("2026-09-01"),
          rule("FREQ=WEEKLY;INTERVAL=2"),
          window("2026-09-01", "2026-10-01"),
        ),
      ),
      ["2026-09-01", "2026-09-15", "2026-09-29"],
    );
  });
});

describe("Fase 1.3 (b) — COUNT limita a série", () => {
  const event = anchorEvent("2026-09-01");

  it("count=4 devolve quatro ocorrências, mesmo com a janela bem aberta", () => {
    const regra = rule("FREQ=WEEKLY;BYDAY=TU,TH", { count: 4 });
    assert.deepEqual(dates(expandRule(event, regra, window("2026-09-01", "2026-12-31"))), [
      "2026-09-01",
      "2026-09-03",
      "2026-09-08",
      "2026-09-10",
    ]);
  });

  it("um COUNT escrito na RRULE é descartado: as colunas são a única fonte", () => {
    const regra = rule("FREQ=WEEKLY;BYDAY=TU,TH;COUNT=4");
    assert.equal(expandRule(event, regra, window("2026-09-01", "2026-10-01")).length, 9);
  });

  it("a coluna count tem precedência sobre o COUNT da string", () => {
    const regra = rule("FREQ=WEEKLY;BYDAY=TU,TH;COUNT=4", { count: 2 });
    assert.deepEqual(dates(expandRule(event, regra, window("2026-09-01", "2026-12-31"))), [
      "2026-09-01",
      "2026-09-03",
    ]);
  });

  it("uma janela posterior ao fim de uma série com count não devolve nada", () => {
    const regra = rule("FREQ=WEEKLY;BYDAY=TU,TH", { count: 6 });
    assert.deepEqual(expandRule(event, regra, window("2026-12-01", "2026-12-31")), []);
  });
});

describe("Fase 1.3 (c) — UNTIL encerra a série, inclusive", () => {
  const event = anchorEvent("2026-09-01");

  it("a ocorrência que cai exatamente no until entra", () => {
    const regra = rule("FREQ=WEEKLY;BYDAY=TU,TH", {
      until: atLocal("2026-09-10", 10, 0, SAO_PAULO),
    });
    assert.deepEqual(dates(expandRule(event, regra, window("2026-09-01", "2026-12-31"))), [
      "2026-09-01",
      "2026-09-03",
      "2026-09-08",
      "2026-09-10",
    ]);
  });

  it("a ocorrência seguinte ao until fica de fora", () => {
    const regra = rule("FREQ=WEEKLY;BYDAY=TU,TH", {
      until: atLocal("2026-09-09", 23, 59, SAO_PAULO),
    });
    assert.deepEqual(dates(expandRule(event, regra, window("2026-09-01", "2026-12-31"))), [
      "2026-09-01",
      "2026-09-03",
      "2026-09-08",
    ]);
  });

  it("um UNTIL escrito na RRULE é descartado: sem a coluna, a série continua", () => {
    const regra = rule("FREQ=WEEKLY;BYDAY=TU,TH;UNTIL=20260903T130000Z");
    assert.equal(expandRule(event, regra, window("2026-09-01", "2026-10-01")).length, 9);
  });
});

describe("Fase 1.3 (d) — EXDATE não renumera as seguintes", () => {
  const event = anchorEvent("2026-09-01");

  it("count=6 com um exdate no meio devolve cinco, não seis", () => {
    const regra = rule("FREQ=WEEKLY;BYDAY=TU,TH", {
      count: 6,
      exdates: [atLocal("2026-09-10", 10, 0, SAO_PAULO)],
    });
    assert.equal(expandRule(event, regra, window("2026-09-01", "2026-12-31")).length, 5);
  });

  it("as ocorrências depois da excluída continuam nas datas delas, sem pular dia", () => {
    const regra = rule("FREQ=WEEKLY;BYDAY=TU,TH", {
      count: 6,
      exdates: [atLocal("2026-09-10", 10, 0, SAO_PAULO)],
    });
    assert.deepEqual(dates(expandRule(event, regra, window("2026-09-01", "2026-12-31"))), [
      "2026-09-01",
      "2026-09-03",
      "2026-09-08",
      "2026-09-15",
      "2026-09-17",
    ]);
  });

  it("a mesma série sem exdate devolve as seis datas", () => {
    const regra = rule("FREQ=WEEKLY;BYDAY=TU,TH", { count: 6 });
    assert.deepEqual(dates(expandRule(event, regra, window("2026-09-01", "2026-12-31"))), [
      "2026-09-01",
      "2026-09-03",
      "2026-09-08",
      "2026-09-10",
      "2026-09-15",
      "2026-09-17",
    ]);
  });

  it("um exdate fora do horário da ocorrência não exclui nada", () => {
    const regra = rule("FREQ=WEEKLY;BYDAY=TU,TH", {
      count: 6,
      exdates: [atLocal("2026-09-10", 11, 0, SAO_PAULO)],
    });
    assert.equal(expandRule(event, regra, window("2026-09-01", "2026-12-31")).length, 6);
  });
});

describe("Fase 1.3 (e) — a âncora é o fuso da regra, não o da máquina", () => {
  it("toda ocorrência de uma série semanal é 10:00 em São Paulo", () => {
    const event = anchorEvent("2026-09-01");
    const regra = rule("FREQ=WEEKLY;BYDAY=TU,TH");
    const drafts = expandRule(event, regra, window("2026-09-01", "2026-10-31"));

    assert.ok(drafts.length > 0);
    for (const draft of drafts) {
      assert.equal(wallTime(draft.startsAt, SAO_PAULO), "10:00");
      assert.equal(wallTime(draft.startsAt, UTC), "13:00");
    }
  });

  it("a série atravessa a virada de horário de verão sem deslocar a hora local", () => {
    // 2026-03-08 é o dia em que Nova York adianta o relógio.
    const event = anchorEvent("2026-03-06", NEW_YORK);
    const regra = rule("FREQ=DAILY", { count: 4, dtstartTz: NEW_YORK });
    const drafts = expandRule(event, regra, window("2026-03-06", "2026-03-12", NEW_YORK));

    assert.deepEqual(dates(drafts, NEW_YORK), [
      "2026-03-06",
      "2026-03-07",
      "2026-03-08",
      "2026-03-09",
    ]);
    for (const draft of drafts) {
      assert.equal(wallTime(draft.startsAt, NEW_YORK), "10:00");
    }
  });

  it("o resultado não depende do relógio do processo", () => {
    const event = anchorEvent("2026-09-01");
    const regra = rule("FREQ=WEEKLY;BYDAY=TU,TH");
    const primeira = expandRule(event, regra, window("2026-09-01", "2026-09-29"));

    const original = Date.now;
    Date.now = () => original() + 5_000_000_000;
    try {
      const segunda = expandRule(event, regra, window("2026-09-01", "2026-09-29"));
      assert.deepEqual(segunda, primeira);
    } finally {
      Date.now = original;
    }
  });
});

describe("Fase 1.3 (f) — ensureOccurrences é idempotente", () => {
  const event = anchorEvent("2026-09-01");
  const regra = rule("FREQ=WEEKLY;BYDAY=TU,TH", { count: 6 });
  const drafts = expandRule(event, regra, window("2026-09-01", "2026-12-31"));

  it("a primeira passada agenda todas", () => {
    assert.equal(ensureOccurrences([], drafts).length, 6);
  });

  it("a segunda passada sobre o mesmo conjunto não agenda nada", () => {
    const existing = ensureOccurrences([], drafts);
    assert.deepEqual(ensureOccurrences(existing, drafts), []);
  });

  it("uma janela seguinte só acrescenta o que é novo", () => {
    const existing = ensureOccurrences(
      [],
      expandRule(event, regra, window("2026-09-01", "2026-09-15")),
    );
    const seguinte = expandRule(event, regra, window("2026-09-01", "2026-12-31"));

    assert.deepEqual(dates(ensureOccurrences(existing, seguinte)), ["2026-09-15", "2026-09-17"]);
  });

  it("uma ocorrência remarcada continua lá e não é sobrescrita", () => {
    const original = drafts[1];
    assert.ok(original !== undefined);
    const existing = [
      {
        ...original,
        startsAt: addMinutes(original.startsAt, minutes(120)),
        override: "moved" as const,
      },
    ];
    const novos = ensureOccurrences(existing, drafts);

    const datasRestantes = drafts
      .filter((draft) => draft.originalStart !== original.originalStart)
      .map((draft) => toPlainDate(draft.originalStart, SAO_PAULO));
    assert.deepEqual(dates(novos), datasRestantes);
    assert.ok(novos.every((draft) => draft.override === "none"));
  });

  it("o rascunho nasce planejado, sem override, com a duração do evento", () => {
    for (const draft of drafts) {
      assert.equal(draft.state, "planejado");
      assert.equal(draft.override, "none");
      assert.equal(draft.endsAt - draft.startsAt, 3_600_000);
      assert.equal(draft.ruleId, RULE_ID);
      assert.equal(draft.eventId, EVENT_ID);
      assert.equal(draft.originalStart, draft.startsAt);
    }
  });
});
