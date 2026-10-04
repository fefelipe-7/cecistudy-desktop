/**
 * Ponte com o backend Rust (spec 01, Fase 1.5, A5).
 *
 * Regra dura: **este arquivo não contém SQL.** Nenhum nome de tabela, nenhum
 * `SELECT`/`INSERT`. Cada função é o par exato de um `#[tauri::command]` em
 * `src-tauri/src/commands.rs`, e o payload já vem em camelCase porque o Rust
 * deserialize com `rename_all = "camelCase"`.
 *
 * Por que a ponte do Calendário vive aqui e não em `src/lib/ipc.ts`: aquele
 * arquivo é o transporte genérico e não deve conhecer domínio. Este conhece
 * `CalendarEvent`, `Responsibility` e `Window` — e é o único lugar onde o
 * formato snake_case ↔ camelCase é traduzido, de forma explícita e testável.
 */

import { invoke } from "../../../lib/ipc.ts";
import type {
  CalendarEvent,
  Commitment,
  IanaTimeZone,
  Instant,
  ItemState,
  Layer,
  LayerId,
  LayerTone,
  Minutes,
  PlainDate,
  Responsibility,
  ResponsibilityId,
  ResponsibilityKind,
  ResponsibilityStep,
  Window,
} from "../domain/types.ts";

// ---------------------------------------------------------------------------
// Formatos que chegam do Rust
// ---------------------------------------------------------------------------

/** `tone` é TEXT no banco porque `LayerTone` é `1|2|3|4|5|"neutral"`. */
interface LayerWire {
  readonly id: string;
  readonly label: string;
  readonly tone: string;
  readonly icon: string;
  readonly visible: boolean;
  readonly position: number;
}

/** Uma instância materializada, como a grade a consome. */
export interface GridItem {
  readonly id: string;
  readonly eventId: string;
  readonly layerId: string;
  readonly title: string;
  readonly startsAt: Instant;
  readonly endsAt: Instant;
  readonly state: ItemState;
  /** `override` é palavra reservada na semântica do Rust; aqui vira `overrideKind`. */
  readonly overrideKind: "none" | "cancelled" | "moved";
  readonly isRecurring: boolean;
}

interface ResponsibilityWire {
  readonly id: string;
  readonly layerId: string;
  readonly title: string;
  readonly kind: ResponsibilityKind;
  readonly commitment: Commitment;
  readonly state: ItemState;
  readonly dueAt: Instant | null;
  readonly plannedDuration: Minutes | null;
  readonly origin: string;
  readonly ownerId: string | null;
  readonly parentId: string | null;
}

/** Instante epoch ms ↔ `Instant` brandado. Só aqui o `number` vira `Instant`. */
function toInstant(value: number): Instant {
  return value as Instant;
}

function toTone(value: string): LayerTone {
  if (value === "neutral") return "neutral";
  const n = Number(value);
  if (n === 1 || n === 2 || n === 3 || n === 4 || n === 5) return n;
  // Um tom fora do enum é defeito de dado, não de chamada: melhor o fallback
  // explícito do que renderizar a camada com cor indefinida.
  return "neutral";
}

export function toLayer(wire: LayerWire): Omit<Layer, "createdAt" | "updatedAt"> {
  return {
    id: wire.id as LayerId,
    label: wire.label,
    tone: toTone(wire.tone),
    icon: wire.icon,
    visible: wire.visible,
  };
}

export function toResponsibility(wire: ResponsibilityWire): Responsibility {
  return {
    id: wire.id as ResponsibilityId,
    layerId: wire.layerId as LayerId,
    title: wire.title,
    kind: wire.kind,
    commitment: wire.commitment,
    state: wire.state,
    dueAt: wire.dueAt === null ? null : toInstant(wire.dueAt),
    plannedDuration: wire.plannedDuration === null ? null : (wire.plannedDuration as Minutes),
    origin: wire.origin as Responsibility["origin"],
    parentId: wire.parentId as ResponsibilityId | null,
    ...(wire.ownerId === null
      ? {}
      : { ownerRef: { ownerType: wire.origin as never, ownerId: wire.ownerId } }),
    createdAt: toInstant(0),
    updatedAt: toInstant(0),
  };
}

