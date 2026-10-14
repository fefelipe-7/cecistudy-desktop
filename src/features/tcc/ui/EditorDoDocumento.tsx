/**
 * O editor do documento (SPEC-D-011 D77, SPEC-D-013 D85..D88).
 *
 * A árvore é o **modelo interno** (`D85`): o editor edita a árvore, e a página é
 * **derivada** no Rust (`paginar_documento`, `D86`). Nada de número de página,
 * altura ou posição de cursor é gravado — reprocessar a geometria muda as páginas
 * sem tocar na árvore. Cada edição passa pelo caminho de escrita
 * (`salvar_documento`), que compara o snapshot canônico e só versiona quando o
 * conteúdo muda de fato (`D87`).
 *
 * A v1 edita **capítulo e parágrafo**. Inserir **citação** e **referência viva**
 * fica de fora de propósito: a citação puxa da Biblioteca e a referência viva, do
 * Conhecimento, e nenhum dos dois existe ainda (`I13`, `D79`). Se a árvore já
 * trouxer um vínculo (importação futura), ele é **mostrado e movível**, nunca
 * inventado aqui.
 *
 * O autosave coalesce a digitação numa escrita (`D87`): não grava por tecla. O
 * número de páginas é recalculado depois da escrita e exibido com o `medidor` que
 * o produziu — aproximar em silêncio é proibido por `D86`.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { ArrowDown, ArrowLeft, ArrowUp, Plus, Trash2 } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";

import {
  paginarDocumento,
  salvarDocumento,
  type Documento,
  type DocumentoSalvo,
  type NoDocumento,
} from "../data/bridge.ts";
import {
  adicionarFilho,
  atualizarTexto,
  atualizarTitulo,
  capituloVazio,
  inserirIrmao,
  mover,
  paragrafoVazio,
  remover,
  textoDoNo,
  type Direcao,
} from "./editar.ts";

/**
 * O autor local. Não há conta no app (`D79`): a escrita é atribuída a quem usa o
 * app, sem inventar um perfil.
 */
const AUTOR_LOCAL = "usuária";

/** O atraso do autosave, em ms. Coalesce a digitação numa escrita (`D87`). */
const ATRASO_DO_AUTOSAVE = 800;

/** O tipo do nó de capítulo, extraído do contrato, para os blocos abaixo. */
type Capitulo = Extract<NoDocumento, { tipo: "capitulo" }>;
/** O tipo do nó de parágrafo, para os blocos abaixo. */
type Paragrafo = Extract<NoDocumento, { tipo: "paragrafo" }>;
/** O tipo do vínculo (citação ou referência viva), só leitura na v1. */
type VinculoDoNo = Extract<NoDocumento, { tipo: "citacao" | "referencia-viva" }>;

