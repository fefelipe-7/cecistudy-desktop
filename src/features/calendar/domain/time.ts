/**
 * D9 — vocabulário de tempo do Calendário.
 *
 * Este é o único módulo de `domain/` autorizado a usar `Date` nativo. Todo o resto
 * do domínio trabalha com `Instant` (epoch ms em UTC, brandado), `PlainDate`
 * (`YYYY-MM-DD`), `Minutes` e `IanaTimeZone`. O relógio é sempre injetado, para que
 * nenhum teste dependa do fuso ou do horário da máquina.
 */

export type Instant = number & { readonly __brand: "Instant" };
export type PlainDate = string & { readonly __brand: "PlainDate" };
export type Minutes = number & { readonly __brand: "Minutes" };
export type IanaTimeZone = string & { readonly __brand: "IanaTimeZone" };

export type Clock = () => Instant;

const MS_PER_MINUTE = 60_000;
const MS_PER_DAY = 86_400_000;

const PLAIN_DATE_PATTERN = /^(\d{4})-(\d{2})-(\d{2})$/;

export function instant(epochMs: number): Instant {
  if (!Number.isFinite(epochMs)) {
    throw new TypeError(`instant exige um número finito, recebeu ${String(epochMs)}`);
  }
  return epochMs as Instant;
}

export function minutes(value: number): Minutes {
  if (!Number.isFinite(value)) {
    throw new TypeError(`minutes exige um número finito, recebeu ${String(value)}`);
  }
  return value as Minutes;
}

export function ianaTimeZone(zone: string): IanaTimeZone {
  if (!isValidTimeZone(zone)) {
    throw new TypeError(`fuso IANA inválido: ${zone}`);
  }
  return zone as IanaTimeZone;
}

export function isValidTimeZone(zone: string): boolean {
  try {
    new Intl.DateTimeFormat("en-US", { timeZone: zone });
    return true;
  } catch {
    return false;
  }
}

export function plainDate(value: string): PlainDate {
  if (!isPlainDate(value)) {
    throw new TypeError(`PlainDate deve estar no formato YYYY-MM-DD, recebeu ${value}`);
  }
  return value as PlainDate;
}

export function isPlainDate(value: string): boolean {
  const match = PLAIN_DATE_PATTERN.exec(value);
  if (match === null) return false;
  const [, year, month, day] = match;
  if (year === undefined || month === undefined || day === undefined) return false;
  const probe = new Date(Date.UTC(Number(year), Number(month) - 1, Number(day)));
  return (
    probe.getUTCFullYear() === Number(year) &&
    probe.getUTCMonth() === Number(month) - 1 &&
    probe.getUTCDate() === Number(day)
  );
}

export function now(clock: Clock): Instant {
  return clock();
}

export function addMinutes(value: Instant, amount: Minutes): Instant {
  return (value + amount * MS_PER_MINUTE) as Instant;
}

/**
 * Assinatura: `minutesBetween(mais_tarde, mais_cedo)`. O resultado é **com sinal** —
 * 10:00 contra 08:00 dá 120, e 08:00 contra 10:00 dá -120.
 */
export function minutesBetween(later: Instant, earlier: Instant): Minutes {
  return minutes((later - earlier) / MS_PER_MINUTE);
}

interface WallClock {
  readonly year: number;
  readonly month: number;
  readonly day: number;
  readonly hour: number;
  readonly minute: number;
  readonly second: number;
}

const formatterCache = new Map<string, Intl.DateTimeFormat>();

