import { canGenerateDelay, isOpenState } from "./commitment.ts";
import type { Commitment, ItemState } from "./types.ts";
import type { Instant } from "./time.ts";

export interface OverdueSubject {
  readonly commitment: Commitment;
  readonly state: ItemState;
  readonly dueAt: Instant | null;
}

/**
 * INV-4 — o atraso é **derivado**, nunca persistido (D11). Nada mais no código compara
 * `dueAt` com o agora: `cancelado`, `dispensado` e `concluido` não atrasam mesmo com
 * prazo vencido, e um item `recomendado` ou `opcional` nunca é lido como atraso (§3).
 */
export function isOverdue(subject: OverdueSubject, now: Instant): boolean {
  const { commitment, state, dueAt } = subject;
  if (dueAt === null) return false;
  if (dueAt >= now) return false;
  if (!canGenerateDelay(commitment)) return false;
  return isOpenState(state);
}
