/**
 * Apresentação do módulo TCC & Projetos (SPEC-D-013, D85..D88).
 *
 * Regra de apresentação, nada mais: os rótulos das listas fechadas que o Rust
 * impõe por `CHECK` (`tipo` de projeto, `estado`, `tipo` e `estado` de item).
 * Isto **não** é a verdade da lista — a verdade é a `CHECK` do schema e o motor
 * em `src-tauri/src/tcc/`. Um valor que o Rust devolva e este mapa não conheça
 * aparece cru, nunca em branco: rótulo desconhecido é melhor que campo vazio.
 */

import type {
  AlvoTipo,
  EstadoItem,
  ItemAResolver,
  NoDocumento,
  Projeto,
  TipoItem,
} from "../data/bridge.ts";

interface Rotulo<T extends string> {
  readonly id: T;
  readonly rotulo: string;
}

/** Os sete tipos fixos de projeto (§4.7 [D]). */
export const TIPOS_DE_PROJETO: ReadonlyArray<Rotulo<string>> = [
  { id: "tcc", rotulo: "TCC" },
  { id: "artigo", rotulo: "Artigo" },
  { id: "iniciacao-cientifica", rotulo: "Iniciação científica" },
  { id: "relatorio-de-estagio", rotulo: "Relatório de estágio" },
  { id: "projeto-de-pesquisa", rotulo: "Projeto de pesquisa" },
  { id: "revisao-bibliografica", rotulo: "Revisão bibliográfica" },
  { id: "estudo-de-caso", rotulo: "Estudo de caso" },
];

/** Os três estados de um projeto. */
export const ESTADOS_DE_PROJETO: ReadonlyArray<Rotulo<string>> = [
  { id: "ativo", rotulo: "Ativo" },
  { id: "encerrado", rotulo: "Encerrado" },
  { id: "arquivado", rotulo: "Arquivado" },
];

/** Os dois tipos de item a resolver. */
export const TIPOS_ITEM: ReadonlyArray<Rotulo<TipoItem>> = [
  { id: "comentario", rotulo: "Comentário" },
  { id: "sugestao", rotulo: "Sugestão" },
];

/** Os quatro estados de decisão de um item (D88). */
export const ESTADOS_ITEM: ReadonlyArray<Rotulo<EstadoItem>> = [
  { id: "pendente", rotulo: "Pendente" },
  { id: "resolvido", rotulo: "Resolvido" },
  { id: "nao-encaixou", rotulo: "Não encaixou" },
  { id: "duplicado", rotulo: "Duplicado" },
];

/** Os dois alvos de um vínculo da Base de pesquisa. Referência é da Biblioteca; Fragmento, do Conhecimento. */
export const TIPOS_DE_ALVO: ReadonlyArray<Rotulo<AlvoTipo>> = [
  { id: "referencia", rotulo: "Referência" },
  { id: "fragmento", rotulo: "Fragmento" },
];

function rotuloDe(lista: ReadonlyArray<Rotulo<string>>, id: string): string {
  return lista.find((item) => item.id === id)?.rotulo ?? id;
}

export function rotuloTipoProjeto(tipo: string): string {
  return rotuloDe(TIPOS_DE_PROJETO, tipo);
}

export function rotuloEstadoProjeto(estado: string): string {
  return rotuloDe(ESTADOS_DE_PROJETO, estado);
}

export function rotuloTipoItem(tipo: TipoItem): string {
  return rotuloDe(TIPOS_ITEM, tipo);
}

export function rotuloEstadoItem(estado: EstadoItem): string {
  return rotuloDe(ESTADOS_ITEM, estado);
}

export function rotuloAlvoTipo(tipo: AlvoTipo): string {
  return rotuloDe(TIPOS_DE_ALVO, tipo);
}

/** Diz se o item ainda espera decisão. */
export function itemPendente(item: ItemAResolver): boolean {
  return item.estado === "pendente";
}

/**
 * A assinatura de autoria do item. Autor desconhecido é **dito** (D78), nunca
 * escondido atrás de um nome vazio.
 */
export function assinaturaDoItem(item: ItemAResolver): string {
  const nome = item.autor.trim();
  if (!item.autorResolvido) return nome ? `${nome} · não identificado` : "autor não identificado";
  return nome || "autor";
}

/** Contagem dos nós do documento, para o resumo do painel. */
export interface ResumoDocumento {
  readonly secoes: number;
  readonly paragrafos: number;
  readonly citacoes: number;
  readonly referencias: number;
  readonly total: number;
}

/**
 * Conta os nós da árvore por tipo. É leitura da árvore que o Rust remontou, não
 * regra: o capítulo é recursivo porque os filhos moram nele.
 */
export function contarArvore(arvore: readonly NoDocumento[]): ResumoDocumento {
  const acc = { secoes: 0, paragrafos: 0, citacoes: 0, referencias: 0, total: 0 };
  for (const no of arvore) {
    acc.total += 1;
    switch (no.tipo) {
      case "capitulo":
        acc.secoes += 1;
        {
          const filhos = contarArvore(no.filhos);
          acc.secoes += filhos.secoes;
          acc.paragrafos += filhos.paragrafos;
          acc.citacoes += filhos.citacoes;
          acc.referencias += filhos.referencias;
          acc.total += filhos.total;
        }
        break;
      case "paragrafo":
        acc.paragrafos += 1;
        break;
      case "citacao":
        acc.citacoes += 1;
        break;
      case "referencia-viva":
        acc.referencias += 1;
        break;
    }
  }
  return acc;
}

/** O rótulo da versão. Sem versão gravada, o documento é rascunho, não "v0". */
export function rotuloVersao(ultimaVersao: number | null): string {
  return ultimaVersao === null ? "rascunho (sem versão)" : `versão ${ultimaVersao}`;
}

/**
 * O limite de projetos **ativos** de §4.7 [D]. Espelho do
 * `LIMITE_DE_PROJETOS_ATIVOS` do Rust (`commands.rs`): a tela avisa antes, e o
 * Rust recusa depois no caminho de escrita. Quando divergirem, o Rust está
 * certo (ADR-006) e este número é o bug.
 */
export const LIMITE_DE_PROJETOS_ATIVOS = 5;

/** Quantos projetos da lista estão ativos. */
export function projetosAtivos(projetos: readonly Projeto[]): number {
  return projetos.filter((projeto) => projeto.estado === "ativo").length;
}

/** Diz se ainda cabe um projeto novo. É aviso de tela; quem grava é o Rust. */
export function podeCriarProjeto(projetos: readonly Projeto[]): boolean {
  return projetosAtivos(projetos) < LIMITE_DE_PROJETOS_ATIVOS;
}