export function EditorDoDocumento({
  documento,
  onVoltar,
  onSalvo,
}: {
  documento: Documento;
  onVoltar: () => void;
  onSalvo: (salvo: DocumentoSalvo) => void;
}) {
  const [arvore, setArvore] = useState<readonly NoDocumento[]>(documento.arvore);
  const [paginas, setPaginas] = useState<number | null>(null);
  const [medidor, setMedidor] = useState<string | null>(null);
  const [situacao, setSituacao] = useState("sem mudanças");

  // O `onSalvo` do pai muda de identidade a cada render; guardá-lo num ref evita
  // que o autosave se reagende sozinho (o efeito depende de `agendar`).
  const onSalvoRef = useRef(onSalvo);
  useEffect(() => {
    onSalvoRef.current = onSalvo;
  });

  const editar = useCallback((f: (a: readonly NoDocumento[]) => readonly NoDocumento[]): void => {
    setArvore((a) => f(a));
  }, []);

  // A paginação inicial: o número é derivado do Rust (D86), nunca contado aqui.
  useEffect(() => {
    let cancelado = false;
    paginarDocumento(documento.id)
      .then((p) => {
        if (cancelado) return;
        setPaginas(p.paginas);
        setMedidor(p.medidor);
      })
      .catch(() => {
        /* a ausência do número não bloqueia a edição */
      });
    return () => {
      cancelado = true;
    };
  }, [documento.id]);

  const agendar = useCallback(async (): Promise<void> => {
    setSituacao("salvando…");
    try {
      const salvo = await salvarDocumento({
        documentoId: documento.id,
        arvore,
        autor: AUTOR_LOCAL,
      });
      onSalvoRef.current(salvo);
      const pag = await paginarDocumento(documento.id);
      setPaginas(pag.paginas);
      setMedidor(pag.medidor);
      setSituacao(salvo.versionada ? `versão ${salvo.numero}` : "sem mudanças");
    } catch (e: unknown) {
      setSituacao(e instanceof Error ? e.message : String(e));
    }
  }, [arvore, documento.id]);

  // Autosave com debounce: uma escrita por rajada de digitação (D87). O primeiro
  // render não grava — o documento acabou de vir do backend.
  const primeira = useRef(true);
  useEffect(() => {
    if (primeira.current) {
      primeira.current = false;
      return;
    }
    const t = window.setTimeout(() => {
      void agendar();
    }, ATRASO_DO_AUTOSAVE);
    return () => window.clearTimeout(t);
  }, [agendar]);

  const sair = useCallback(async (): Promise<void> => {
    await agendar();
    onVoltar();
  }, [agendar, onVoltar]);

  const acoes: Acoes = {
    onTitulo: (id, titulo) => editar((a) => atualizarTitulo(a, id, titulo)),
    onTexto: (id, texto) => editar((a) => atualizarTexto(a, id, texto)),
    onRemover: (id) => editar((a) => remover(a, id)),
    onMover: (id, direcao) => editar((a) => mover(a, id, direcao)),
    onParagrafoDepois: (id) => editar((a) => inserirIrmao(a, id, paragrafoVazio())),
    onParagrafoDentro: (id) => editar((a) => adicionarFilho(a, id, paragrafoVazio())),
    onCapituloDentro: (id) => editar((a) => adicionarFilho(a, id, capituloVazio("Subcapítulo"))),
  };

  return (
    <div className="mx-auto flex h-full max-w-3xl flex-col px-6 py-6">
      <div className="flex flex-wrap items-center gap-2">
        <Button variant="ghost" size="sm" className="h-8 text-[12px]" onClick={() => void sair()}>
          <ArrowLeft className="size-3.5" />
          Voltar
        </Button>
        <h2 className="truncate text-[13px] font-semibold">{documento.titulo}</h2>
        <Badge variant="outline" className="text-[10.5px]">
          {paginas === null ? "—" : `${paginas} ${paginas === 1 ? "página" : "páginas"}`}
        </Badge>
        {medidor !== null && (
          <span className="text-[10.5px] text-muted-foreground">
            medidor {medidor} — número aproximado (D86)
          </span>
        )}
        <span className="ml-auto shrink-0 text-[11px] text-muted-foreground">{situacao}</span>
      </div>

      <p className="mt-3 rounded-[8px] border border-border/70 bg-muted/40 px-3 py-2 text-[11.5px] text-muted-foreground">
        A página é derivada do conteúdo: o editor não grava número de página nem posição. Inserir
        citação e referência viva espera a Biblioteca e o Conhecimento (I13/D79); a importação e a
        exportação de DOCX entram na fase de round-trip (D78).
      </p>

      <div className="mt-4 flex-1 overflow-y-auto pr-1">
        {arvore.length === 0 ? (
          <p className="py-8 text-center text-[12.5px] text-muted-foreground">
            Documento vazio. Comece por um capítulo.
          </p>
        ) : (
          <Lista nos={arvore} acoes={acoes} />
        )}
      </div>

      <div className="mt-4 flex items-center gap-2 border-t border-border/60 pt-3">
        <Button
          variant="outline"
          size="sm"
          className="h-8 text-[12px]"
          onClick={() => editar((a) => [...a, capituloVazio("Novo capítulo")])}
        >
          <Plus className="size-3.5" />
          Capítulo
        </Button>
        <Button
          variant="outline"
          size="sm"
          className="h-8 text-[12px]"
          onClick={() => editar((a) => [...a, paragrafoVazio()])}
        >
          <Plus className="size-3.5" />
          Parágrafo
        </Button>
      </div>
    </div>
  );
}

/** As operações que os blocos disparam. Todas são puras do lado de `editar.ts`. */
interface Acoes {
  readonly onTitulo: (id: string, titulo: string) => void;
  readonly onTexto: (id: string, texto: string) => void;
  readonly onRemover: (id: string) => void;
  readonly onMover: (id: string, direcao: Direcao) => void;
  readonly onParagrafoDepois: (id: string) => void;
  readonly onParagrafoDentro: (id: string) => void;
  readonly onCapituloDentro: (id: string) => void;
}

function Lista({ nos, acoes }: { nos: readonly NoDocumento[]; acoes: Acoes }) {
  return (
    <ul className="flex flex-col gap-2">
      {nos.map((no, indice) => (
        <li key={no.id}>
          {no.tipo === "capitulo" ? (
            <BlocoCapitulo no={no} indice={indice} total={nos.length} acoes={acoes} />
          ) : no.tipo === "paragrafo" ? (
            <BlocoParagrafo no={no} indice={indice} total={nos.length} acoes={acoes} />
          ) : (
            <BlocoVinculo no={no} indice={indice} total={nos.length} acoes={acoes} />
          )}
        </li>
      ))}
    </ul>
  );
}

