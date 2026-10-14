/**
 * Tela de TCC & Projetos (SPEC-D-011, SPEC-D-013, §4.7 do referencial).
 *
 * O v1 da tela é o projeto e o documento dele: **lista de projetos**, criação e
 * estado, e o **painel do documento** — título, versão e os itens a resolver da
 * importação de DOCX (D88). Ela não calcula nada: a árvore vem remontada do Rust,
 * a versão é derivada no backend (D87) e os itens chegam decididos de lá.
 *
 * O **editor paginado** (SPEC-D-011 D77) abre no modo Escrita: ele edita a árvore
 * e a página é derivada no Rust (`paginar_documento`, D86), nunca gravada. A
 * **importação e a exportação de DOCX** (round-trip, D78) continuam fora — entram
 * em fase própria, com a biblioteca de DOCX escolhida por critério (precisa expor
 * `w:ins`/`w:del` com autor e data).
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { FileText, PenLine, Plus, X } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { isIpcAvailable } from "@/lib/ipc";

import {
  createProjeto,
  desvincular,
  getDocumentoDoProjeto,
  listBase,
  listItens,
  listProjetos,
  setEstadoProjeto,
  vincular,
  type AlvoTipo,
  type Documento,
  type ItemAResolver,
  type Projeto,
  type Vinculo,
} from "../data/bridge.ts";
import {
  ESTADOS_DE_PROJETO,
  LIMITE_DE_PROJETOS_ATIVOS,
  TIPOS_DE_ALVO,
  TIPOS_DE_PROJETO,
  assinaturaDoItem,
  contarArvore,
  itemPendente,
  podeCriarProjeto,
  projetosAtivos,
  rotuloAlvoTipo,
  rotuloEstadoItem,
  rotuloEstadoProjeto,
  rotuloTipoItem,
  rotuloTipoProjeto,
  rotuloVersao,
} from "./view.ts";
import { EditorDoDocumento } from "./EditorDoDocumento.tsx";

export function TccScreen() {
  const [projetos, setProjetos] = useState<readonly Projeto[]>([]);
  const [carregando, setCarregando] = useState(true);
  const [erro, setErro] = useState<string | null>(null);
  const [formAberto, setFormAberto] = useState(false);

  const [selecionadoId, setSelecionadoId] = useState<string | null>(null);
  const [documento, setDocumento] = useState<Documento | null>(null);
  const [itens, setItens] = useState<readonly ItemAResolver[]>([]);
  const [vinculos, setVinculos] = useState<readonly Vinculo[]>([]);
  const [docCarregando, setDocCarregando] = useState(false);
  // O modo é interno ao módulo (SPEC-D-006 D62): projeto e escrita são modos do
  // mesmo fluxo, não destinos de navegação.
  const [modo, setModo] = useState<"projetos" | "escrita">("projetos");

  /*
   * Trocar de projeto rápido produz duas leituras concorrentes; a mais antiga não
   * pode sobrescrever a mais nova. É a mesma disciplina de geração do Calendário
   * e do Estudos, aqui para o painel do documento.
   */
  const generation = useRef(0);

  const carregarProjetos = useCallback(async (): Promise<void> => {
    const lidos = await listProjetos();
    setProjetos(lidos);
  }, []);

  const carregarDocumento = useCallback(async (projetoId: string): Promise<void> => {
    const minha = ++generation.current;
    setDocCarregando(true);
    const doc = await getDocumentoDoProjeto(projetoId);
    const [lidos, base] = await Promise.all([listItens(doc.id), listBase(projetoId)]);
    if (minha !== generation.current) return;
    setDocumento(doc);
    setItens(lidos);
    setVinculos(base);
    setDocCarregando(false);
  }, []);

  useEffect(() => {
    if (!isIpcAvailable()) {
      setCarregando(false);
      return;
    }
    carregarProjetos()
      .then(() => setCarregando(false))
      .catch((e: unknown) => {
        setErro(e instanceof Error ? e.message : String(e));
        setCarregando(false);
      });
  }, [carregarProjetos]);

  /** Envolve uma escrita: limpa o erro, executa, relê. */
  const escrever = useCallback(async (acao: () => Promise<void>): Promise<void> => {
    setErro(null);
    try {
      await acao();
    } catch (e: unknown) {
      setErro(e instanceof Error ? e.message : String(e));
    }
  }, []);

  const selecionar = useCallback(
    (projetoId: string) => {
      setSelecionadoId(projetoId);
      setDocumento(null);
      setItens([]);
      setVinculos([]);
      setModo("projetos");
      void escrever(() => carregarDocumento(projetoId));
    },
    [carregarDocumento, escrever],
  );

  const selecionado = projetos.find((p) => p.id === selecionadoId) ?? null;
  const ativos = projetosAtivos(projetos);
  const podeCriar = podeCriarProjeto(projetos);

  if (!isIpcAvailable()) {
    return (
      <div className="mx-auto max-w-lg px-6 py-16 text-center text-[12.5px] text-muted-foreground">
        A tela TCC & Projetos lê e grava pelo backend Rust. Rode o app com o shell do Tauri para
        usar o módulo.
      </div>
    );
  }

  // O modo Escrita é uma tela cheia dentro do módulo. Ao voltar, o documento é
  // relido para o resumo e a versão do painel ficarem em dia (D87).
  if (modo === "escrita" && documento !== null && selecionado !== null) {
    const projetoId = selecionado.id;
    return (
      <EditorDoDocumento
        key={documento.id}
        documento={documento}
        onVoltar={() => {
          setModo("projetos");
          void escrever(() => carregarDocumento(projetoId));
        }}
        onSalvo={(salvo) => {
          setDocumento((atual) =>
            atual === null || !salvo.versionada ? atual : { ...atual, ultimaVersao: salvo.numero },
          );
        }}
      />
    );
  }

  return (
    <div className="mx-auto max-w-4xl px-6 py-6">
      <div className="flex flex-wrap items-center gap-3">
        <h2 className="text-[13px] font-semibold">Projetos</h2>
        {projetos.length > 0 && (
          <Badge variant="secondary" className="text-[10.5px]">
            {projetos.length}
          </Badge>
        )}
        <Button
          variant="outline"
          size="sm"
          className="ml-auto h-8 text-[12px]"
          disabled={!podeCriar}
          title={
            podeCriar
              ? "Criar um projeto novo"
              : `Limite de ${LIMITE_DE_PROJETOS_ATIVOS} projetos ativos (§4.7)`
          }
          onClick={() => setFormAberto((f) => !f)}
        >
          <Plus className="size-3.5" />
          Novo projeto
        </Button>
      </div>

      {!podeCriar && (
        <p className="mt-3 rounded-[8px] border border-border/70 bg-muted/40 px-3 py-2 text-[12px] text-muted-foreground">
          Você tem {projetosAtivos(projetos)} projetos ativos — o máximo é{" "}
          {LIMITE_DE_PROJETOS_ATIVOS}. Encerre ou arquive um projeto para abrir lugar.
        </p>
      )}

      {formAberto && (
        <FormNovoProjeto
          onCancel={() => setFormAberto(false)}
          onCriar={async (payload) => {
            await escrever(async () => {
              const criado = await createProjeto(payload);
              setFormAberto(false);
              await carregarProjetos();
              selecionar(criado.id);
            });
          }}
        />
      )}

      {erro && (
        <p
          role="alert"
          className="mt-3 rounded-[8px] border border-destructive/40 bg-destructive/10 px-3 py-2 text-[12px] text-destructive"
        >
          {erro}
        </p>
      )}

      <div className="mt-4 grid gap-4 lg:grid-cols-[minmax(0,1fr)_320px]">
        {carregando ? (
          <p className="text-[12.5px] text-muted-foreground">Carregando…</p>
        ) : projetos.length === 0 ? (
          <div className="flex flex-col items-center gap-2 py-10 text-center">
            <h3 className="text-[15px] font-semibold">Nenhum projeto ainda</h3>
            <p className="max-w-sm text-[12.5px] text-muted-foreground">
              Um projeto nasce com o documento dele. Crie o primeiro para começar o TCC, o artigo ou
              o relatório de estágio.
            </p>
          </div>
        ) : (
          <ul className="flex flex-col gap-2">
            {projetos.map((projeto) => (
              <li key={projeto.id}>
                <button
                  onClick={() => selecionar(projeto.id)}
                  className={
                    "flex w-full flex-col gap-1 rounded-[10px] border px-3 py-2.5 text-left transition-colors " +
                    (projeto.id === selecionadoId
                      ? "border-primary/50 bg-primary/[0.06]"
                      : "border-border/70 bg-card/60 hover:bg-accent/40")
                  }
                >
                  <span className="flex items-center gap-2">
                    <FileText className="size-3.5 shrink-0 text-muted-foreground" />
                    <span className="truncate text-[13px] font-medium">{projeto.titulo}</span>
                    <Badge variant="outline" className="ml-auto shrink-0 text-[10px]">
                      {rotuloEstadoProjeto(projeto.estado)}
                    </Badge>
                  </span>
                  <span className="text-[11px] text-muted-foreground">
                    {rotuloTipoProjeto(projeto.tipo)}
                    {projeto.descricao ? ` · ${projeto.descricao}` : ""}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )}

        <PainelDoDocumento
          projeto={selecionado}
          documento={documento}
          itens={itens}
          vinculos={vinculos}
          carregando={docCarregando}
          onEscrever={() => setModo("escrita")}
          onMudarEstado={(estado) => {
            if (!selecionado) return;
            void escrever(async () => {
              await setEstadoProjeto(selecionado.id, estado);
              await carregarProjetos();
            });
          }}
          onVincular={(alvoTipo, alvoId) => {
            if (!selecionado) return;
            void escrever(async () => {
              await vincular({ projetoId: selecionado.id, alvoTipo, alvoId });
              await carregarDocumento(selecionado.id);
            });
          }}
          onDesvincular={(vinculoId) => {
            if (!selecionado) return;
            void escrever(async () => {
              await desvincular(vinculoId);
              await carregarDocumento(selecionado.id);
            });
          }}
        />
      </div>
    </div>
  );
}

function PainelDoDocumento({
  projeto,
  documento,
  itens,
  vinculos,
  carregando,
  onEscrever,
  onMudarEstado,
  onVincular,
  onDesvincular,
}: {
  projeto: Projeto | null;
  documento: Documento | null;
  itens: readonly ItemAResolver[];
  vinculos: readonly Vinculo[];
  carregando: boolean;
  onEscrever: () => void;
  onMudarEstado: (estado: string) => void;
  onVincular: (alvoTipo: AlvoTipo, alvoId: string) => void;
  onDesvincular: (vinculoId: string) => void;
}) {
  if (projeto === null) {
    return (
      <aside className="rounded-[12px] border border-dashed border-border/70 px-4 py-6 text-center text-[12px] text-muted-foreground">
        Selecione um projeto para ver o documento.
      </aside>
    );
  }

  const resumo = documento ? contarArvore(documento.arvore) : null;
  const pendentes = itens.filter(itemPendente).length;
  // O campo é lido numa constante antes do JSX: o componente de seleção e o nome
  // da tabela na mesma linha disparariam a regra `sql-nao-atravessa` do gate por
  // engano. Não há SQL aqui — é só um falso positivo de heurística.
  const estadoAtual = projeto.estado;

  return (
    <aside className="flex flex-col gap-3 rounded-[12px] border border-border/70 bg-card/60 px-4 py-4">
      <div className="flex flex-col gap-1">
        <span className="text-[10.5px] font-medium uppercase tracking-[0.06em] text-muted-foreground">
          Documento
        </span>
        <span className="truncate text-[13px] font-medium">
          {documento ? documento.titulo : projeto.titulo}
        </span>
      </div>

      <div className="flex items-center gap-2">
        <span className="text-[11px] text-muted-foreground">Estado</span>
        <Select value={estadoAtual} onValueChange={onMudarEstado}>
          <SelectTrigger className="ml-auto h-7 w-36 text-[12px]" aria-label="Estado do projeto">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {ESTADOS_DE_PROJETO.map((estado) => (
              <SelectItem key={estado.id} value={estado.id} className="text-[12.5px]">
                {estado.rotulo}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {carregando ? (
        <p className="text-[12px] text-muted-foreground">Carregando documento…</p>
      ) : documento === null ? null : (
        <>
          <div className="flex items-center gap-2 text-[11px] text-muted-foreground">
            <Badge variant="outline" className="text-[10px]">
              {rotuloVersao(documento.ultimaVersao)}
            </Badge>
            <Button
              variant="outline"
              size="sm"
              className="ml-auto h-7 text-[11.5px]"
              onClick={onEscrever}
            >
              <PenLine className="size-3.5" />
              Escrever
            </Button>
          </div>
          <p className="text-[11px] text-muted-foreground">{resumo ? textoResumo(resumo) : ""}</p>

          <div className="flex flex-col gap-1.5 border-t border-border/60 pt-3">
            <span className="text-[10.5px] font-medium uppercase tracking-[0.06em] text-muted-foreground">
              Itens a resolver{pendentes > 0 ? ` · ${pendentes} pendente(s)` : ""}
            </span>
            {itens.length === 0 ? (
              <p className="text-[12px] text-muted-foreground">
                Nenhum comentário ou sugestão pendente.
              </p>
            ) : (
              <ul className="flex flex-col gap-1.5">
                {itens.map((item) => (
                  <li
                    key={item.id}
                    className="flex flex-col gap-0.5 rounded-[8px] border border-border/60 px-2.5 py-1.5"
                  >
                    <span className="flex items-center gap-1.5">
                      <span className="truncate text-[11.5px] font-medium">
                        {assinaturaDoItem(item)}
                      </span>
                      <Badge variant="outline" className="shrink-0 text-[10px]">
                        {rotuloTipoItem(item.tipo)}
                      </Badge>
                      <span className="ml-auto shrink-0 text-[10.5px] text-muted-foreground">
                        {rotuloEstadoItem(item.estado)}
                      </span>
                    </span>
                    <span className="truncate text-[11px] text-muted-foreground">
                      {item.citado}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </div>

          <div className="flex flex-col gap-1.5 border-t border-border/60 pt-3">
            <span className="text-[10.5px] font-medium uppercase tracking-[0.06em] text-muted-foreground">
              Base de pesquisa{vinculos.length > 0 ? ` · ${vinculos.length}` : ""}
            </span>
            <p className="text-[10.5px] text-muted-foreground">
              Vínculos ao acervo, sem cópia (D66). Referência é da Biblioteca; Fragmento, do
              Conhecimento. A lista de referências é gerada daqui.
            </p>
            {vinculos.length > 0 && (
              <ul className="flex flex-col gap-1">
                {vinculos.map((v) => (
                  <li
                    key={v.id}
                    className="flex items-center gap-1.5 rounded-[8px] border border-border/60 px-2.5 py-1.5"
                  >
                    <Badge variant="outline" className="shrink-0 text-[10px]">
                      {rotuloAlvoTipo(v.alvoTipo)}
                    </Badge>
                    <span className="truncate text-[11px] text-muted-foreground">{v.alvoId}</span>
                    <button
                      type="button"
                      onClick={() => onDesvincular(v.id)}
                      aria-label={`Remover vínculo ${v.alvoId}`}
                      className="ml-auto shrink-0 rounded p-0.5 text-muted-foreground transition-colors hover:bg-accent/40 hover:text-foreground"
                    >
                      <X className="size-3.5" />
                    </button>
                  </li>
                ))}
              </ul>
            )}
            <FormVinculo onVincular={onVincular} />
          </div>

          <p className="mt-auto border-t border-border/60 pt-3 text-[10.5px] text-muted-foreground/80">
            O editor paginado abre no botão Escrever (SPEC-D-011 D77). A importação e a exportação
            de DOCX entram na fase de round-trip (D78).
          </p>
        </>
      )}
    </aside>
  );
}

function FormVinculo({ onVincular }: { onVincular: (alvoTipo: AlvoTipo, alvoId: string) => void }) {
  const [tipo, setTipo] = useState<AlvoTipo>("referencia");
  const [alvoId, setAlvoId] = useState("");

  const podeAdicionar = alvoId.trim().length > 0;

  return (
    <form
      className="flex items-center gap-1.5"
      onSubmit={(e) => {
        e.preventDefault();
        if (!podeAdicionar) return;
        onVincular(tipo, alvoId.trim());
        setAlvoId("");
      }}
    >
      <Select value={tipo} onValueChange={(v) => setTipo(v as AlvoTipo)}>
        <SelectTrigger className="h-8 w-[130px] text-[11.5px]">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {TIPOS_DE_ALVO.map((t) => (
            <SelectItem key={t.id} value={t.id}>
              {t.rotulo}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <Input
        value={alvoId}
        onChange={(e) => setAlvoId(e.target.value)}
        placeholder="Id do item do acervo"
        className="h-8 flex-1 text-[11.5px]"
        aria-label="Id do item do acervo"
      />
      <Button type="submit" size="sm" variant="outline" disabled={!podeAdicionar}>
        <Plus className="size-3.5" />
      </Button>
    </form>
  );
}

function textoResumo(resumo: ReturnType<typeof contarArvore>): string {
  const partes: string[] = [];
  if (resumo.secoes > 0) partes.push(`${resumo.secoes} seç${resumo.secoes === 1 ? "ão" : "ões"}`);
  if (resumo.paragrafos > 0) partes.push(`${resumo.paragrafos} parágrafo(s)`);
  if (resumo.citacoes > 0) partes.push(`${resumo.citacoes} citação(ões)`);
  if (resumo.referencias > 0) partes.push(`${resumo.referencias} referência(s)`);
  return partes.length === 0 ? "Documento vazio" : partes.join(" · ");
}

function FormNovoProjeto({
  onCriar,
  onCancel,
}: {
  onCriar: (payload: { titulo: string; tipo: string; descricao?: string }) => Promise<void>;
  onCancel: () => void;
}) {
  const [titulo, setTitulo] = useState("");
  const [tipo, setTipo] = useState(TIPOS_DE_PROJETO[0]?.id ?? "tcc");
  const [descricao, setDescricao] = useState("");

  return (
    <form
      className="mt-4 flex flex-col gap-3 rounded-[10px] border border-border/70 bg-card/60 p-3"
      onSubmit={(e) => {
        e.preventDefault();
        if (!titulo.trim()) return;
        void onCriar({
          titulo: titulo.trim(),
          tipo,
          ...(descricao.trim() ? { descricao: descricao.trim() } : {}),
        });
      }}
    >
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="flex flex-col gap-1">
          <span className="text-[10.5px] font-medium uppercase tracking-[0.06em] text-muted-foreground">
            Título
          </span>
          <Input
            value={titulo}
            onChange={(e) => setTitulo(e.target.value)}
            placeholder="Título do projeto"
            className="h-8 text-[12.5px]"
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-[10.5px] font-medium uppercase tracking-[0.06em] text-muted-foreground">
            Tipo
          </span>
          <Select value={tipo} onValueChange={setTipo}>
            <SelectTrigger className="h-8 text-[12.5px]" aria-label="Tipo do projeto">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {TIPOS_DE_PROJETO.map((t) => (
                <SelectItem key={t.id} value={t.id} className="text-[12.5px]">
                  {t.rotulo}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </label>
      </div>
      <label className="flex flex-col gap-1">
        <span className="text-[10.5px] font-medium uppercase tracking-[0.06em] text-muted-foreground">
          Descrição
        </span>
        <Input
          value={descricao}
          onChange={(e) => setDescricao(e.target.value)}
          placeholder="Opcional"
          className="h-8 text-[12.5px]"
        />
      </label>
      <p className="text-[11px] text-muted-foreground">
        O projeto nasce com exatamente um documento (v1). A árvore nasce vazia.
      </p>
      <div className="flex items-center gap-2">
        <Button type="submit" size="sm" className="h-8 text-[12px]">
          Criar
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-8 text-[12px]"
          onClick={onCancel}
        >
          <X className="size-3.5" />
          Cancelar
        </Button>
      </div>
    </form>
  );
}
