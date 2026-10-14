/**
 * Tela do Estudos (SPEC-D-010, F8 · §4.4 do referencial).
 *
 * O v1 da tela é o coração de §4.4: a **fila de revisão do dia** e o ato de
 * revisar. Ela não calcula nada — a fila vem `devida` do Rust, o vencimento vem
 * derivado, e a nota vira estado + evento no backend (D76). A retenção desejada
 * é **parâmetro**: o seletor começa em 0,9, que é o padrão **provisório [P]** —
 * §4.4 marca o padrão como decisão da dona do produto, e a tela não esconde isso.
 *
 * Não existe aqui, de propósito: as quatro telas de §4.4 (Hoje · Conteúdos ·
 * Prática · Sessões e Histórico), trilhas, materiais e o modo preparação de
 * prova (**[P]**). Este arquivo é a fila e a revisão.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { Plus, X } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
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

import {
  createCartao,
  createConteudo,
  filaDoDia,
  revisarCartao,
  type CartaoDaFila,
} from "../data/bridge.ts";
import {
  AVALIACOES,
  RETENCAO_PROVISORIA,
  RETENCOES,
  rotuloDaFila,
  textoVencimento,
} from "./view.ts";

export function EstudosScreen() {
  const [fila, setFila] = useState<readonly CartaoDaFila[]>([]);
  const [carregando, setCarregando] = useState(true);
  const [erro, setErro] = useState<string | null>(null);
  const [retencao, setRetencao] = useState(String(RETENCAO_PROVISORIA));
  const [mostrandoResposta, setMostrandoResposta] = useState(false);
  const [formAberto, setFormAberto] = useState(false);

  /*
   * Revisar recarrega a fila; trocas rápidas de retenção no meio da carga não
   * existem aqui, mas a mesma disciplina do Calendário vale: resposta obsoleta
   * não sobrescreve a nova.
   */
  const generation = useRef(0);

  const carregar = useCallback(async (): Promise<void> => {
    const minha = ++generation.current;
    const lida = await filaDoDia();
    if (minha !== generation.current) return;
    setFila(lida);
  }, []);

  useEffect(() => {
    if (!isIpcAvailable()) {
      setCarregando(false);
      return;
    }
    carregar()
      .then(() => setCarregando(false))
      .catch((e: unknown) => {
        setErro(e instanceof Error ? e.message : String(e));
        setCarregando(false);
      });
  }, [carregar]);

  /** Envolve uma escrita: limpa o erro, executa, relê a fila. */
  const escrever = useCallback(
    async (acao: () => Promise<unknown>): Promise<void> => {
      setErro(null);
      try {
        await acao();
        await carregar();
      } catch (e: unknown) {
        setErro(e instanceof Error ? e.message : String(e));
      }
    },
    [carregar],
  );

  const atual = fila[0] ?? null;

  if (!isIpcAvailable()) {
    return (
      <div className="mx-auto max-w-lg px-6 py-16 text-center text-[12.5px] text-muted-foreground">
        A tela Estudos lê e grava pelo backend Rust. Rode o app com o shell do Tauri para usar o
        módulo.
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-2xl px-6 py-6">
      <div className="flex flex-wrap items-center gap-3">
        <h2 className="text-[13px] font-semibold">Fila de revisão de hoje</h2>
        {fila.length > 0 && (
          <Badge variant="secondary" className="text-[10.5px]">
            {fila.length} {fila.length === 1 ? "cartão" : "cartões"}
          </Badge>
        )}
        <div className="ml-auto flex items-center gap-2">
          {atual && (
            <Select value={retencao} onValueChange={setRetencao}>
              <SelectTrigger className="h-8 w-40 text-[12px]" aria-label="Retenção desejada">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {RETENCOES.map((r) => (
                  <SelectItem key={r} value={String(r)} className="text-[12.5px]">
                    retenção {String(r).replace(".", ",")}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
          <Button
            variant="outline"
            size="sm"
            className="h-8 text-[12px]"
            onClick={() => setFormAberto((f) => !f)}
          >
            <Plus className="size-3.5" />
            Novo cartão
          </Button>
        </div>
      </div>
      {atual && (
        <p className="mt-1 text-[11px] text-muted-foreground">
          retenção desejada {retencao.replace(".", ",")} — padrão provisório, decisão da dona do
          produto pendente
        </p>
      )}

      {formAberto && (
        <FormNovoCartao
          onCancel={() => setFormAberto(false)}
          onCriar={async (payload) => {
            await escrever(async () => {
              const conteudo = await createConteudo({ titulo: payload.tituloConteudo });
              await createCartao({
                conteudoId: conteudo.id,
                frente: payload.frente,
                verso: payload.verso,
              });
              setFormAberto(false);
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

      {carregando ? (
        <p className="mt-10 text-center text-[12.5px] text-muted-foreground">Carregando…</p>
      ) : atual === null ? (
        <div className="mt-10 flex flex-col items-center gap-2 text-center">
          <h3 className="text-[15px] font-semibold">Nada para revisar hoje</h3>
          <p className="max-w-sm text-[12.5px] text-muted-foreground">
            Cartões revisados voltam quando o vencimento chega. Crie cartões novos para encontrá-los
            aqui hoje mesmo.
          </p>
        </div>
      ) : (
        <CartaoDeRevisao
          cartao={atual}
          mostrandoResposta={mostrandoResposta}
          onVerResposta={() => setMostrandoResposta(true)}
          onAvaliar={(rating) => {
            setMostrandoResposta(false);
            void escrever(() =>
              revisarCartao({ cartaoId: atual.id, rating, retencao: Number(retencao) }),
            );
          }}
        />
      )}
    </div>
  );
}

function CartaoDeRevisao({
  cartao,
  mostrandoResposta,
  onVerResposta,
  onAvaliar,
}: {
  cartao: CartaoDaFila;
  mostrandoResposta: boolean;
  onVerResposta: () => void;
  onAvaliar: (rating: 1 | 2 | 3 | 4) => void;
}) {
  return (
    <section className="mt-4 flex flex-col rounded-[12px] border border-border/70 bg-card/60 px-5 py-4">
      <header className="flex items-center gap-2">
        <span className="text-[11px] text-muted-foreground">{cartao.tituloConteudo}</span>
        <Badge variant="outline" className="text-[10px]">
          {rotuloDaFila(cartao)}
        </Badge>
        <span className="ml-auto text-[11px] text-muted-foreground">
          {textoVencimento(cartao.vencimento)}
        </span>
      </header>

      <p className="mt-4 text-center text-[15px] font-medium leading-relaxed">{cartao.frente}</p>

      {mostrandoResposta ? (
        <>
          <p className="mt-2 border-t border-border/60 pt-3 text-center text-[13.5px] text-muted-foreground">
            {cartao.verso}
          </p>
          <div className="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-4">
            {AVALIACOES.map((avaliacao) => (
              <Button
                key={avaliacao.id}
                variant="outline"
                size="sm"
                className="h-9 text-[12.5px]"
                onClick={() => onAvaliar(avaliacao.id)}
              >
                {avaliacao.rotulo}
              </Button>
            ))}
          </div>
        </>
      ) : (
        <Button size="sm" className="mx-auto mt-4 h-9 w-40 text-[12.5px]" onClick={onVerResposta}>
          Ver resposta
        </Button>
      )}
    </section>
  );
}

function FormNovoCartao({
  onCriar,
  onCancel,
}: {
  onCriar: (payload: { tituloConteudo: string; frente: string; verso: string }) => Promise<void>;
  onCancel: () => void;
}) {
  const [tituloConteudo, setTituloConteudo] = useState("");
  const [frente, setFrente] = useState("");
  const [verso, setVerso] = useState("");

  return (
    <form
      className="mt-4 flex flex-col gap-3 rounded-[10px] border border-border/70 bg-card/60 p-3"
      onSubmit={(e) => {
        e.preventDefault();
        if (!tituloConteudo.trim() || !frente.trim() || !verso.trim()) return;
        void onCriar({
          tituloConteudo: tituloConteudo.trim(),
          frente: frente.trim(),
          verso: verso.trim(),
        });
      }}
    >
      <div className="flex flex-wrap items-end gap-3">
        <label className="flex w-56 flex-col gap-1">
          <span className="text-[10.5px] font-medium uppercase tracking-[0.06em] text-muted-foreground">
            Conteúdo
          </span>
          <Input
            value={tituloConteudo}
            onChange={(e) => setTituloConteudo(e.target.value)}
            placeholder="Sobre o que é o cartão"
            className="h-8 text-[12.5px]"
          />
        </label>
        <Label className="text-[11px] text-muted-foreground">
          o conteúdo é o tópico (§3.1: Estudos é o dono); o cartão nasce sem passado
        </Label>
      </div>
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="flex flex-col gap-1">
          <span className="text-[10.5px] font-medium uppercase tracking-[0.06em] text-muted-foreground">
            Frente
          </span>
          <Input
            value={frente}
            onChange={(e) => setFrente(e.target.value)}
            placeholder="A pergunta"
            className="h-8 text-[12.5px]"
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-[10.5px] font-medium uppercase tracking-[0.06em] text-muted-foreground">
            Verso
          </span>
          <Input
            value={verso}
            onChange={(e) => setVerso(e.target.value)}
            placeholder="A resposta"
            className="h-8 text-[12.5px]"
          />
        </label>
      </div>
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