// ---------------------------------------------------------------------------
// Comandos
// ---------------------------------------------------------------------------

export async function migrate(): Promise<{ readonly version: number }> {
  return invoke<{ version: number }>("migrate");
}

export async function listLayers(): Promise<Omit<Layer, "createdAt" | "updatedAt">[]> {
  const wire = await invoke<LayerWire[]>("list_layers");
  return wire.map(toLayer);
}

export async function setLayerVisibility(id: string, visible: boolean): Promise<void> {
  await invoke<void>("set_layer_visibility", { payload: { id, visible } });
}

export async function listEventsInWindow(window: Window): Promise<GridItem[]> {
  return invoke<GridItem[]>("list_events_in_window", {
    window: { from: window.from, to: window.to },
  });
}

export interface ResponsibilityFilter {
  readonly dueBefore?: Instant | null;
  readonly state?: ItemState | null;
  readonly layerId?: LayerId | null;
}

export async function listResponsibilities(
  filter: ResponsibilityFilter = {},
): Promise<Responsibility[]> {
  const wire = await invoke<ResponsibilityWire[]>("list_responsibilities", {
    filter: {
      dueBefore: filter.dueBefore ?? null,
      state: filter.state ?? null,
      layerId: filter.layerId ?? null,
    },
  });
  return wire.map(toResponsibility);
}

export interface UpsertEventPayload {
  readonly id?: string;
  readonly layerId: string;
  readonly title: string;
  readonly notes?: string;
  readonly startsAt: Instant;
  readonly endsAt: Instant;
  readonly allDay: boolean;
  readonly plainDate?: PlainDate;
  readonly timeZone: IanaTimeZone;
  readonly location?: string;
  readonly commitment: Commitment;
  readonly state?: ItemState;
  readonly origin: string;
  readonly ownerType?: string;
  readonly ownerId?: string;
}

/**
 * Campos ausentes não são enviados como `undefined`. O Rust distingue
 * `Some(None)` de `None`, e `JSON.stringify` já remove `undefined` — então
 * espalhar o objeto omite a chave e o Rust lê `None`. É isso que faz o patch
 * parcial não sobrescrever o que não foi pedido.
 */
export async function createEvent(payload: UpsertEventPayload): Promise<CalendarEvent> {
  return invoke<CalendarEvent>("create_event", { payload: { ...payload } });
}

export type EventPatch = Partial<Omit<UpsertEventPayload, "id">>;

export async function updateEvent(id: string, patch: EventPatch): Promise<CalendarEvent> {
  return invoke<CalendarEvent>("update_event", { payload: { id, patch: { ...patch } } });
}

export async function deleteEvent(id: string): Promise<void> {
  await invoke<void>("delete_event", { payload: { id } });
}

export async function setOccurrenceState(
  id: string,
  state: ItemState,
  reason?: string,
): Promise<void> {
  await invoke("set_occurrence_state", {
    payload: { id, state, ...(reason === undefined ? {} : { reason }) },
  });
}

export async function moveOccurrence(
  id: string,
  startsAt: Instant,
  endsAt: Instant,
): Promise<void> {
  await invoke("move_occurrence", { payload: { id, startsAt, endsAt } });
}

export type RecurrenceScope = "serie" | "esta_ocorrencia" | "esta_e_seguintes";

/**
 * Regra de recorrência, exatamente como o Rust a expande.
 *
 * `untilAt` e `count` são colunas e **vencem** `rrule` — o backend foi escrito
 * para que editar a string não apague o limite que a usuária gravou. `exdates`
 * é uma lista de `Instant`, não uma string RRULE.
 */
export interface UpsertRecurrencePayload {
  readonly id?: string;
  readonly eventId: string;
  readonly rrule: string;
  readonly dtstartTz: IanaTimeZone;
  readonly untilAt?: Instant | null;
  readonly count?: number | null;
  readonly exdates?: readonly Instant[] | null;
}

export async function upsertRecurrence(payload: UpsertRecurrencePayload): Promise<void> {
  await invoke<void>("upsert_recurrence", {
    payload: {
      ...payload,
      // O Rust guarda `exdates` como JSON de epoch ms; converter aqui mantém o
      // formato de domínio (`Instant`) longe do formato de armazenamento.
      exdatesJson:
        payload.exdates === undefined || payload.exdates === null
          ? null
          : JSON.stringify(payload.exdates),
    },
  });
}

