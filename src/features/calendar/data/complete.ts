/**
 * Concluir um item da grade (INV-3 · item D1.1).
 *
 * A regra é do Rust: `complete_item` grava o `execution_record` e reflete o
 * estado no alvo, recusando antes de qualquer escrita quando o registro falta.
 * Este módulo só monta o payload — e existe separado da tela para ser testável
 * sem React e sem Tauri, do mesmo jeito que `calendar-store.ts`.
 *
 * Ele existe porque antes a tela chamava `set_occurrence_state(id, "concluido")`,
 * que muda o estado **sem** histórico. Concluir exige histórico: é o que a
 * INV-3 proíbe faltar.
 */

import {
  completeItem,
  type ExecutionInput,
  type ExecutionTargetWire,
  type GridItem,
} from "./bridge.ts";
import { minutes } from "../domain/time.ts";
import type { Instant } from "../domain/types.ts";

/** O item da grade é sempre uma ocorrência materializada, então o alvo é ela. */
const GRID_TARGET_KIND = "occurrence" as const;

export interface CompleteDeps {
  readonly completeItem: (target: ExecutionTargetWire, record: ExecutionInput) => Promise<void>;
}

/**
 * O registro do que foi feito, derivado do item e do instante da conclusão.
 *
 * `startedAt` é o início do item; `finishedAt` é o relógio da conclusão. A
 * duração é o intervalo entre os dois, no mínimo 1 minuto, para não gravar um
 * registro de duração zero quando a usuária conclui antes de o item começar.
 */
export function executionOf(item: GridItem, finishedAt: Instant): ExecutionInput {
  const duracaoMin = Math.max(1, Math.round((finishedAt - item.startsAt) / 60_000));
  return {
    startedAt: item.startsAt,
    finishedAt,
    actualDuration: minutes(duracaoMin),
    result: "concluido",
    source: "manual",
  };
}

/**
 * Conclui o item da grade pelo caminho que grava o `execution_record`.
 *
 * O `finishedAt` é parâmetro para o relógio ficar injetável — a tela passa
 * `instant(Date.now())`, e o teste passa um instante fixo.
 */
export async function completeGridItem(
  deps: CompleteDeps,
  item: GridItem,
  finishedAt: Instant,
): Promise<void> {
  await deps.completeItem({ kind: GRID_TARGET_KIND, id: item.id }, executionOf(item, finishedAt));
}