function partsFormatter(timeZone: string): Intl.DateTimeFormat {
  const cached = formatterCache.get(timeZone);
  if (cached !== undefined) return cached;
  const created = new Intl.DateTimeFormat("en-US", {
    timeZone,
    hourCycle: "h23",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
  formatterCache.set(timeZone, created);
  return created;
}

function wallClockOf(value: Instant, timeZone: string): WallClock {
  const parts = partsFormatter(timeZone).formatToParts(new Date(value));
  const read = (type: Intl.DateTimeFormatPartTypes): number => {
    const found = parts.find((part) => part.type === type);
    if (found === undefined) {
      throw new RangeError(`Intl não devolveu a parte ${type} para o fuso ${timeZone}`);
    }
    return Number(found.value);
  };
  return {
    year: read("year"),
    month: read("month"),
    day: read("day"),
    // `hourCycle: "h23"` já evita o "24" que alguns runtimes devolvem com `hour12: false`.
    hour: read("hour") % 24,
    minute: read("minute"),
    second: read("second"),
  };
}

/** Deslocamento do fuso, em ms, no instante informado: `relogio local - UTC`. */
function offsetAt(value: number, timeZone: string): number {
  const local = wallClockOf(value as Instant, timeZone);
  return (
    Date.UTC(local.year, local.month - 1, local.day, local.hour, local.minute, local.second) - value
  );
}

/**
 * Resolve um "relógio de parede" (data e hora locais, sem fuso) no instante
 * correspondente. Passar duas vezes corrige a borda de horário de verão: no dia em que
 * o fuso adianta, `offsetAt` muda entre a primeira estimativa e o resultado.
 */
function resolveWallClock(wallMs: number, timeZone: string): number {
  let candidate = wallMs - offsetAt(wallMs, timeZone);
  const corrected = wallMs - offsetAt(candidate, timeZone);
  if (corrected !== candidate) candidate = corrected;
  return candidate;
}

/**
 * Converte um instante para a data civil que a usuária vê no fuso informado.
 * O resultado **não** depende do fuso da máquina.
 */
export function toPlainDate(value: Instant, timeZone: IanaTimeZone): PlainDate {
  const local = wallClockOf(value, timeZone);
  const year = String(local.year).padStart(4, "0");
  const month = String(local.month).padStart(2, "0");
  const day = String(local.day).padStart(2, "0");
  return `${year}-${month}-${day}` as PlainDate;
}

/**
 * Instante da meia-noite local do dia que contém `value`, no fuso informado.
 * Corrige o deslocamento duas vezes para sobreviver a bordas de horário de verão.
 */
export function startOfDay(value: Instant, timeZone: IanaTimeZone): Instant {
  const local = wallClockOf(value, timeZone);
  const naive = Date.UTC(local.year, local.month - 1, local.day, 0, 0, 0);
  return resolveWallClock(naive, timeZone) as Instant;
}

export function endOfDay(value: Instant, timeZone: IanaTimeZone): Instant {
  return addDaysInTz(startOfDay(value, timeZone), 1, timeZone);
}

/**
 * Soma dias **civis** no fuso, preservando a hora local. A hora é preservada como
 * relógio de parede, não como tempo decorrido: atravessar a mudança de horário de
 * verão muda o instante em 23h ou 25h, mas a hora mostrada continua a mesma.
 */
export function addDaysInTz(value: Instant, days: number, timeZone: IanaTimeZone): Instant {
  const local = wallClockOf(value, timeZone);
  const shifted = new Date(
    Date.UTC(local.year, local.month - 1, local.day + days, local.hour, local.minute, local.second),
  );
  return resolveWallClock(shifted.getTime(), timeZone) as Instant;
}

export function fromPlainDate(date: PlainDate, timeZone: IanaTimeZone): Instant {
  const [year, month, day] = plainDateToParts(date);
  return resolveWallClock(Date.UTC(year, month - 1, day, 0, 0, 0), timeZone) as Instant;
}

export function plainDateToParts(date: PlainDate): readonly [number, number, number] {
  const match = PLAIN_DATE_PATTERN.exec(date);
  if (match === null) {
    throw new TypeError(`PlainDate deve estar no formato YYYY-MM-DD, recebeu ${date}`);
  }
  const [, year, month, day] = match;
  if (year === undefined || month === undefined || day === undefined) {
    throw new TypeError(`PlainDate deve estar no formato YYYY-MM-DD, recebeu ${date}`);
  }
  return [Number(year), Number(month), Number(day)] as const;
}

/**
 * Quantos dias o mês de `date` tem: 28, 29, 30 ou 31.
 *
 * O dia do mês é uma **constante de calendário**, não um horário: por isso esta
 * função não recebe fuso e não toca em `Date`. A versão anterior da grade do mês
 * usava `new Date(ano, mes, 0)` — o dia zero do mês seguinte, resolvido no fuso
 * da máquina, o que é a regra de D6 trocada por um detalhe de implementação.
 *
 * O dia 1 normaliza antes do cálculo: `month - 1` é o mês zero-based, e o dia
 * zero de março é 28 de fevereiro (ou 29 em ano bissexto), que é exatamente a
 * resposta que se quer.
 */
export function daysInMonth(date: PlainDate): number {
  const [year, month] = plainDateToParts(date);
  const probe = new Date(Date.UTC(year, month, 0));
  return probe.getUTCDate();
}

export function toDate(value: Instant): Date {
  return new Date(value);
}

export function durationOf(startsAt: Instant, endsAt: Instant): Minutes {
  return minutesBetween(endsAt, startsAt);
}

/**
 * Despeja o instante num "relógio de parede": a data e a hora que a usuária lê no fuso,
 * expressas como se fossem UTC. É a representação que a recorrência usa, para que uma
 * regra de terça 10:00 continue terça 10:00 depois de uma viagem — D6.
 */
export function toWallClock(value: Instant, timeZone: IanaTimeZone): number {
  const local = wallClockOf(value, timeZone);
  return Date.UTC(local.year, local.month - 1, local.day, local.hour, local.minute, local.second);
}

/** O inverso de `toWallClock`: resolve um relógio de parede no fuso indicado. */
export function fromWallClock(wallMs: number, timeZone: IanaTimeZone): Instant {
  return resolveWallClock(wallMs, timeZone) as Instant;
}

export function containsInstant(startsAt: Instant, endsAt: Instant, value: Instant): boolean {
  return value >= startsAt && value < endsAt;
}

export const MS_PER_MINUTE_MS = MS_PER_MINUTE;
export const MS_PER_DAY_MS = MS_PER_DAY;
