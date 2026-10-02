/**
 * Vocabulário do Calendário (spec 01 §3.1 e §3.2). Este módulo é a **fonte única**
 * dos tipos de domínio: `layers.ts` e `commitment.ts` importam daqui, nunca o inverso.
 *
 * Todo instante é `Instant` (epoch ms em UTC, brandado), nunca `Date` — o relógio
 * entra por injeção (D9). Nenhuma regra mora aqui: este arquivo só descreve dados.
 */

import type { IanaTimeZone, Instant, Minutes, PlainDate } from "./time.ts";

export type { IanaTimeZone, Instant, Minutes, PlainDate } from "./time.ts";
export type { Clock } from "./time.ts";

export type LayerId = "faculdade" | "estudos" | "tcc" | "estagio" | "conhecimento" | "google";

export type LayerTone = 1 | 2 | 3 | 4 | 5 | "neutral";

export type Origin =
  "cecistudy" | "faculdade" | "estudos" | "tcc" | "estagio" | "conhecimento" | "google";

export type Commitment = "obrigatorio" | "importante" | "recomendado" | "opcional";

export type ItemState =
  | "planejado"
  | "em_andamento"
  | "concluido"
  | "adiado"
  | "nao_realizado"
  | "cancelado"
  | "dispensado";

export type ItemId = string & { readonly __brand: "ItemId" };
export type CalendarEventId = string & { readonly __brand: "CalendarEventId" };
export type RecurrenceRuleId = string & { readonly __brand: "RecurrenceRuleId" };
export type OccurrenceId = string & { readonly __brand: "OccurrenceId" };
export type ResponsibilityId = string & { readonly __brand: "ResponsibilityId" };
export type ResponsibilityStepId = string & { readonly __brand: "ResponsibilityStepId" };
export type PlanBlockId = string & { readonly __brand: "PlanBlockId" };
export type ExecutionRecordId = string & { readonly __brand: "ExecutionRecordId" };

export interface Auditable {
  readonly id: string;
  readonly createdAt: Instant;
  readonly updatedAt: Instant;
}

/** D14 — vínculo com o módulo dono. `null` nos dois lados ou preenchido nos dois. */
export interface OwnerRef {
  readonly ownerType: Origin | "responsabilidade";
  readonly ownerId: string;
}

export interface CalendarEvent extends Auditable {
  readonly id: CalendarEventId;
  readonly layerId: LayerId;
  readonly title: string;
  readonly notes?: string;
  readonly startsAt: Instant;
  readonly endsAt: Instant;
  readonly allDay: boolean;
  readonly plainDate?: PlainDate;
  readonly timeZone: IanaTimeZone;
  readonly location?: string;
  readonly commitment: Commitment;
  readonly state: ItemState;
  readonly origin: Origin;
  readonly ownerRef?: OwnerRef;
}

export interface RecurrenceRule extends Auditable {
  readonly id: RecurrenceRuleId;
  readonly eventId: CalendarEventId;
  readonly rrule: string;
  readonly dtstartTz: IanaTimeZone;
  readonly until?: Instant;
  readonly count?: number;
  readonly exdates: readonly Instant[];
}

export type OccurrenceOverride = "none" | "cancelled" | "moved";

export interface Occurrence extends Auditable {
  readonly id: OccurrenceId;
  readonly eventId: CalendarEventId;
  /** `null` = evento avulso, sem regra. */
  readonly ruleId: RecurrenceRuleId | null;
  /** Nunca muda: é a identidade da instância junto ao Google (`originalStartTime`). */
  readonly originalStart: Instant;
  readonly startsAt: Instant;
  readonly endsAt: Instant;
  readonly state: ItemState;
  readonly override: OccurrenceOverride;
  readonly cancelReason?: string;
  readonly executionRecordId: ExecutionRecordId | null;
}

export type ResponsibilityKind =
  "tarefa" | "leitura" | "pesquisa" | "revisao" | "escrita" | "preparacao";

export interface Responsibility extends Auditable {
  readonly id: ResponsibilityId;
  readonly layerId: LayerId;
  readonly title: string;
  readonly notes?: string;
  readonly kind: ResponsibilityKind;
  readonly commitment: Commitment;
  readonly state: ItemState;
  /** D2 — um prazo principal por responsabilidade. O pai grava a soma. */
  readonly dueAt: Instant | null;
  readonly plannedDuration: Minutes | null;
  readonly objective?: string;
  readonly origin: Origin;
  readonly ownerRef?: OwnerRef;
  readonly parentId: ResponsibilityId | null;
}

export type StepState = "pendente" | "em_andamento" | "concluida";

export interface ResponsibilityStep extends Auditable {
  readonly id: ResponsibilityStepId;
  readonly responsibilityId: ResponsibilityId;
  readonly title: string;
  readonly position: number;
  readonly state: StepState;
  readonly dueAt: Instant | null;
  readonly completedAt: Instant | null;
}

/** INV-1 — `responsibilityId` é `NOT NULL` no schema: bloco sem obrigação não existe. */
export type PlanBlockState = "planejado" | "em_andamento" | "concluido" | "nao_realizado";

