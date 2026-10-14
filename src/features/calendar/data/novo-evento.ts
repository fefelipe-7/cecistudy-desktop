/**
 * Tradução do formulário de criação para o payload do bridge (slice 4).
 *
 * Morar aqui, e não no componente, não é organização: é **testabilidade**. O
 * `node --test` roda `.ts` e não roda `.tsx`, então uma função de tradução dentro
 * de `NovoEvento.tsx` seria verificável só com o toolchain inteiro. E tradução de
 * "o que a usuária digitou" para instante é exatamente a parte que erra quando
 * erra — um evento gravado uma hora fora compila, passa o typecheck e aparece
 * na grade no horário errado, que é plausível o bastante para passar por escolha.
 *
 * Não é regra de domínio: não há invariante do Calendário aqui, e sim a
 * montagem de um comando. A regra de recorrência continua no Rust — estas linhas
 * produzem a *string* que o Rust interpreta, e o teste garante que a string sai
 * bem formada, não que ela signifique o que(significa).
 */

import { LAYER_IDS, isLayerId } from "../domain/layers.ts";
import { addMinutes, fromPlainDate, minutes, plainDate, type Instant } from "../domain/time.ts";
import { CALENDAR_ZONE } from "../domain/zone.ts";
import type { Commitment, LayerId } from "../domain/types.ts";
import type { UpsertEventPayload } from "./bridge.ts";

/**
 * As recorrências que a tela oferece.
 *
 * `%D` e `%2C` são **placeholders**, não texto de RFC: `%D` vira o dia que a
 * usuária digitou e `%2C` vira os dias da semana marcados. Sem eles, "todo mês"
 * seria dia 1 em silêncio e "toda semana" seria uma linha sem `BYDAY` — que é
 * exatamente o bug que o motor Rust tinha,produzindo uma ocorrência por dia.
 */
export const REPETICOES = [
  { id: "nenhuma", label: "Não repete", rrule: "" },
  { id: "diaria", label: "Todo dia", rrule: "FREQ=DAILY" },
  { id: "semanal", label: "Toda semana", rrule: "FREQ=WEEKLY;BYDAY=%2C" },
  { id: "quinzenal", label: "A cada 15 dias", rrule: "FREQ=DAILY;INTERVAL=15" },
  { id: "mensal", label: "Todo mês no dia do evento", rrule: "FREQ=MONTHLY;BYMONTHDAY=%D" },
] as const;

export type RepeticaoId = (typeof REPETICOES)[number]["id"];

/**
 * Os dias na ordem que o `BYDAY` da RFC 5545 nomeia. `MO..SU` é a ordem
 *alfabética de segunda a domingo, que é o que a RFC define — não a ordem do
 * calendário brasileiro, que começa no domingo.
 */
export const DIAS_SEMANA = [
  { valor: "MO", curto: "S" },
  { valor: "TU", curto: "T" },
  { valor: "WE", curto: "Q" },
  { valor: "TH", curto: "Q" },
  { valor: "FR", curto: "S" },
  { valor: "SA", curto: "S" },
  { valor: "SU", curto: "D" },
] as const;

export const COMPROMISSOS: readonly { readonly id: Commitment; readonly label: string }[] = [
  { id: "obrigatorio", label: "Obrigatório" },
  { id: "importante", label: "Importante" },
  { id: "recomendado", label: "Recomendado" },
  { id: "opcional", label: "Opcional" },
];

/**
 * As camadas que o formulário oferece: as do domínio **menos** a do Google.
 *
 * `google` é somente leitura (INV-2/INV-6). Oferecê-la faria o formulário montar
 * uma escrita que `guard_origin` recusa — botão que leva a erro, que é pior do
 * que botão ausente.
 */
export const CAMADAS_EDITAVEIS: readonly LayerId[] = LAYER_IDS.filter(
  (id) => isLayerId(id) && id !== "google",
);

export interface CamposNovoEvento {
  readonly titulo: string;
  /** `YYYY-MM-DD`, como vem do `<input type="date">`. */
  readonly data: string;
  /** `HH:MM`, como vem do `<input type="time">`. */
  readonly hora: string;
  readonly duracaoMin: number;
  readonly camada: LayerId;
  readonly compromisso: Commitment;
  readonly repeticao: RepeticaoId;
  readonly dias: readonly string[];
}

export interface NovoEvento {
  readonly payload: UpsertEventPayload;
  /** A linha `RRULE` pronta, ou `null` quando não repete. */
  readonly rrule: string | null;
}

export function montarNovoEvento(campos: CamposNovoEvento): NovoEvento {
  const dia = plainDate(campos.data);
  const [h, m] = campos.hora.split(":");
  const minutosDoDia = Number.parseInt(h ?? "0", 10) * 60 + Number.parseInt(m ?? "0", 10);

  const inicio = addMinutes(fromPlainDate(dia, CALENDAR_ZONE), minutes(minutosDoDia));
  const fim = addMinutes(inicio, minutes(campos.duracaoMin));

  let rrule: string | null = null;
  const base = REPETICOES.find((r) => r.id === campos.repeticao)?.rrule ?? "";
  if (base !== "") {
    const diaDoMes = Number.parseInt(dia.slice(8, 10), 10);
    rrule = base
      .replace("%D", String(diaDoMes))
      .replace("%2C", campos.dias.length > 0 ? campos.dias.join(",") : "MO");
  }

  return {
    payload: {
      layerId: campos.camada,
      title: campos.titulo.trim(),
      startsAt: inicio as Instant,
      endsAt: fim as Instant,
      allDay: false,
      plainDate: dia,
      timeZone: CALENDAR_ZONE,
      commitment: campos.compromisso,
      origin: "cecistudy",
    },
    rrule,
  };
}
