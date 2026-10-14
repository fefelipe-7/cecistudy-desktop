/**
 * Índice dia → ocorrências (A-09 · D15).
 *
 * A grade precisa de uma pergunta por dia, não por item: "o que ocupa este dia?".
 * A versão anterior respondia indexando por `item.startsAt`, o que é quase certo
 * e por isso passa no teste do dia comum — e está errado no dia em que um item
 * **atravessa a meia-noite**. Esse item é buscado pelo SQL (a janela de leitura tem
 * um dia de folga de cada ponta justamente por isso), devolvido, indexado pelo dia
 * em que **começou**… que pode não ser coluna nenhuma da tela atual. Ele existia
 * no banco, era pago na leitura, e não aparecia.
 *
 * A regra é a mesma de `placeInDay`, aplicada ao índice: um item pertence a todo
 * dia civil que ele **toca**, e só a esses. E o índice cobre a **janela inteira**,
 * não só os dias visíveis — a Agenda pergunta sobre um dia que pode estar fora da
 * semana em foco, e responder "não sei" seria o mesmo bug com outro nome.
 *
 * Puro, como o resto de `domain/`: sem React, sem `Date` direto, sem IPC. O fuso
 * entra por parâmetro — D6, sem exceção.
 */

import {
  addDaysInTz,
  startOfDay,
  toPlainDate,
  type IanaTimeZone,
  type Instant,
  type PlainDate,
} from "./time.ts";

/** O que o índice sabe sobre um item: quando começa e quando termina. */
export interface Spans {
  readonly startsAt: Instant;
  readonly endsAt: Instant;
}

/** Um `Map` de dia → itens. Imutável, para que o `memo` possa comparar identidade. */
export type DayIndex<T> = ReadonlyMap<PlainDate, readonly T[]>;

const EMPTY: readonly never[] = [];

/** Uma janela carregada. Os mesmos dois campos que a tela já calculava. */
export interface DayWindow {
  readonly from: Instant;
  readonly to: Instant;
}

/**
 * Uma ocorrência toca o dia `day` quando seu intervalo atravessa o dia inteiro:
 * começa antes do fim daquele dia e termina depois do começo dele.
 *
 * É a mesma comparação de `containsInstant`, com o dia como intervalo — meia-noite
 * do dia seguinte já pertence ao dia seguinte, e nenhum item é contado duas vezes.
 */
export function spansDay(item: Spans, day: Instant, timeZone: IanaTimeZone): boolean {
  const dayStart = startOfDay(day, timeZone);
  const dayEnd = addDaysInTz(dayStart, 1, timeZone);
  return item.startsAt < dayEnd && item.endsAt > dayStart;
}

/**
 * Indexa por dia tocado, recortado na janela.
 *
 * Percorre os **dias que cada item toca**, não os dias da janela: um item de duas
 * horas custa duas iterações mesmo numa janela de 67 dias, onde percorrer
 * "itens × dias" pagaria 200 × 67 a cada releitura.
 */
export function indexByDay<T extends Spans>(
  items: readonly T[],
  window: DayWindow,
  timeZone: IanaTimeZone,
): DayIndex<T> {
  const index = new Map<PlainDate, T[]>();
  const windowStart = startOfDay(window.from, timeZone);
  const windowEnd = startOfDay(window.to, timeZone);

  for (const item of items) {
    // Intervalo vazio ou invertido não ocupa dia nenhum. `placeInDay` já devolve
    // `[]` nesse caso (`to <= from`), e o índice precisa concordar: um item que a
    // coluna não desenha mas o índice lista é um item que aparece em `+N` no mês
    // e não existe em lugar nenhum da grade.
    if (item.endsAt <= item.startsAt) continue;

    // O item é recortado na janela antes de caminhar: a leitura traz um dia de
    // folga de cada ponta, e caminhar até lá criaria chaves que nada consulta.
    let cursor = item.startsAt < windowStart ? windowStart : startOfDay(item.startsAt, timeZone);
    const last = item.endsAt > windowEnd ? windowEnd : startOfDay(item.endsAt, timeZone);

    while (cursor <= last) {
      if (spansDay(item, cursor, timeZone)) {
        const key = toPlainDate(cursor, timeZone);
        const bucket = index.get(key);
        if (bucket !== undefined) bucket.push(item);
        else index.set(key, [item]);
      }
      cursor = addDaysInTz(cursor, 1, timeZone);
    }
  }
  return index;
}

/** O balde de um dia, ou a lista vazia **constante** — nunca um `[]` novo. */
export function itemsOn<T>(index: DayIndex<T>, key: PlainDate): readonly T[] {
  return index.get(key) ?? (EMPTY as readonly T[]);
}
