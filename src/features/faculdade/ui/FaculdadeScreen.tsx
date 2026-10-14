/**
 * Tela do Faculdade (SPEC-D-010, F8 · §4.3 do referencial).
 *
 * O papel desta tela no desktop é o de §3.2: **leitura e lançamento rápido**.
 * Ela não calcula nada — média, situação e frequência restante vêm derivadas
 * do Rust (`mediaDaDisciplina`, D74/I1) — e as escritas passam pelos comandos
 * com as guardas de D75 no caminho: o semestre encerrado desabilita estrutura
 * na UI, e se algo escapar, o Rust recusa com erro legível (I7). A nota segue
 * lançável depois de encerrar (I9).
 *
 * Não existe aqui, de propósito: regra de média editável na tela (é
 * Configurações), visão por período letivo do mobile (SPEC-005) e a tela de
 * aulas/frequência por disciplina — a frequência entra no cadastro da
 * disciplina e o resto é leitura.
 */

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { Plus, X } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { isIpcAvailable } from "@/lib/ipc";
import { cn } from "@/lib/utils";

import {
  createAvaliacao,
  createDisciplina,
  createSemestre,
  encerrarSemestre,
  lancarNota,
  listAvaliacoes,
  listDisciplinas,
  listSemestres,
  mediaDaDisciplina,
  type Avaliacao,
  type AvaliacaoComNota,
  type Disciplina,
  type MediaResultado,
  type Semestre,
} from "../data/bridge.ts";

type TipoAvaliacao = Avaliacao["tipo"];

interface NovaAvaliacaoForm {
  readonly disciplinaId: string;
  readonly titulo: string;
  readonly tipo: TipoAvaliacao;
  readonly peso: number;
  readonly valeNota?: boolean;
  readonly plainDate?: string;
}
import {
  TIPOS_AVALIACAO,
  formatarMedia,
  semestreAberto,
  situacaoRotulo,
  textoFrequencia,
  textoNotaNecessaria,
  tipoRotulo,
} from "./view.ts";

/** O estado de leitura de um semestre: disciplinas e a média derivada de cada. */
interface PainelSemestre {
  readonly disciplinas: readonly Disciplina[];
  readonly medias: ReadonlyMap<string, MediaResultado>;
}

const VAZIO: PainelSemestre = { disciplinas: [], medias: new Map() };