export interface PlanBlock extends Auditable {
  readonly id: PlanBlockId;
  readonly responsibilityId: ResponsibilityId;
  readonly layerId: LayerId;
  readonly startsAt: Instant;
  readonly endsAt: Instant;
  readonly timeZone: IanaTimeZone;
  readonly plannedDuration: Minutes;
  readonly state: PlanBlockState;
  readonly origin: Origin;
  readonly timerState: "parado" | "rodando" | "pausado";
  readonly accumulated: Minutes;
}

export type ExecutionTargetKind = "event" | "occurrence" | "responsibility" | "block";

export interface ExecutionTarget {
  readonly kind: ExecutionTargetKind;
  readonly id: string;
}

/** D12 — append-only: corrigir cria outro registro, o anterior permanece. */
export interface ExecutionRecord extends Auditable {
  readonly id: ExecutionRecordId;
  readonly target: ExecutionTarget;
  readonly startedAt: Instant;
  readonly finishedAt: Instant;
  readonly actualDuration: Minutes;
  readonly result?: "concluido" | "parcial";
  readonly notes?: string;
  readonly source: "manual" | "timer";
  readonly correctedBy: ExecutionRecordId | null;
}

export interface Layer extends Auditable {
  readonly id: LayerId;
  readonly label: string;
  readonly tone: LayerTone;
  readonly icon: string;
  readonly visible: boolean;
}

export type SyncState = "sincronizado" | "pendente" | "conflito" | "removido_remoto";

/** Schema criado na spec 01; preenchido pela spec 05. */
export interface ExternalLink {
  readonly id: string;
  readonly entityType: "event" | "occurrence" | "responsibility" | "block";
  readonly entityId: string;
  readonly system: "google";
  readonly remoteCalendarId: string;
  readonly remoteEventId: string;
  readonly remoteEtag?: string;
  readonly remoteIcalUid?: string;
  readonly lastSyncedAt: Instant;
  readonly lastSeenRemoteUpdated?: Instant;
  readonly lastChangeOrigin: "cecistudy" | "google";
  readonly syncState: SyncState;
  readonly syncFields: readonly string[];
  readonly conflictLocal?: string;
  readonly conflictRemote?: string;
}

export type SuggestionState = "pendente" | "aceita" | "recusada" | "expirada";

/** Schema criado na spec 01; a regra que produz isto é a spec 06. */
export interface BlockSuggestion extends Auditable {
  readonly id: string;
  readonly responsibilityId: ResponsibilityId;
  readonly layerId: LayerId;
  readonly startsAt: Instant;
  readonly endsAt: Instant;
  readonly rationale: string;
  readonly score: number;
  readonly state: SuggestionState;
  readonly resolvedAt: Instant | null;
  readonly producedBy: string;
}

export interface Window {
  readonly from: Instant;
  readonly to: Instant;
}

export interface UpsertEvent {
  readonly id?: CalendarEventId;
  readonly layerId: LayerId;
  readonly title: string;
  readonly notes?: string;
  readonly startsAt: Instant;
  readonly endsAt: Instant;
  readonly allDay?: boolean;
  readonly plainDate?: PlainDate;
  readonly timeZone: IanaTimeZone;
  readonly location?: string;
  readonly commitment: Commitment;
  readonly state?: ItemState;
  readonly origin: Origin;
  readonly ownerRef?: OwnerRef;
}

export interface UpsertResponsibility {
  readonly id?: ResponsibilityId;
  readonly layerId: LayerId;
  readonly title: string;
  readonly notes?: string;
  readonly kind: ResponsibilityKind;
  readonly commitment: Commitment;
  readonly state?: ItemState;
  readonly dueAt: Instant | null;
  readonly plannedDuration?: Minutes | null;
  readonly objective?: string;
  readonly origin: Origin;
  readonly ownerRef?: OwnerRef;
  readonly parentId?: ResponsibilityId | null;
}

export interface UpsertBlock {
  readonly id?: PlanBlockId;
  readonly responsibilityId: ResponsibilityId;
  readonly layerId: LayerId;
  readonly startsAt: Instant;
  readonly endsAt: Instant;
  readonly timeZone: IanaTimeZone;
  readonly plannedDuration: Minutes;
  readonly state?: PlanBlockState;
  readonly origin: Origin;
}

export function eventId(value: string): CalendarEventId {
  return value as CalendarEventId;
}

export function recurrenceRuleId(value: string): RecurrenceRuleId {
  return value as RecurrenceRuleId;
}

export function occurrenceId(value: string): OccurrenceId {
  return value as OccurrenceId;
}

export function responsibilityId(value: string): ResponsibilityId {
  return value as ResponsibilityId;
}

export function responsibilityStepId(value: string): ResponsibilityStepId {
  return value as ResponsibilityStepId;
}

export function planBlockId(value: string): PlanBlockId {
  return value as PlanBlockId;
}

export function executionRecordId(value: string): ExecutionRecordId {
  return value as ExecutionRecordId;
}
