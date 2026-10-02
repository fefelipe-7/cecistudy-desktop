import rrulePackage from "rrule";
import type { Options } from "rrule";

import { addMinutes, durationOf, fromWallClock, toWallClock } from "./time.ts";
import type { Instant } from "./time.ts";
import type {
  CalendarEvent,
  CalendarEventId,
  ItemState,
  OccurrenceOverride,
  RecurrenceRule,
  RecurrenceRuleId,
  Window,
} from "./types.ts";

const { RRule, RRuleSet } = rrulePackage;

/**
 * Uma ocorrência que ainda não foi gravada. `originalStart` é a identidade da instância:
 * ele nunca muda, e é o par que o Google Calendar chama de `recurringEventId` +
 * `originalStartTime`. `startsAt` só se afasta dele quando a usuária remarca (§5 de
 * `calendario.md`).
 */
export interface OccurrenceDraft {
  readonly eventId: CalendarEventId;
  readonly ruleId: RecurrenceRuleId;
  readonly originalStart: Instant;
  readonly startsAt: Instant;
  readonly endsAt: Instant;
  readonly state: ItemState;
  readonly override: OccurrenceOverride;
}

type EventAnchor = Pick<CalendarEvent, "id" | "startsAt" | "endsAt">;
export type { EventAnchor };

/**
 * A string RRULE carrega apenas o padrão (FREQ, INTERVAL, BYDAY, ...). Os limites da
 * série — `until`, `count` e `exdates` — vivem nas colunas da tabela `event_recurrence`,
 * porque precisam de fusos distintos e de edição individual. Os limites que existem
 * como coluna são removidos da string na leitura, para que exista uma única fonte; o
 * `exdate` nem chega a entrar nas options — exclusão é recurso do conjunto.
 *
 * A exclusão usa `RRuleSet.exdate`, que casa a data da ocorrência pelo instante exato:
 * por isso o `originalStart` das ocorrências seguintes não desliza quando um dia sai.
 */
function buildRuleSet(event: EventAnchor, rule: RecurrenceRule) {
  const options: Partial<Options> = RRule.parseString(rule.rrule);

  delete options.until;
  delete options.count;

  // A âncora é o `startsAt` do evento, lido como relógio de parede no fuso da regra.
  // É isso que mantém "terça às 10:00 em São Paulo" estável mesmo com a máquina em UTC.
  options.dtstart = new Date(toWallClock(event.startsAt, rule.dtstartTz));

  if (rule.count !== undefined) options.count = rule.count;
  if (rule.until !== undefined) {
    options.until = new Date(toWallClock(rule.until, rule.dtstartTz));
  }

  const set = new RRuleSet();
  set.rrule(new RRule(options));
  for (const value of rule.exdates) {
    set.exdate(new Date(toWallClock(value, rule.dtstartTz)));
  }
  return set;
}

/**
 * Expande a regra na janela pedida. Uma ocorrência pertence à janela quando ela
 * **intersecta** a janela — um bloco de 09:00 às 11:00 que começa antes do primeiro dia
 * visível não pode sumir no meio do scroll.
 *
 * As exclusões entram como `exdate` do próprio conjunto, nunca como filtro posterior. Por
 * isso o `originalStart` das ocorrências seguintes não desliza quando um dia é excluído.
 */
export function expandRule(
  event: EventAnchor,
  rule: RecurrenceRule,
  window: Window,
): OccurrenceDraft[] {
  const duration = durationOf(event.startsAt, event.endsAt);
  const expanded = buildRuleSet(event, rule);

  const fromWall = toWallClock(window.from, rule.dtstartTz) - duration * 60_000;
  const toWall = toWallClock(window.to, rule.dtstartTz);
  const candidates = expanded.between(new Date(fromWall), new Date(toWall), true);

  const drafts: OccurrenceDraft[] = [];
  for (const candidate of candidates) {
    const startsAt = fromWallClock(candidate.getTime(), rule.dtstartTz);
    const endsAt = addMinutes(startsAt, duration);
    if (startsAt >= window.to) continue;
    if (endsAt <= window.from) continue;
    drafts.push({
      eventId: event.id,
      ruleId: rule.id,
      originalStart: startsAt,
      startsAt,
      endsAt,
      state: "planejado",
      override: "none",
    });
  }
  return drafts;
}

/**
 * O que precisa ser gravado para a janela ficar materializada. Idempotente por
 * construção: a constraint `UNIQUE(event_id, original_start)` do banco resolve a corrida
 * entre duas leituras simultâneas da mesma janela; esta função resolve o caso comum.
 *
 * Uma ocorrência já remarcada (`override = "moved"`) ou cancelada continua no banco e
 * nunca é sobrescrita: a regra gera a mesma data de novo, mas a decisão da usuária vale.
 */
export function ensureOccurrences(
  existing: readonly { readonly originalStart: Instant }[],
  drafts: readonly OccurrenceDraft[],
): OccurrenceDraft[] {
  const taken = new Set(existing.map((item) => item.originalStart));
  const scheduled = new Set<Instant>();

  const pending: OccurrenceDraft[] = [];
  for (const draft of drafts) {
    if (taken.has(draft.originalStart)) continue;
    if (scheduled.has(draft.originalStart)) continue;
    scheduled.add(draft.originalStart);
    pending.push(draft);
  }
  return pending;
}
