/**
 * As operações de edição da árvore do documento (SPEC-D-011 D77, SPEC-D-013
 * D85).
 *
 * São **funções puras**: entram a árvore e um id, sai uma árvore nova — sem
 * `setState`, sem IPC, sem tocar na persistência. É a **operação de UI**: a
 * validação de verdade (id vazio, vínculo sem alvo, colunas casando com o tipo)
 * é do Rust, em `linhas_da_arvore`/`salvar_documento` (`D85`), e a geometria
 * A4-ABNT é aplicada lá, no motor `paginar`. Aqui não se re-deriva nenhuma
 * dessas regras — só se move o conteúdo que o editor mostra.
 *
 * A árvore é **imutável**: cada função devolve uma árvore nova e deixa a antiga
 * intacta. O autosave comparara o snapshot canônico no Rust (`D87`) e só
 * versiona quando o conteúdo muda de fato.
 *
 * Na v1 o parágrafo tem **texto simples**: `atualizarTexto` troca o inline por
 * **um único trecho sem marcas**. A marcação inline (negrito, itálico) volta na
 * fase de texto rico; hoje o editor **não** finge que a preserva — ele a
 * substitui de forma declarada.
 */

import type { Inline, NoDocumento } from "../data/bridge.ts";

/** A direção de um movimento entre irmãos. */
export type Direcao = "cima" | "baixo";

/** O tipo do nó de capítulo, extraído do contrato. */
type Capitulo = Extract<NoDocumento, { tipo: "capitulo" }>;
/** O tipo do nó de parágrafo, extraído do contrato. */
type Paragrafo = Extract<NoDocumento, { tipo: "paragrafo" }>;

/**
 * Um id novo de nó. Os ids vêm **do cliente** (`D85`): a árvore é montada no
 * editor e o Rust só valida e grava. `crypto.randomUUID` é o uuid do app.
 */
export function novoId(): string {
  return crypto.randomUUID();
}

/** Um capítulo vazio, com id novo. */
export function capituloVazio(titulo = ""): NoDocumento {
  return { tipo: "capitulo", id: novoId(), titulo, filhos: [] };
}

/** Um parágrafo vazio (ou com texto), com id novo e um trecho sem marcas. */
export function paragrafoVazio(texto = ""): NoDocumento {
  return { tipo: "paragrafo", id: novoId(), inline: [{ texto, marcas: [] }] };
}

/** O texto simples de um nó. Só o parágrafo tem texto; o resto devolve vazio. */
export function textoDoNo(no: NoDocumento): string {
  if (no.tipo !== "paragrafo") return "";
  return no.inline.map((run) => run.texto).join("");
}

/**
 * Aplica `f` a cada nó, recursivamente, remontando a árvore.
 *
 * `f` devolve o nó (talvez modificado) ou `null` para removê-lo. Quando `f`
 * devolve um capítulo, os filhos dele são percorridos — é o que permite editar
 * em qualquer profundidade com uma função só.
 */
function transformar(
  arvore: readonly NoDocumento[],
  f: (no: NoDocumento) => NoDocumento | null,
): NoDocumento[] {
  const saida: NoDocumento[] = [];
  for (const no of arvore) {
    const transformado = f(no);
    if (transformado === null) continue;
    if (transformado.tipo === "capitulo") {
      saida.push({ ...transformado, filhos: transformar(transformado.filhos, f) });
    } else {
      saida.push(transformado);
    }
  }
  return saida;
}

/** Troca o título de um capítulo. Sem o id, a árvore volta igual. */
export function atualizarTitulo(
  arvore: readonly NoDocumento[],
  id: string,
  titulo: string,
): NoDocumento[] {
  return transformar(arvore, (no): NoDocumento =>
    no.tipo === "capitulo" && no.id === id ? { ...no, titulo } : no,
  );
}

/**
 * Troca o texto de um parágrafo por **um trecho sem marcas** (v1 de texto
 * simples). A troca é declarada: o editor não preserva marcas inline ainda.
 */
export function atualizarTexto(
  arvore: readonly NoDocumento[],
  id: string,
  texto: string,
): NoDocumento[] {
  return transformar(arvore, (no): NoDocumento => {
    if (no.tipo !== "paragrafo" || no.id !== id) return no;
    const inline: Inline[] = [{ texto, marcas: [] }];
    return { tipo: "paragrafo", id: no.id, inline };
  });
}

/** Remove o nó com o id dado, em qualquer profundidade. */
export function remover(arvore: readonly NoDocumento[], id: string): NoDocumento[] {
  return transformar(arvore, (no) => (no.id === id ? null : no));
}

/**
 * Troca dois irmãos de lugar. Devolve `null` quando o id não está nesta lista;
 * devolve a lista igual quando o id está na ponta e não há para onde mover.
 */
function trocar(
  irmaos: readonly NoDocumento[],
  id: string,
  deslocamento: number,
): readonly NoDocumento[] | null {
  const indice = irmaos.findIndex((no) => no.id === id);
  if (indice === -1) return null;
  const destino = indice + deslocamento;
  if (destino < 0 || destino >= irmaos.length) return irmaos;
  const atual = irmaos[indice];
  const vizinho = irmaos[destino];
  if (atual === undefined || vizinho === undefined) return irmaos;
  const copia = [...irmaos];
  copia[indice] = vizinho;
  copia[destino] = atual;
  return copia;
}

/** Move um nó uma posição para cima ou para baixo, entre os irmãos dele. */
export function mover(
  arvore: readonly NoDocumento[],
  id: string,
  direcao: Direcao,
): readonly NoDocumento[] {
  const deslocamento = direcao === "cima" ? -1 : 1;
  const naRaiz = trocar(arvore, id, deslocamento);
  if (naRaiz !== null) return naRaiz;
  return arvore.map((no) =>
    no.tipo === "capitulo" ? { ...no, filhos: mover(no.filhos, id, direcao) } : no,
  );
}

/**
 * Insere `novo` logo **depois** do nó com o id dado, no mesmo nível. Devolve
 * `null` quando o id não está nesta lista.
 */
function inserirDepois(
  irmaos: readonly NoDocumento[],
  id: string,
  novo: NoDocumento,
): readonly NoDocumento[] | null {
  const indice = irmaos.findIndex((no) => no.id === id);
  if (indice === -1) return null;
  const copia = [...irmaos];
  copia.splice(indice + 1, 0, novo);
  return copia;
}

/** Insere um irmão logo depois do nó com o id dado, em qualquer profundidade. */
export function inserirIrmao(
  arvore: readonly NoDocumento[],
  id: string,
  novo: NoDocumento,
): readonly NoDocumento[] {
  const naRaiz = inserirDepois(arvore, id, novo);
  if (naRaiz !== null) return naRaiz;
  return arvore.map((no) =>
    no.tipo === "capitulo" ? { ...no, filhos: inserirIrmao(no.filhos, id, novo) } : no,
  );
}

/**
 * Anexa `novo` como último filho do capítulo com o id dado. É o caminho para
 * preencher um capítulo **vazio**, onde não há irmão para referenciar.
 */
export function adicionarFilho(
  arvore: readonly NoDocumento[],
  capituloId: string,
  novo: NoDocumento,
): NoDocumento[] {
  return transformar(arvore, (no): NoDocumento =>
    no.tipo === "capitulo" && no.id === capituloId ? { ...no, filhos: [...no.filhos, novo] } : no,
  );
}

/** Um capítulo estreitado, para o teste e para quem monta o `adicionarFilho`. */
export type { Capitulo, Paragrafo };
