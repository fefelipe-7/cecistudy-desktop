/**
 * Dica do cecinho para o desktop (`SPEC-D-014` D90).
 *
 * Determinística: a mesma situação sempre devolve a mesma dica, e as candidatas
 * são avaliadas por prioridade (a mais específica vence). Espelha
 * `cecistudy/src/lib/tips.ts`, com a voz reduzida de D4 — sem "cantinho", sem
 * diminutivos, sem emoji decorativo.
 *
 * Dado puro: recebe `now` por parâmetro, nunca chama o relógio por dentro.
 */

export interface TipContext {
  /** Flashcards com revisão vencida hoje. */
  dueCards?: number;
  /** Tarefas pendentes. */
  pendingTasks?: number;
  /** Data (YYYY-MM-DD) da próxima prova pendente, se houver. */
  nextExamDate?: string | null;
  /** Há leitura em andamento? */
  readingInProgress?: boolean;
  /** Dias ativos da sequência atual. */
  streakDays?: number;
  /** Minutos de foco registrados hoje. */
  focusMinutesToday?: number;
}

/** Dias (inteiro ≥ 0) entre hoje e a data YYYY-MM-DD; null se inválida ou passada. */
export function daysUntil(date?: string | null, now = new Date()): number | null {
  if (!date) return null;
  const target = new Date(`${date}T00:00:00`);
  if (Number.isNaN(target.getTime())) return null;
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const diff = Math.round((target.getTime() - today.getTime()) / 86400000);
  return diff >= 0 ? diff : null;
}

interface TipCandidate {
  matches: () => boolean;
  message: string;
}

export const DICA_PADRAO = "sem pressa: uma página, um cartão, um resumo por vez já move o dia.";

function candidatesFor(ctx: TipContext, now: Date): TipCandidate[] {
  const examDays = daysUntil(ctx.nextExamDate, now);
  const plural = (n: number, one: string, many: string) => (n === 1 ? one : many);

  return [
    {
      matches: () => examDays != null && examDays <= 7,
      message: "a prova está perto — revise os tópicos principais um de cada vez.",
    },
    {
      matches: () => (ctx.dueCards ?? 0) > 0,
      message: `tem ${ctx.dueCards} ${plural(ctx.dueCards ?? 0, "cartão esperando", "cartões esperando")} revisão — comece pelos rápidos.`,
    },
    {
      matches: () => (ctx.pendingTasks ?? 0) > 0,
      message: "pegue a tarefa de prazo mais próximo e resolva ela primeiro — o resto flui.",
    },
    {
      matches: () => ctx.readingInProgress === true,
      message: "alguns parágrafos da leitura em andamento por dia terminam o livro.",
    },
    {
      matches: () => (ctx.streakDays ?? 0) >= 3 && (ctx.focusMinutesToday ?? 0) === 0,
      message: `${ctx.streakDays} dias seguidos — uma sessão curta hoje mantém a sequência.`,
    },
    {
      matches: () => (ctx.streakDays ?? 0) >= 3,
      message: `${ctx.streakDays} dias seguidos de estudo. você está construindo algo consistente.`,
    },
    {
      matches: () => (ctx.focusMinutesToday ?? 0) === 0,
      message: "uma sessão leve de foco já conta. comece devagar, sem pressa.",
    },
    {
      matches: () => true,
      message: DICA_PADRAO,
    },
  ];
}

/** Dica contextual do cecinho para o momento atual (a mais específica vence). */
export function pickTip(ctx: TipContext, now = new Date()): string {
  const match = candidatesFor(ctx, now).find((c) => c.matches());
  return match?.message ?? DICA_PADRAO;
}
