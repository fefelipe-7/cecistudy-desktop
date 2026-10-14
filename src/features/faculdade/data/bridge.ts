/**
 * Ponte com o backend Rust do módulo Faculdade (SPEC-D-010, D74/D75).
 *
 * A mesma regra da ponte do Calendário: **este arquivo não contém SQL** e não
 * recalcula nada. Cada função é o par exato de um `#[tauri::command]` em
 * `src-tauri/src/commands.rs`, e o payload já vem em camelCase porque o Rust
 * deserialize com `rename_all = "camelCase"`.
 *
 * D74: a média é **derivada no Rust** (`src-tauri/src/materia/media.rs`). Se a
 * tela precisar da média, ela chama `mediaDaDisciplina` — qualquer segundo
 * cálculo aqui seria regra de domínio na interface, a mesma classe de bug de
 * `FREQ=MONTHLY` (SPEC-D-002). Os tipos abaixo são o contrato (SPEC-D-010 §5),
 * nada mais.
 */

import { invoke } from "../../../lib/ipc.ts";

export interface Semestre {
  readonly id: string;
  readonly rotulo: string;
  readonly startsOn: string;
  readonly endsOn: string;
  /** `null` = aberto. Encerrar é ato que grava a regra congelada (D75). */
  readonly encerradoEm: number | null;
}

export interface Disciplina {
  readonly id: string;
  readonly semestreId: string;
  readonly nome: string;
  readonly codigo: string | null;
  readonly professor: string | null;
  readonly cargaHoraria: number | null;
  readonly status: "ativa" | "arquivado";
  readonly totalAulas: number;
  readonly faltas: number;
}

export interface Avaliacao {
  readonly id: string;
  readonly disciplinaId: string;
  readonly titulo: string;
  readonly tipo: "prova" | "trabalho" | "apresentacao" | "entrega" | "outro";
  readonly peso: number;
  /** §4.2 **[D]**: avaliação que não vale nota vira responsabilidade, fora da média. */
  readonly valeNota: boolean;
  readonly plainDate: string | null;
}

export interface Nota {
  readonly id: string;
  readonly avaliacaoId: string;
  readonly valor: number;
  readonly lancadaEm: number;
}

/**
 * Saída do motor de notas — SPEC-D-010 §5. Tudo derivado, nada persistido (I1).
 * `situacao` é apresentação: filtro por situação é feito no Rust, com o enum
 * (I15) — a string nunca ordena nem filtra.
 */
export interface MediaResultado {
  readonly media: number | null;
  readonly mediaSemArredondamento: number | null;
  readonly situacao: "sem-notas" | "aprovada" | "precisa-tirar" | "so-com-exame" | "nao-alcanca";
  readonly notaNecessariaProxima: number | null;
  readonly frequenciaRestante: number | null;
  readonly regraAplicada: string;
}

/** Configuração de regra, igual nas três camadas de D75. */
export interface RegraMedia {
  readonly tipoDeCalculo: "simples" | "ponderada";
  readonly notaMinima: number;
  readonly arredondamento: number;
  readonly frequenciaMinima: number;
  readonly exameModo: "nenhum" | "recuperacao" | "substitui";
  readonly exameMinima: number;
  readonly exameElegivelMinima: number;
}

// ---------------------------------------------------------------------------
// Leitura
// ---------------------------------------------------------------------------

export async function listSemestres(): Promise<Semestre[]> {
  return invoke<Semestre[]>("list_semestres");
}

export async function listDisciplinas(semestreId?: string | null): Promise<Disciplina[]> {
  return invoke<Disciplina[]>("list_disciplinas", {
    filter: { semestreId: semestreId ?? null },
  });
}

export async function mediaDaDisciplina(disciplinaId: string): Promise<MediaResultado> {
  return invoke<MediaResultado>("media_da_disciplina", { payload: { disciplinaId } });
}

/** Avaliação com a nota atual (`null` = sem nota lançada). `LEFT JOIN` no Rust. */
export interface AvaliacaoComNota extends Avaliacao {
  readonly nota: Nota | null;
}

export async function listAvaliacoes(disciplinaId: string): Promise<AvaliacaoComNota[]> {
  return invoke<AvaliacaoComNota[]>("list_avaliacoes", { payload: { disciplinaId } });
}

// ---------------------------------------------------------------------------
// Escrita
// ---------------------------------------------------------------------------

export interface NovoSemestre {
  readonly rotulo: string;
  readonly startsOn: string;
  readonly endsOn: string;
}

export async function createSemestre(payload: NovoSemestre): Promise<Semestre> {
  return invoke<Semestre>("create_semestre", { payload: { ...payload } });
}

export interface NovaDisciplina {
  readonly semestreId: string;
  readonly nome: string;
  readonly codigo?: string;
  readonly professor?: string;
  readonly cargaHoraria?: number;
  readonly totalAulas?: number;
}

export async function createDisciplina(payload: NovaDisciplina): Promise<Disciplina> {
  return invoke<Disciplina>("create_disciplina", { payload: { ...payload } });
}

export interface NovaAvaliacao {
  readonly disciplinaId: string;
  readonly titulo: string;
  readonly tipo: Avaliacao["tipo"];
  readonly peso: number;
  readonly valeNota?: boolean;
  readonly plainDate?: string;
}

export async function createAvaliacao(payload: NovaAvaliacao): Promise<Avaliacao> {
  return invoke<Avaliacao>("create_avaliacao", { payload: { ...payload } });
}

/**
 * Lança (ou corrige) a nota de uma avaliação. I9: **sem guarda de semestre** —
 * nota é dado, e continua lançável depois de encerrar. A correção da nota em
 * semestre encerrado não toca a regra (I7).
 */
export async function lancarNota(avaliacaoId: string, valor: number): Promise<Nota> {
  return invoke<Nota>("lancar_nota", { payload: { avaliacaoId, valor } });
}

/**
 * Padrão global (D75). Sem guarda de semestre: quem ele move são as
 * disciplinas não personalizadas — e é isso que a herança viva manda (I6).
 */
export async function setRegraPadrao(regra: RegraMedia): Promise<void> {
  await invoke<void>("set_regra_padrao", { payload: { ...regra } });
}

/** Personalização por disciplina (D75): substituição **por inteiro** (I7/I9). */
export async function setRegraDisciplina(disciplinaId: string, regra: RegraMedia): Promise<void> {
  await invoke<void>("set_regra_disciplina", {
    payload: { disciplinaId, regra: { ...regra } },
  });
}

/**
 * D75: encerrar é um **ato que grava** — congela a regra efetiva de cada
 * disciplina e fecha o semestre. Depois disto, regra e estrutura não mudam (I7);
 * só a nota continua lançável (I9).
 */
export async function encerrarSemestre(semestreId: string): Promise<void> {
  await invoke<void>("encerrar_semestre", { payload: { semestreId } });
}
