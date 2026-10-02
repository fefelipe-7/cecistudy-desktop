/**
 * Formatadores de data da tela do Calendário (DP2).
 *
 * `Intl.DateTimeFormat` é caro de construir: cada `new` parses o locale e monta
 * a tabela de formatos, e o `CalendarScreen` chamava `new Intl.DateTimeFormat`
 * **dentro de corpo de render** — seis lugares, um deles por item desenhado.
 * Com 200 ocorrências na semana eram ~200 construções por quadro, mais 8
 * `formatToParts` por item em `placeInDay`.
 *
 * A correção não é um `useMemo`: é reconhecer `Intl` como **infraestrutura**, e
 * infraestrutura é singleton de módulo. Cada par (locale, opções) tem exatamente
 * um formatador, criado na primeira chamada e reutilizado para sempre.
 *
 * Isto é cache de plataforma, não memoização de domínio (`AGENTS.md:17`): nenhuma
 * regra do Calendário mora aqui, e o `domain/` não importa este arquivo.
 */

import { CALENDAR_ZONE } from "../domain/zone.ts";
import type { Instant } from "../domain/time.ts";

const formatterCache = new Map<string, Intl.DateTimeFormat>();

function formatter(locale: string, options: Intl.DateTimeFormatOptions, timeZone: string) {
  const key = `${locale}|${timeZone}|${JSON.stringify(options)}`;
  const cached = formatterCache.get(key);
  if (cached !== undefined) return cached;
  const created = new Intl.DateTimeFormat(locale, { ...options, timeZone });
  formatterCache.set(key, created);
  return created;
}

// Os cinco pares (formato, fuso) que a tela usa. Fixos no módulo: nenhum deles
// depende de prop, então não há nada a reconstruir quando a tela renderiza.
const zone = CALENDAR_ZONE;

/** `HH:MM` — a hora que a grade e a Agenda mostram. `hour12: false` evita AM/PM. */
const clock = formatter("pt-BR", { hour: "2-digit", minute: "2-digit", hour12: false }, zone);

/** `seg`, `ter`, … — o cabeçalho de dias da Semana e o dia da Agenda. */
const weekdayShort = formatter("pt-BR", { weekday: "short" }, zone);

/** `segunda-feira, 10 de março` — a linha de título da Agenda. */
const weekdayLong = formatter("pt-BR", { weekday: "long", day: "2-digit", month: "long" }, zone);

/** `março de 2026` — o título do modo Mês. */
const monthYear = formatter("pt-BR", { month: "long", year: "numeric" }, zone);

/** `10 de março de 2026` — o título dos modos que mostram um dia só. */
const longDay = formatter("pt-BR", { day: "2-digit", month: "long", year: "numeric" }, zone);

export function formatClock(value: Instant): string {
  return clock.format(new Date(value));
}

export function formatWeekdayShort(value: Instant): string {
  return weekdayShort.format(new Date(value));
}

export function formatWeekdayLong(value: Instant): string {
  return weekdayLong.format(new Date(value));
}

export function formatMonthYear(value: Instant): string {
  return monthYear.format(new Date(value));
}

export function formatLongDay(value: Instant): string {
  return longDay.format(new Date(value));
}

/**
 * Rótulos de hora da coluna de horas: `06:00`, `07:00`, … `23:00`.
 *
 * Não passa por `Intl` de propósito: a hora do eixo **não é um horário**. Ela é a
 * posição de uma faixa na grade, e o fuso do eixo é o fuso da grade. Passar por
 * `Intl` exigiria fabricar um instante para cada rótulo e fixar `timeZone: "UTC"`
 * para desfazer o deslocamento — mais uma fonte de grammática de hora, e não
 * menos. `formatClock` e isto precisam concordar, e concordam: `HH:MM` com dois
 * dígitos nos dois lugares.
 */
export const hourLabels: readonly string[] = Array.from(
  { length: 24 },
  (_, hour) => `${String(hour).padStart(2, "0")}:00`,
);
