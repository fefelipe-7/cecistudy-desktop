/**
 * Semente de desenvolvimento (spec 01, Fase 1.9).
 *
 * Duas guardas, e as duas importam:
 *
 * 1. A guarda de modo de desenvolvimento — em produção o Vite remove a árvore,
 *    e o bundle passa a não carregar um arquivo que inventa eventos. O critério
 *    de aceite é `rg "dev-seed" dist/assets` → vazio.
 * 2. Banco vazio — a semente roda **uma vez**. Sem esta guarda, cada recarga
 *    do dev criaria outro evento de exemplo e a tela viraria um lixo de dados
 *    repetidos; e um banco com trabalho real da usuária não pode ser tocado.
 *
 * Ela grava pelo mesmo caminho da UI (`data/bridge.ts`), e não por SQL: um
 * arquivo de seed que escreve por fora da ponte acabaria divergindo das
 * invariantes do Rust sem ninguém perceber.
 */

import {
  createResponsibility,
  createEvent,
  listEventsInWindow,
  planBlock,
  upsertRecurrence,
} from "./bridge.ts";
import { fromPlainDate, plainDate } from "../domain/time.ts";
import type { IanaTimeZone, Instant, LayerId, Minutes } from "../domain/types.ts";

/** A semente assume a camada "Faculdade" — a primeira de D4. */
const LAYER_ID: LayerId = "faculdade" as LayerId;

const ZONE: IanaTimeZone = "America/Sao_Paulo" as IanaTimeZone;

/** Segunda-feira da semana corrente, às 08:00. */
function nextMondayAt(hour: number): Instant {
  const today = new Date();
  const weekday = today.getDay();
  const offset = weekday === 0 ? 1 : 8 - weekday;
  const base = new Date(today);
  base.setDate(base.getDate() + offset);
  const date = plainDate(
    `${base.getFullYear()}-${String(base.getMonth() + 1).padStart(2, "0")}-${String(base.getDate()).padStart(2, "0")}`,
  );
  const start = fromPlainDate(date, ZONE);
  return (start + hour * 3_600_000) as Instant;
}

/**
 * Cria 1 responsabilidade, 1 regra semanal e 1 bloco.
 *
 * Devolve `false` quando não havia o que fazer — banco cheio, ou fora de `DEV` —
 * para que quem chama possa distinguir "semeado" de "pulado" sem try/catch.
 */
export async function seedIfEmpty(): Promise<boolean> {
  if (!import.meta.env.DEV) return false;

  // Janela ampla de propósito: se já existe alguma coisa no semestre, a semente
  // é pulada. Checar só a semana deixaria passar um banco com itens antigos.
  const probe = await listEventsInWindow({
    from: 0 as Instant,
    to: 253_402_300_799_000 as Instant,
  });
  if (probe.length > 0) return false;

  const startsAt = nextMondayAt(8);
  const endsAt = (startsAt + 90 * 60_000) as Instant;

  const event = await createEvent({
    layerId: LAYER_ID,
    title: "Aula de Cálculo I",
    notes: "Exemplo criado pela semente de desenvolvimento.",
    startsAt,
    endsAt,
    allDay: false,
    timeZone: ZONE,
    commitment: "obrigatorio",
    origin: "cecistudy",
  });

  // A série é criada logo depois do evento: a regra referencia `eventId`, e o
  // Rust recusa se o evento não existir ainda.
  await upsertRecurrence({
    eventId: event.id,
    rrule: "FREQ=WEEKLY;BYDAY=MO,WE",
    dtstartTz: ZONE,
  });

  const responsibility = await createResponsibility({
    layerId: LAYER_ID,
    title: "Lista 3 — exercícios 4 a 12",
    commitment: "obrigatorio",
    origin: "cecistudy",
    kind: "tarefa",
    dueAt: (nextMondayAt(23) + 6 * 86_400_000) as Instant,
    plannedDuration: 120 as Minutes,
  });

  // INV-1: o bloco **exige** a responsabilidade. Aqui está a prova de que a
  // relação é obrigatória — sem `responsibilityId` o Rust nem compila o payload.
  await planBlock({
    responsibilityId: responsibility.id,
    layerId: LAYER_ID,
    startsAt: nextMondayAt(10) as Instant,
    endsAt: nextMondayAt(12) as Instant,
    timeZone: ZONE,
    plannedDuration: 120 as Minutes,
    origin: "cecistudy",
  });

  return true;
}