export async function updateRecurrence(
  ruleId: string,
  rrule: string,
  scope: RecurrenceScope,
  occurrenceId?: string,
): Promise<void> {
  await invoke("update_recurrence", {
    payload: {
      ruleId,
      rrule,
      scope,
      // Os escopos locais não identificam a ocorrência sem isto; o Rust recusa
      // em vez de adivinhar qual instância é "esta".
      ...(occurrenceId === undefined ? {} : { occurrenceId }),
    },
  });
}

/**
 * Payload de responsabilidade.
 *
 * Antes isto era `Omit<UpsertEventPayload, "allDay" | "startsAt" | "endsAt">`, o
 * que obrigava a caller a mandar `timeZone` e `location` — campos que não existem
 * em `responsibility`. O `serde` do Rust os ignoraria em silêncio, então o erro
 * ficava invisível até alguém notar que a responsabilidade "aceitou" um fuso que
 * não influence em nada. O tipo agora é o que o Rust realmente lê.
 */
export interface UpsertResponsibilityPayload {
  readonly id?: string;
  readonly layerId: string;
  readonly title: string;
  readonly notes?: string;
  readonly commitment: Commitment;
  readonly state?: ItemState;
  readonly origin: string;
  readonly ownerType?: string;
  readonly ownerId?: string;
  readonly kind: ResponsibilityKind;
  readonly dueAt: Instant | null;
  readonly plannedDuration?: Minutes | null;
  readonly parentId?: ResponsibilityId | null;
}

export async function createResponsibility(
  payload: UpsertResponsibilityPayload,
): Promise<Responsibility> {
  return invoke<Responsibility>("create_responsibility", { payload: { ...payload } });
}

export async function setStepState(
  id: string,
  state: ResponsibilityStep["state"],
): Promise<ResponsibilityStep> {
  return invoke<ResponsibilityStep>("set_step_state", { payload: { id, state } });
}

export interface UpsertBlockPayload {
  readonly id?: string;
  readonly responsibilityId: string;
  readonly layerId: string;
  readonly startsAt: Instant;
  readonly endsAt: Instant;
  readonly timeZone: IanaTimeZone;
  readonly plannedDuration: Minutes;
  readonly state?: "planejado" | "em_andamento" | "concluido" | "nao_realizado";
  readonly origin: string;
}

export async function planBlock(payload: UpsertBlockPayload): Promise<void> {
  await invoke("plan_block", { payload: { ...payload } });
}

export async function rescheduleBlock(
  id: string,
  startsAt: Instant,
  endsAt: Instant,
): Promise<void> {
  await invoke("reschedule_block", { payload: { id, startsAt, endsAt } });
}

export interface ExecutionInput {
  readonly startedAt: Instant;
  readonly finishedAt: Instant;
  readonly actualDuration: Minutes;
  readonly result?: "concluido" | "parcial";
  readonly notes?: string;
  readonly source: "manual" | "timer";
}

export interface ExecutionTargetWire {
  readonly kind: "event" | "occurrence" | "responsibility" | "block";
  readonly id: string;
}

/** INV-3 — `record` é obrigatório: sem ele o Rust recusa e nada é escrito. */
export async function completeItem(
  target: ExecutionTargetWire,
  record: ExecutionInput,
): Promise<void> {
  await invoke("complete_item", { payload: { target, record } });
}

export async function recordExecution(
  target: ExecutionTargetWire,
  record: ExecutionInput,
): Promise<void> {
  await invoke("record_execution", { payload: { target, record } });
}

/**
 * Transição de estado que **não** é conclusão.
 *
 * O Rust recusa `concluido` por aqui (INV-3) e recusa transição fora da tabela
 * de §3.1. Esta função não decide nada disso: quem decide é o backend, e o
 * erro volta como rejeição da promessa.
 */
export async function setItemState(target: ExecutionTargetWire, state: ItemState): Promise<void> {
  await invoke("set_item_state", { payload: { target, state } });
}