function BlocoCapitulo({
  no,
  indice,
  total,
  acoes,
}: {
  no: Capitulo;
  indice: number;
  total: number;
  acoes: Acoes;
}) {
  return (
    <div className="rounded-[10px] border border-border/70 bg-card/40 px-3 py-2">
      <div className="flex items-center gap-2">
        <Input
          value={no.titulo}
          onChange={(e) => acoes.onTitulo(no.id, e.target.value)}
          placeholder="Título do capítulo"
          aria-label="Título do capítulo"
          className="h-8 flex-1 text-[13px] font-medium"
        />
        <BotoesDeOrdem id={no.id} indice={indice} total={total} acoes={acoes} />
        <BotaoRemover id={no.id} acoes={acoes} />
      </div>

      <div className="mt-2 border-l border-border/60 pl-3">
        {no.filhos.length === 0 ? (
          <p className="py-1 text-[11.5px] text-muted-foreground">Capítulo vazio.</p>
        ) : (
          <Lista nos={no.filhos} acoes={acoes} />
        )}
      </div>

      <div className="mt-2 flex items-center gap-1.5">
        <Button
          variant="ghost"
          size="sm"
          className="h-7 text-[11.5px]"
          onClick={() => acoes.onParagrafoDentro(no.id)}
        >
          <Plus className="size-3" />
          Parágrafo
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="h-7 text-[11.5px]"
          onClick={() => acoes.onCapituloDentro(no.id)}
        >
          <Plus className="size-3" />
          Subcapítulo
        </Button>
      </div>
    </div>
  );
}

function BlocoParagrafo({
  no,
  indice,
  total,
  acoes,
}: {
  no: Paragrafo;
  indice: number;
  total: number;
  acoes: Acoes;
}) {
  const texto = textoDoNo(no);
  return (
    <div className="flex items-start gap-2 rounded-[10px] border border-border/70 bg-card/40 px-3 py-2">
      <Textarea
        value={texto}
        onChange={(e) => acoes.onTexto(no.id, e.target.value)}
        placeholder="Escreva o parágrafo…"
        aria-label="Parágrafo"
        rows={Math.min(14, Math.max(2, Math.ceil(texto.length / 70) + 1))}
        className="min-h-0 flex-1 resize-y border-0 bg-transparent px-0 py-0 text-[12.5px] leading-relaxed shadow-none focus-visible:ring-0"
      />
      <div className="flex shrink-0 flex-col items-center">
        <BotoesDeOrdem id={no.id} indice={indice} total={total} acoes={acoes} />
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="size-6 text-muted-foreground"
          aria-label="Inserir parágrafo abaixo"
          onClick={() => acoes.onParagrafoDepois(no.id)}
        >
          <Plus className="size-3.5" />
        </Button>
        <BotaoRemover id={no.id} acoes={acoes} />
      </div>
    </div>
  );
}

function BlocoVinculo({
  no,
  indice,
  total,
  acoes,
}: {
  no: VinculoDoNo;
  indice: number;
  total: number;
  acoes: Acoes;
}) {
  const rotulo = no.tipo === "citacao" ? "Citação" : "Referência viva";
  const alvo = no.tipo === "citacao" ? no.referenciaId : no.fragmentoId;
  return (
    <div className="flex items-center gap-2 rounded-[10px] border border-dashed border-border/70 px-3 py-2 text-[11.5px] text-muted-foreground">
      <Badge variant="outline" className="shrink-0 text-[10px]">
        {rotulo}
      </Badge>
      <span className="truncate">{alvo}</span>
      <span className="ml-auto hidden shrink-0 sm:inline">vem do acervo · só leitura aqui</span>
      <BotoesDeOrdem id={no.id} indice={indice} total={total} acoes={acoes} />
      <BotaoRemover id={no.id} acoes={acoes} />
    </div>
  );
}

function BotoesDeOrdem({
  id,
  indice,
  total,
  acoes,
}: {
  id: string;
  indice: number;
  total: number;
  acoes: Acoes;
}) {
  return (
    <div className="flex shrink-0 flex-col">
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="size-6 text-muted-foreground"
        disabled={indice === 0}
        aria-label="Mover para cima"
        onClick={() => acoes.onMover(id, "cima")}
      >
        <ArrowUp className="size-3.5" />
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="size-6 text-muted-foreground"
        disabled={indice === total - 1}
        aria-label="Mover para baixo"
        onClick={() => acoes.onMover(id, "baixo")}
      >
        <ArrowDown className="size-3.5" />
      </Button>
    </div>
  );
}

function BotaoRemover({ id, acoes }: { id: string; acoes: Acoes }) {
  return (
    <Button
      type="button"
      variant="ghost"
      size="icon"
      className="size-6 shrink-0 text-muted-foreground hover:text-destructive"
      aria-label="Remover"
      onClick={() => acoes.onRemover(id)}
    >
      <Trash2 className="size-3.5" />
    </Button>
  );
}
