/**
 * Apresentação da tela Faculdade (SPEC-D-010, F8).
 *
 * Aqui mora **regra de apresentação** — rótulo, formato e texto — e nada mais.
 * A média, a situação e a frequência restante vêm **derivadas do Rust**
 * (`mediaDaDisciplina`, D74/I1); se um rótulo precisasse calcular algo, seria
 * regra de domínio na interface — a mesma classe de bug de `FREQ=MONTHLY`
 * (SPEC-D-002). `situacao` é string vinda do enum do Rust e **nunca** é
 * comparada com "quanto falta" aqui: cada texto abaixo é escolha de rótulo,
 * não cálculo.
 */

import type { Disciplina, MediaResultado, Semestre } from "../data/bridge.ts";

export type Situacao = MediaResultado["situacao"];

/**
 * O rótulo humano de cada situação (SPEC-D-010 §5). `situacao` chega como
 * string do Rust e o mapeamento abaixo é total: situação desconhecida estoura
 * em vez de renderizar silêncio.
 */
const SITUACAO_ROTULO: Record<Situacao, string> = {
  "sem-notas": "sem notas",
  aprovada: "aprovada",
  "precisa-tirar": "precisa tirar",
  "so-com-exame": "só com exame",
  "nao-alcanca": "não alcança",
};

export function situacaoRotulo(situacao: Situacao): string {
  const rotulo = SITUACAO_ROTULO[situacao];
  if (rotulo === undefined) throw new Error(`situação desconhecida: ${situacao}`);
  return rotulo;
}

/** Uma casa decimal, vírgula pt-BR. `null` (sem notas) é travessão, não zero. */
export function formatarMedia(valor: number | null): string {
  if (valor === null) return "—";
  return valor.toFixed(1).replace(".", ",");
}

/** O texto de "quanto falta", quando a situação pede. `null` = não se aplica. */
export function textoNotaNecessaria(media: MediaResultado): string | null {
  if (media.situacao !== "precisa-tirar" && media.situacao !== "so-com-exame") return null;
  if (media.notaNecessariaProxima === null) return null;
  const onde = media.situacao === "precisa-tirar" ? "na próxima" : "no exame";
  return `precisa ${formatarMedia(media.notaNecessariaProxima)} ${onde}`;
}

/** A frequência em uma linha: faltas, total e o que ainda pode faltar. */
export function textoFrequencia(disciplina: Disciplina, media: MediaResultado): string | null {
  if (disciplina.totalAulas <= 0) return null;
  const restante = media.frequenciaRestante === null ? "—" : `${media.frequenciaRestante}`;
  return `${disciplina.faltas} falta${disciplina.faltas === 1 ? "" : "s"} de ${disciplina.totalAulas} · pode faltar ${restante}`;
}

/** Aberto é `encerradoEm === null` (D75: encerrar é o ato que grava a regra). */
export function semestreAberto(semestre: Semestre): boolean {
  return semestre.encerradoEm === null;
}

/** Rótulo dos cinco tipos que o Rust aceita (`TIPOS_AVALIACAO`, F5). */
export const TIPOS_AVALIACAO: ReadonlyArray<{
  readonly id: "prova" | "trabalho" | "apresentacao" | "entrega" | "outro";
  readonly rotulo: string;
}> = [
  { id: "prova", rotulo: "Prova" },
  { id: "trabalho", rotulo: "Trabalho" },
  { id: "apresentacao", rotulo: "Apresentação" },
  { id: "entrega", rotulo: "Entrega" },
  { id: "outro", rotulo: "Outro" },
];

export function tipoRotulo(tipo: string): string {
  return TIPOS_AVALIACAO.find((t) => t.id === tipo)?.rotulo ?? tipo;
}
