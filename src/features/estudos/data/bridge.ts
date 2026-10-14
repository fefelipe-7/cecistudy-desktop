/**
 * Ponte com o backend Rust do módulo Estudos (SPEC-D-010, D76).
 *
 * A mesma regra das outras pontes: **este arquivo não contém SQL** e não
 * recalcula nada. O FSRS é da crate em Rust, o vencimento é derivado na leitura
 * (cláusula do vencimento, D76) — qualquer segundo cálculo aqui seria regra de
 * domínio na interface. A zona do vencimento fica no Rust (literal único do
 * produto); a ponte não opina sobre fuso.
 */

import { invoke } from "../../../lib/ipc.ts";

export interface Conteudo {
  readonly id: string;
  readonly titulo: string;
  readonly texto: string | null;
}

/**
 * O cartão com o estado FSRS que o banco guarda (D76). `estabilidade`/`dificuldade`
 * `null` é o cartão novo; `vencimento` é derivado na leitura, `null` quando
 * nunca houve agendamento.
 */
export interface Cartao {
  readonly id: string;
  readonly conteudoId: string;
  readonly frente: string;
  readonly verso: string;
  readonly estabilidade: number | null;
  readonly dificuldade: number | null;
  readonly ultimaRevisao: number | null;
  readonly intervaloDias: number | null;
  readonly relearning: boolean;
  readonly vencimento: string | null;
}

/** Um cartão devido hoje, com o título do conteúdo. */
export interface CartaoDaFila {
  readonly id: string;
  readonly conteudoId: string;
  readonly tituloConteudo: string;
  readonly frente: string;
  readonly verso: string;
  /** `null` = cartão novo: devido hoje, sem agendamento anterior. */
  readonly vencimento: string | null;
  readonly relearning: boolean;
}

// ---------------------------------------------------------------------------
// Escrita
// ---------------------------------------------------------------------------

export interface NovoConteudo {
  readonly titulo: string;
  readonly texto?: string;
}

export async function createConteudo(payload: NovoConteudo): Promise<Conteudo> {
  return invoke<Conteudo>("create_conteudo", { payload: { ...payload } });
}

export interface NovoCartao {
  readonly conteudoId: string;
  readonly frente: string;
  readonly verso: string;
}

export async function createCartao(payload: NovoCartao): Promise<Cartao> {
  return invoke<Cartao>("create_cartao", { payload: { ...payload } });
}

/**
 * A revisão que move o cartão: `rating` 1..4 (again/hard/good/easy) e a
 * retenção desejada em (0, 1). O padrão da retenção é **[P]** — a tela envia
 * 0,9 provisório com seletor visível, e a dona do produto confirma.
 */
export interface RevisaoDoCartao {
  readonly cartaoId: string;
  readonly rating: 1 | 2 | 3 | 4;
  readonly retencao: number;
}

export async function revisarCartao(payload: RevisaoDoCartao): Promise<Cartao> {
  return invoke<Cartao>("revisar_cartao", { payload: { ...payload } });
}

// ---------------------------------------------------------------------------
// Leitura
// ---------------------------------------------------------------------------

export async function filaDoDia(): Promise<CartaoDaFila[]> {
  return invoke<CartaoDaFila[]>("fila_do_dia", { payload: {} });
}
