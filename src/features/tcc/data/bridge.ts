/**
 * Ponte com o backend Rust do módulo TCC & Projetos (SPEC-D-011, SPEC-D-013).
 *
 * A mesma regra das outras pontes: **este arquivo não contém SQL** e não
 * recalcula nada. A árvore, a paginação e a serialização canônica da versão
 * vivem em `src-tauri/src/tcc/`; o versionamento é decidido no Rust (D87:
 * escrita sem mudança **não** cria versão). Aqui só há o tipo e a chamada.
 *
 * O formato dos nós é o do contrato de
 * `cecistudy-desktop/docs/espacos/tcc-modelo-do-documento.md` §Contrato. Este
 * arquivo é o **espelho de tipo** do Rust: quando divergirem, o Rust está certo
 * (ADR-006) e este é o bug.
 */

import { invoke } from "../../../lib/ipc.ts";

// ---------------------------------------------------------------------------
// A árvore do documento (D85)
// ---------------------------------------------------------------------------

/** Como uma citação ou fragmento aponta para a fonte (D66). */
export type ModoVinculo = "vivo" | "snapshot" | "copia-editorial";

/** Marca de formatação de um trecho. Lista fechada, em kebab-case. */
export type MarcaInline = "negrito" | "italico" | "sublinhado" | "sobrescrito" | "subscrito";

/** Um trecho de texto com marcas. */
export interface Inline {
  readonly texto: string;
  readonly marcas: readonly MarcaInline[];
}

/**
 * Um nó da árvore. Discriminado por `tipo`, kebab-case, como o
 * `#[serde(tag = "tipo")]` do Rust.
 *
 * A posição na árvore não mora no nó: o nó é conteúdo. A persistência guarda
 * `pai_id`/`ordem` (lista de adjacência ordenada), e é o Rust que monta e
 * desmonta — a tela vê a árvore aninhada.
 */
export type NoDocumento =
  | {
      readonly tipo: "capitulo";
      readonly id: string;
      readonly titulo: string;
      readonly filhos: readonly NoDocumento[];
    }
  | { readonly tipo: "paragrafo"; readonly id: string; readonly inline: readonly Inline[] }
  | {
      readonly tipo: "citacao";
      readonly id: string;
      readonly referenciaId: string;
      readonly modo: ModoVinculo;
    }
  | { readonly tipo: "referencia-viva"; readonly id: string; readonly fragmentoId: string };

// ---------------------------------------------------------------------------
// Projeto e documento
// ---------------------------------------------------------------------------

/** O projeto, como a lista da tela vê. `tipo` e `estado` são listas fechadas. */
export interface Projeto {
  readonly id: string;
  readonly titulo: string;
  readonly tipo: string;
  readonly estado: string;
  readonly descricao: string | null;
  readonly createdAt: number;
  readonly updatedAt: number;
}

export interface Documento {
  readonly id: string;
  readonly projetoId: string;
  readonly titulo: string;
  readonly arvore: readonly NoDocumento[];
  /** `null` = nenhuma versão ainda. */
  readonly ultimaVersao: number | null;
}

// ---------------------------------------------------------------------------
// Paginação (derivada, nunca gravada — D77/D86)
// ---------------------------------------------------------------------------

/**
 * A geometria da página (D85/D86). Em milímetros; o corpo, em pontos. É
 * **parâmetro** do motor, nunca coluna: mudá-la reprocessa o documento sem
 * tocar na árvore. Espelho do `Geometria` do Rust; quando divergirem, o Rust
 * está certo (ADR-006).
 */
export interface Geometria {
  readonly larguraMm: number;
  readonly alturaMm: number;
  readonly margemSupMm: number;
  readonly margemInfMm: number;
  readonly margemEsqMm: number;
  readonly margemDirMm: number;
  readonly corpoPt: number;
  readonly entrelinha: number;
}

/**
 * O resultado de paginar. `paginas` e `alturasPorNo` são o cálculo puro; o
 * `medidor` **declara** qual medida o produziu porque `D86` proíbe paginação
 * silenciosamente aproximada. `geometria` é a que foi de fato aplicada.
 */
export interface Paginacao {
  readonly paginas: number;
  readonly alturasPorNo: Readonly<Record<string, number>>;
  readonly medidor: string;
  readonly geometria: Geometria;
}

// ---------------------------------------------------------------------------
// Base de pesquisa (vínculos, sem cópia — D66/I11)
// ---------------------------------------------------------------------------

/** Os dois alvos possíveis de um vínculo. Referência é da Biblioteca; Fragmento, do Conhecimento. */
export type AlvoTipo = "referencia" | "fragmento";

/**
 * Um vínculo da Base de pesquisa. O projeto **aponta** para o acervo e guarda só
 * o id (`alvoId`), nunca uma cópia. `alvoId` é referência viva sem FK enquanto a
 * Biblioteca e o Conhecimento não existem.
 */
export interface Vinculo {
  readonly id: string;
  readonly projetoId: string;
  readonly alvoTipo: AlvoTipo;
  readonly alvoId: string;
  readonly criadoEm: number;
}

/**
 * O resultado de salvar. `versionada = false` quando a árvore não mudou: o Rust
 * compara o snapshot canônico e não grava versão (D87).
 */
export interface DocumentoSalvo {
  readonly documentoId: string;
  readonly numero: number;
  readonly versionada: boolean;
}

// ---------------------------------------------------------------------------
// Item a resolver (D88)
// ---------------------------------------------------------------------------

export type TipoItem = "comentario" | "sugestao";

export type EstadoItem = "pendente" | "resolvido" | "nao-encaixou" | "duplicado";