export function FaculdadeScreen() {
  const [semestres, setSemestres] = useState<readonly Semestre[]>([]);
  const [semestreId, setSemestreId] = useState<string | null>(null);
  const [painel, setPainel] = useState<PainelSemestre>(VAZIO);
  const [disciplinaId, setDisciplinaId] = useState<string | null>(null);
  const [avaliacoes, setAvaliacoes] = useState<readonly AvaliacaoComNota[]>([]);
  const [carregando, setCarregando] = useState(true);
  const [erro, setErro] = useState<string | null>(null);
  const [formAberto, setFormAberto] = useState<"semestre" | "disciplina" | null>(null);

  /*
   * Troca de semestre rápida produz leituras concorrentes; a resposta que
   * chega por último tem de vencer, e respostas obsoletas têm de morrer. É o
   * mesmo `generation` do Calendário — booleano não distingue a segunda troca
   * da primeira.
   */
  const generation = useRef(0);

  const semestre = semestres.find((s) => s.id === semestreId) ?? null;
  const aberto = semestre !== null && semestreAberto(semestre);
  const disciplina = painel.disciplinas.find((d) => d.id === disciplinaId) ?? null;

  const carregarSemestres = useCallback(async (): Promise<void> => {
    const lista = await listSemestres();
    setSemestres(lista);
    // Sem seleção válida, a primeira leitura fixa o mais recente (a lista já
    // vem ordenada por `starts_on DESC` no Rust).
    setSemestreId((atual) => {
      if (atual !== null && lista.some((s) => s.id === atual)) return atual;
      return lista[0]?.id ?? null;
    });
  }, []);

  const carregarPainel = useCallback(async (id: string): Promise<void> => {
    const minha = ++generation.current;
    const disciplinas = await listDisciplinas(id);
    const medias = new Map<string, MediaResultado>();
    for (const d of disciplinas) {
      medias.set(d.id, await mediaDaDisciplina(d.id));
    }
    if (minha !== generation.current) return;
    setPainel({ disciplinas, medias });
    setDisciplinaId((atual) => {
      if (atual !== null && disciplinas.some((d) => d.id === atual)) return atual;
      return disciplinas[0]?.id ?? null;
    });
  }, []);

  const carregarAvaliacoes = useCallback(async (id: string): Promise<void> => {
    setAvaliacoes(await listAvaliacoes(id));
  }, []);

  /*
   * Toda escrita reler o painel inteiro: a média é derivada de tudo que a
   * escrita tocou, e "recarregar só o que mudou" aqui seria recalcular no
   * front o que o Rust já recalcula — duplicação de regra.
   */
  const aposEscrita = useCallback(async (): Promise<void> => {
    if (semestreId !== null) await carregarPainel(semestreId);
    if (disciplinaId !== null) await carregarAvaliacoes(disciplinaId);
  }, [semestreId, disciplinaId, carregarPainel, carregarAvaliacoes]);

  useEffect(() => {
    if (!isIpcAvailable()) {
      setCarregando(false);
      return;
    }
    carregarSemestres()
      .then(() => setCarregando(false))
      .catch((e: unknown) => {
        setErro(e instanceof Error ? e.message : String(e));
        setCarregando(false);
      });
  }, [carregarSemestres]);

  useEffect(() => {
    if (!isIpcAvailable() || semestreId === null) {
      setPainel(VAZIO);
      setDisciplinaId(null);
      return;
    }
    carregarPainel(semestreId).catch((e: unknown) =>
      setErro(e instanceof Error ? e.message : String(e)),
    );
  }, [semestreId, carregarPainel]);

  useEffect(() => {
    if (!isIpcAvailable() || disciplinaId === null) {
      setAvaliacoes([]);
      return;
    }
    carregarAvaliacoes(disciplinaId).catch((e: unknown) =>
      setErro(e instanceof Error ? e.message : String(e)),
    );
  }, [disciplinaId, carregarAvaliacoes]);

  /** Envolve uma escrita: limpa o erro, executa, relê, e mostra o erro do Rust. */
  const escrever = useCallback(
    async (acao: () => Promise<unknown>): Promise<void> => {
      setErro(null);
      try {
        await acao();
        await carregarSemestres();
        await aposEscrita();
      } catch (e: unknown) {
        setErro(e instanceof Error ? e.message : String(e));
      }
    },
    [carregarSemestres, aposEscrita],
  );

  if (!isIpcAvailable()) {
    return (
      <div className="mx-auto max-w-lg px-6 py-16 text-center text-[12.5px] text-muted-foreground">
        A tela Faculdade lê e grava pelo backend Rust. Rode o app com o shell do Tauri para usar o
        módulo.
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-5xl px-6 py-6">
      <div className="flex flex-wrap items-center gap-3">
        <Select
          {...(semestreId !== null ? { value: semestreId } : {})}
          onValueChange={(id) => {
            setDisciplinaId(null);
            setSemestreId(id);
          }}
        >
          <SelectTrigger className="h-8 w-52 text-[12.5px]">
            <SelectValue placeholder="Nenhum semestre" />
          </SelectTrigger>
          <SelectContent>
            {semestres.map((s) => (
              <SelectItem key={s.id} value={s.id} className="text-[12.5px]">
                {s.rotulo}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        {semestre && (
          <Badge variant={aberto ? "secondary" : "outline"} className="text-[10.5px]">
            {aberto ? "aberto" : "encerrado"}
          </Badge>
        )}
        <div className="ml-auto flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            className="h-8 text-[12px]"
            onClick={() => setFormAberto((f) => (f === "semestre" ? null : "semestre"))}
          >
            <Plus className="size-3.5" />
            Novo semestre
          </Button>
          {semestre && aberto && (
            <Button
              variant="outline"
              size="sm"
              className="h-8 text-[12px]"
              onClick={() => {
                const confirmado = window.confirm(
                  `Encerrar "${semestre.rotulo}"? A regra de média de cada disciplina congela, e disciplinas e avaliações não mudam mais. Notas continuam lançáveis.`,
                );
                if (confirmado) {
                  void escrever(() => encerrarSemestre(semestre.id));
                }
              }}
            >
              Encerrar semestre
            </Button>
          )}
        </div>
      </div>

      {formAberto === "semestre" && (
        <FormSemestre
          onCancel={() => setFormAberto(null)}
          onSalvar={(payload) =>
            escrever(async () => {
              const s = await createSemestre(payload);
              setSemestreId(s.id);
              setFormAberto(null);
            })
          }
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

      {carregando ? (
        <p className="mt-10 text-center text-[12.5px] text-muted-foreground">Carregando…</p>
      ) : semestres.length === 0 ? (
        <EmptyState
          titulo="Nenhum semestre ainda"
          texto="Crie o primeiro semestre para começar a cadastrar disciplinas."
        />
      ) : (
        <div className="mt-5 grid gap-4 lg:grid-cols-[280px_minmax(0,1fr)]">
          <ListaDisciplinas
            painel={painel}
            selecionada={disciplinaId}
            aberto={aberto}
            onSelect={setDisciplinaId}
            onNova={() => setFormAberto((f) => (f === "disciplina" ? null : "disciplina"))}
            formAberto={formAberto === "disciplina"}
            onCancelarForm={() => setFormAberto(null)}
            onSalvarForm={(nome) =>
              semestre &&
              escrever(async () => {
                await createDisciplina({ semestreId: semestre.id, nome });
                setFormAberto(null);
              })
            }
          />
          <DetalleDisciplina
            disciplina={disciplina}
            media={disciplina ? (painel.medias.get(disciplina.id) ?? null) : null}
            avaliacoes={avaliacoes}
            aberto={aberto}
            onLancarNota={(avaliacaoId, valor) => escrever(() => lancarNota(avaliacaoId, valor))}
            onNovaAvaliacao={(payload) => disciplina && escrever(() => createAvaliacao(payload))}
          />
        </div>
      )}
    </div>
  );
}

function EmptyState({ titulo, texto }: { titulo: string; texto: string }) {
  return (
    <div className="mt-10 flex flex-col items-center gap-2 text-center">
      <h2 className="text-[15px] font-semibold">{titulo}</h2>
      <p className="max-w-sm text-[12.5px] text-muted-foreground">{texto}</p>
    </div>
  );
}

function FormSemestre({
  onSalvar,
  onCancel,
}: {
  onSalvar: (payload: { rotulo: string; startsOn: string; endsOn: string }) => void;
  onCancel: () => void;
}) {
  const [rotulo, setRotulo] = useState("");
  const [startsOn, setStartsOn] = useState("");
  const [endsOn, setEndsOn] = useState("");

  return (
    <form
      className="mt-4 flex flex-wrap items-end gap-3 rounded-[10px] border border-border/70 bg-card/60 p-3"
      onSubmit={(e) => {
        e.preventDefault();
        if (!rotulo.trim() || !startsOn || !endsOn) return;
        onSalvar({ rotulo: rotulo.trim(), startsOn, endsOn });
      }}
    >
      <Campo rotulo="Rótulo" className="w-28">
        <Input
          value={rotulo}
          onChange={(e) => setRotulo(e.target.value)}
          placeholder="2026-2"
          className="h-8 text-[12.5px]"
        />
      </Campo>
      <Campo rotulo="Começa">
        <Input
          type="date"
          value={startsOn}
          onChange={(e) => setStartsOn(e.target.value)}
          className="h-8 w-36 text-[12.5px]"
        />
      </Campo>
      <Campo rotulo="Termina">
        <Input
          type="date"
          value={endsOn}
          onChange={(e) => setEndsOn(e.target.value)}
          className="h-8 w-36 text-[12.5px]"
        />
      </Campo>
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

function Campo({
  rotulo,
  children,
  className,
}: {
  rotulo: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <label className={cn("flex flex-col gap-1", className)}>
      <span className="text-[10.5px] font-medium uppercase tracking-[0.06em] text-muted-foreground">
        {rotulo}
      </span>
      {children}
    </label>
  );
}

function ListaDisciplinas({
  painel,
  selecionada,
  aberto,
  onSelect,
  onNova,
  formAberto,
  onCancelarForm,
  onSalvarForm,
}: {
  painel: PainelSemestre;
  selecionada: string | null;
  aberto: boolean;
  onSelect: (id: string) => void;
  onNova: () => void;
  formAberto: boolean;
  onCancelarForm: () => void;
  onSalvarForm: (nome: string) => void;
}) {
  return (
    <div className="flex min-w-0 flex-col rounded-[10px] border border-border/70 bg-card/60">
      <div className="flex items-center justify-between px-3 py-2">
        <span className="text-[10.5px] font-semibold uppercase tracking-[0.08em] text-muted-foreground/80">
          Disciplinas
        </span>
        {aberto && (
          <Button variant="ghost" size="sm" className="h-7 px-2 text-[11.5px]" onClick={onNova}>
            <Plus className="size-3" />
            Nova
          </Button>
        )}
      </div>
      {formAberto && (
        <form
          className="flex items-center gap-2 px-3 pb-2"
          onSubmit={(e) => {
            e.preventDefault();
            const nome = new FormData(e.currentTarget).get("nome");
            if (typeof nome === "string" && nome.trim()) onSalvarForm(nome.trim());
          }}
        >
          <Input
            name="nome"
            placeholder="Nome da disciplina"
            className="h-8 text-[12.5px]"
            autoFocus
          />
          <Button type="submit" size="sm" className="h-8 text-[12px]">
            Criar
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-8 px-2 text-[12px]"
            onClick={onCancelarForm}
          >
            <X className="size-3.5" />
          </Button>
        </form>
      )}
      <div className="flex-1 overflow-auto px-1.5 pb-1.5">
        {painel.disciplinas.length === 0 ? (
          <p className="px-2 py-4 text-[12px] text-muted-foreground">
            Nenhuma disciplina neste semestre ainda.
          </p>
        ) : (
          painel.disciplinas.map((d) => {
            const media = painel.medias.get(d.id);
            return (
              <button
                key={d.id}
                onClick={() => onSelect(d.id)}
                className={cn(
                  "flex w-full items-center justify-between gap-2 rounded-[8px] px-2.5 py-2 text-left hover:bg-accent/60",
                  selecionada === d.id && "bg-accent",
                )}
              >
                <span className="min-w-0">
                  <span className="block truncate text-[12.5px] font-medium">{d.nome}</span>
                  <span className="block text-[11px] text-muted-foreground">
                    {media ? situacaoRotulo(media.situacao) : "…"}
                  </span>
                </span>
                <span className="shrink-0 text-[13px] font-semibold tabular-nums">
                  {media ? formatarMedia(media.media) : "—"}
                </span>
              </button>
            );
          })
        )}
      </div>
    </div>
  );
}

function DetalleDisciplina({
  disciplina,
  media,
  avaliacoes,
  aberto,
  onLancarNota,
  onNovaAvaliacao,
}: {
  disciplina: Disciplina | null;
  media: MediaResultado | null;
  avaliacoes: readonly AvaliacaoComNota[];
  aberto: boolean;
  onLancarNota: (avaliacaoId: string, valor: number) => void;
  onNovaAvaliacao: (payload: NovaAvaliacaoForm) => void;
}) {
  if (disciplina === null) {
    return (
      <div className="grid place-items-center rounded-[10px] border border-dashed border-border/70 text-[12.5px] text-muted-foreground">
        Selecione uma disciplina para ver avaliações e lançar notas.
      </div>
    );
  }

  return (
    <div className="min-w-0 rounded-[10px] border border-border/70 bg-card/60">
      <div className="flex flex-wrap items-baseline gap-3 border-b border-border/70 px-4 py-3">
        <h2 className="text-[15px] font-semibold">{disciplina.nome}</h2>
        {disciplina.codigo && (
          <span className="text-[11.5px] text-muted-foreground">{disciplina.codigo}</span>
        )}
        <span className="ml-auto flex items-baseline gap-2">
          <span className="text-[22px] font-semibold tabular-nums">
            {media ? formatarMedia(media.media) : "—"}
          </span>
          {media && (
            <Badge
              variant={media.situacao === "aprovada" ? "secondary" : "outline"}
              className="text-[10.5px]"
            >
              {situacaoRotulo(media.situacao)}
            </Badge>
          )}
        </span>
      </div>
      {media && (
        <p className="px-4 pt-2 text-[11.5px] text-muted-foreground">
          {textoNotaNecessaria(media) ?? `regra ${media.regraAplicada}`}
          {textoFrequencia(disciplina, media) && (
            <>
              {" · "}
              {textoFrequencia(disciplina, media)}
            </>
          )}
        </p>
      )}

      <ul className="flex flex-col gap-1.5 px-3 py-3">
        {avaliacoes.length === 0 && (
          <li className="px-1 py-2 text-[12px] text-muted-foreground">
            Nenhuma avaliação nesta disciplina.
          </li>
        )}
        {avaliacoes.map((av) => (
          <LinhaAvaliacao key={av.id} avaliacao={av} onLancarNota={onLancarNota} />
        ))}
      </ul>

      {aberto && <FormAvaliacao disciplinaId={disciplina.id} onSalvar={onNovaAvaliacao} />}
      {!aberto && (
        <p className="px-4 pb-3 text-[11px] text-muted-foreground">
          Semestre encerrado: a estrutura não muda mais (I7). Lançar e corrigir nota continua aberto
          (I9).
        </p>
      )}
    </div>
  );
}

function LinhaAvaliacao({
  avaliacao,
  onLancarNota,
}: {
  avaliacao: AvaliacaoComNota;
  onLancarNota: (avaliacaoId: string, valor: number) => void;
}) {
  const [valor, setValor] = useState(() =>
    avaliacao.nota ? String(avaliacao.nota.valor).replace(".", ",") : "",
  );

  return (
    <li className="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-[8px] px-2 py-1.5 hover:bg-accent/40">
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[12.5px] font-medium">{avaliacao.titulo}</span>
        <span className="block text-[11px] text-muted-foreground">
          {tipoRotulo(avaliacao.tipo)} · peso {formatarMedia(avaliacao.peso)}
          {!avaliacao.valeNota && " · não vale nota"}
        </span>
      </span>
      {avaliacao.valeNota && (
        <form
          className="flex items-center gap-1.5"
          onSubmit={(e) => {
            e.preventDefault();
            const numero = Number(valor.replace(",", "."));
            if (Number.isFinite(numero)) onLancarNota(avaliacao.id, numero);
          }}
        >
          <Input
            inputMode="decimal"
            value={valor}
            onChange={(e) => setValor(e.target.value)}
            placeholder="nota"
            aria-label={`Nota de ${avaliacao.titulo}`}
            className="h-7 w-20 text-[12px] tabular-nums"
          />
          <Button type="submit" variant="outline" size="sm" className="h-7 px-2.5 text-[11.5px]">
            {avaliacao.nota ? "Corrigir" : "Lançar"}
          </Button>
        </form>
      )}
      {!avaliacao.valeNota && avaliacao.nota === null && (
        <span className="text-[11px] text-muted-foreground">sem nota — não vale nota</span>
      )}
    </li>
  );
}

function FormAvaliacao({
  disciplinaId,
  onSalvar,
}: {
  disciplinaId: string;
  onSalvar: (payload: {
    disciplinaId: string;
    titulo: string;
    tipo: "prova" | "trabalho" | "apresentacao" | "entrega" | "outro";
    peso: number;
    valeNota?: boolean;
    plainDate?: string;
  }) => void;
}) {
  const [titulo, setTitulo] = useState("");
  const [tipo, setTipo] = useState<TipoAvaliacao>("prova");
  const [peso, setPeso] = useState("1");
  const [valeNota, setValeNota] = useState(true);
  const [plainDate, setPlainDate] = useState("");

  return (
    <form
      className="flex flex-wrap items-end gap-3 border-t border-border/70 px-4 py-3"
      onSubmit={(e) => {
        e.preventDefault();
        const pesoNumero = Number(peso.replace(",", "."));
        if (!titulo.trim() || !Number.isFinite(pesoNumero)) return;
        onSalvar({
          disciplinaId,
          titulo: titulo.trim(),
          tipo,
          peso: pesoNumero,
          ...(valeNota ? {} : { valeNota: false }),
          ...(plainDate ? { plainDate } : {}),
        });
        setTitulo("");
        setPlainDate("");
      }}
    >
      <Campo rotulo="Nova avaliação" className="min-w-40 flex-1">
        <Input
          value={titulo}
          onChange={(e) => setTitulo(e.target.value)}
          placeholder="Título"
          className="h-8 text-[12.5px]"
        />
      </Campo>
      <Campo rotulo="Tipo" className="w-36">
        <Select value={tipo} onValueChange={(v) => setTipo(v as TipoAvaliacao)}>
          <SelectTrigger className="h-8 text-[12.5px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {TIPOS_AVALIACAO.map((t) => (
              <SelectItem key={t.id} value={t.id} className="text-[12.5px]">
                {t.rotulo}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Campo>
      <Campo rotulo="Peso" className="w-20">
        <Input
          inputMode="decimal"
          value={peso}
          onChange={(e) => setPeso(e.target.value)}
          className="h-8 text-[12.5px] tabular-nums"
        />
      </Campo>
      <Campo rotulo="Data" className="w-36">
        <Input
          type="date"
          value={plainDate}
          onChange={(e) => setPlainDate(e.target.value)}
          className="h-8 text-[12.5px]"
        />
      </Campo>
      <label className="flex items-center gap-1.5 pb-1.5 text-[12px] text-muted-foreground">
        <Checkbox checked={valeNota} onCheckedChange={(v) => setValeNota(v === true)} />
        vale nota
      </label>
      <Button type="submit" size="sm" className="h-8 text-[12px]">
        Adicionar
      </Button>
    </form>
  );
}
