import type { Instant } from "./time.ts";

export interface Interval {
  readonly startsAt: Instant;
  readonly endsAt: Instant;
}

export type OverlapGroup<T> = readonly [T, readonly T[]];

function overlaps<T extends Interval>(a: T, b: T): boolean {
  // Intervalos que só se tocam (10:00) não se sobrepõem: o fim é exclusivo.
  return a.startsAt < b.endsAt && b.startsAt < a.endsAt;
}

/**
 * D7 — sobreposição é derivada, sinalizada e nunca bloqueante. Devolve só os grupos com
 * duas ou mais caixas, na ordem em que a primeira começa; item isolado não aparece.
 * A varredura ordena por início e carrega o fim mais tarde do grupo, o que junta uma
 * cadeia a-b-c num grupo só em tempo linear.
 */
export function findOverlaps<T extends Interval>(items: readonly T[]): OverlapGroup<T>[] {
  const sorted = [...items].sort((a, b) => a.startsAt - b.startsAt || a.endsAt - b.endsAt);

  const groups: OverlapGroup<T>[] = [];
  let current: T[] = [];
  let reach: number | null = null;

  const flush = (): void => {
    const [anchor, ...rest] = current;
    if (anchor !== undefined && rest.length > 0) {
      groups.push([anchor, rest] as const);
    }
    current = [];
    reach = null;
  };

  for (const item of sorted) {
    if (reach !== null && item.startsAt >= reach) flush();
    reach = reach === null ? item.endsAt : Math.max(reach, item.endsAt);
    current.push(item);
  }
  flush();

  return groups;
}

/** Atalho booleano para o aviso "estas duas caixas se sobrepõem". */
export function overlapsPair<T extends Interval>(a: T, b: T): boolean {
  return overlaps(a, b);
}