/** Um item da lista de decisão da importação de DOCX. */
export interface ItemAResolver {
  readonly id: string;
  readonly documentoId: string;
  readonly tipo: TipoItem;
  readonly autor: string;
  /** `false` = autor não identificado no arquivo (D78), nunca inventado. */
  readonly autorResolvido: boolean;
  /** Quando o autor marcou a data, em epoch-ms. `null` é "sem data". */
  readonly dataAutor: number | null;
  /** A âncora é o texto citado, nunca um deslocamento (D78). */
  readonly citado: string;
  /** Em `duplicado`, o id do item que ficou. */
  readonly substituto: string | null;
  readonly texto: string | null;
  readonly origemArquivo: string | null;
  readonly importadoEm: number;
  readonly estado: EstadoItem;
  /** `null` enquanto pendente. */
  readonly resolvidoEm: number | null;
}

// ---------------------------------------------------------------------------
// Leitura
// ---------------------------------------------------------------------------

export async function listProjetos(): Promise<Projeto[]> {
  return invoke<Projeto[]>("list_projetos");
}

export async function getDocumento(documentoId: string): Promise<Documento> {
  return invoke<Documento>("get_documento", { payload: { documentoId } });
}

/**
 * Lê o documento pelo **projeto**, sem precisar do `documento_id`. Na v1 um
 * projeto tem exatamente um documento (`documento.projeto_id` é `UNIQUE`), e a
 * lista de projetos não carrega o vínculo: quem abre o projeto pede o documento
 * por aqui.
 */
export async function getDocumentoDoProjeto(projetoId: string): Promise<Documento> {
  return invoke<Documento>("get_documento_do_projeto", { payload: { projetoId } });
}

/**
 * Pagina o documento sem gravar nada: a página é **derivada** (`D77`/`D86`).
 * Sem `geometria`, vale o A4 ABNT do backend. O retorno traz o `medidor` que
 * produziu o número — em produção o medidor real de fontes ainda é dependência
 * declarada (`D86`), e a tela **mostra** isso em vez de aproximar em silêncio.
 */
export async function paginarDocumento(
  documentoId: string,
  geometria?: Geometria,
): Promise<Paginacao> {
  return invoke<Paginacao>("paginar_documento", {
    payload: geometria === undefined ? { documentoId } : { documentoId, geometria },
  });
}

export async function listItens(documentoId: string): Promise<ItemAResolver[]> {
  return invoke<ItemAResolver[]>("list_itens", { payload: { documentoId } });
}

/** Os vínculos do projeto, na ordem em que entraram. */
export async function listBase(projetoId: string): Promise<Vinculo[]> {
  return invoke<Vinculo[]>("list_base", { payload: { projetoId } });
}

// ---------------------------------------------------------------------------
// Escrita
// ---------------------------------------------------------------------------

export interface NovoProjeto {
  readonly titulo: string;
  readonly tipo: string;
  readonly descricao?: string;
}

/** Cria o projeto e o documento dele numa transação: um não existe sem o outro. */
export async function createProjeto(payload: NovoProjeto): Promise<Projeto> {
  return invoke<Projeto>("create_projeto", { payload: { ...payload } });
}

export async function setEstadoProjeto(projetoId: string, estado: string): Promise<void> {
  return invoke<void>("set_estado_projeto", { payload: { projetoId, estado } });
}

export interface NovoVinculo {
  readonly projetoId: string;
  readonly alvoTipo: AlvoTipo;
  readonly alvoId: string;
}

/**
 * Vincula o projeto a um alvo do acervo. Idempotente no Rust: vincular o mesmo
 * (`projeto`, `alvoTipo`, `alvoId`) de novo devolve o vínculo que já existia, sem
 * duplicar (I11).
 */
export async function vincular(payload: NovoVinculo): Promise<Vinculo> {
  return invoke<Vinculo>("vincular", { payload: { ...payload } });
}

/** Remove o vínculo. O alvo (Biblioteca/Conhecimento) não é tocado. */
export async function desvincular(vinculoId: string): Promise<void> {
  return invoke<void>("desvincular", { payload: { vinculoId } });
}

export interface SalvarDocumento {
  readonly documentoId: string;
  readonly arvore: readonly NoDocumento[];
  readonly autor: string;
  readonly motivo?: string;
}

/**
 * Substitui a árvore do documento. Se ela mudou, o Rust grava uma versão (D87);
 * se não mudou, devolve `versionada: false` sem tocar no banco.
 */
export async function salvarDocumento(payload: SalvarDocumento): Promise<DocumentoSalvo> {
  return invoke<DocumentoSalvo>("salvar_documento", {
    payload: { ...payload, arvore: [...payload.arvore] },
  });
}

export interface NovoItem {
  readonly documentoId: string;
  readonly tipo: TipoItem;
  readonly autor: string;
  /** O padrão é `true`; `false` é autor desconhecido (D78). */
  readonly autorResolvido?: boolean;
  readonly dataAutorMs?: number;
  readonly citado: string;
  readonly texto?: string;
  readonly origemArquivo?: string;
}

/** Anexa um item a resolver. A importação nunca altera a árvore (D88). */
export async function registrarItem(payload: NovoItem): Promise<ItemAResolver> {
  return invoke<ItemAResolver>("registrar_item", { payload: { ...payload } });
}

/**
 * Decide um item. `duplicado` exige `substituto`; sem ele o Rust recusa (D88).
 */
export async function resolverItem(
  itemId: string,
  estado: EstadoItem,
  substituto?: string,
): Promise<void> {
  return invoke<void>("resolver_item", { payload: { itemId, estado, substituto } });
}
