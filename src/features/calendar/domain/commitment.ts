import type { Commitment, ItemState } from "./types.ts";

export const COMMITMENTS: readonly Commitment[] = [
  "obrigatorio",
  "importante",
  "recomendado",
  "opcional",
] as const;

export const COMMITMENT_LABEL: Record<Commitment, string> = {
  obrigatorio: "Obrigatório",
  importante: "Importante",
  recomendado: "Recomendado",
  opcional: "Opcional",
};

export const ITEM_STATES: readonly ItemState[] = [
  "planejado",
  "em_andamento",
  "concluido",
  "adiado",
  "nao_realizado",
  "cancelado",
  "dispensado",
] as const;

/** §4 — rótulos em português. O estado é dado; como ele se lê é apresentação. */
export const ITEM_STATE_LABEL: Record<ItemState, string> = {
  planejado: "Planejado",
  em_andamento: "Em andamento",
  concluido: "Concluído",
  adiado: "Adiado",
  nao_realizado: "Não realizado",
  cancelado: "Cancelado",
  dispensado: "Dispensado",
};

/** §3 — só `obrigatorio` e `importante` podem virar atraso. */
const DELAYABLE: ReadonlySet<Commitment> = new Set<Commitment>(["obrigatorio", "importante"]);

/** §3/§4 — os quatro estados em que a atividade ainda está em aberto. */
const OPEN_STATES: ReadonlySet<ItemState> = new Set<ItemState>([
  "planejado",
  "em_andamento",
  "adiado",
  "nao_realizado",
]);

export function canGenerateDelay(commitment: Commitment): boolean {
  return DELAYABLE.has(commitment);
}

export function isOpenState(state: ItemState): boolean {
  return OPEN_STATES.has(state);
}

export function commitmentLabel(commitment: Commitment): string {
  return COMMITMENT_LABEL[commitment];
}

export function stateLabel(state: ItemState): string {
  return ITEM_STATE_LABEL[state];
}
