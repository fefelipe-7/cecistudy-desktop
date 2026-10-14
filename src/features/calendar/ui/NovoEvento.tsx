/**
 * Criar evento (slice 4).
 *
 * O formulário é um componente próprio e não um pedaço do `CalendarScreen`, por
 * dois motivos: `CalendarScreen` já é o arquivo mais comprido da tela e este
 * formulário tem estado próprio (oito campos) que não é da grade; e a parte que
 * importa — traduzir os campos em instantes e na linha `RRULE` — mora em
 * `data/novo-evento.ts`, que é pura e testável sem React e sem toolchain.
 *
 * O que fica aqui é renders e `invoke`. O que decide é lá e no Rust.
 */

import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";

import { layerClass, layerTone, LAYERS } from "../domain/layers.ts";
import { createEvent, upsertRecurrence } from "../data/bridge.ts";
import {
  CAMADAS_EDITAVEIS,
  COMPROMISSOS,
  DIAS_SEMANA,
  montarNovoEvento,
  REPETICOES,
  type RepeticaoId,
} from "../data/novo-evento.ts";
import { CALENDAR_ZONE } from "../domain/zone.ts";
import type { Commitment, LayerId } from "../domain/types.ts";

export interface NovoEventoDialogProps {
  readonly aberto: boolean;
  readonly onOpenChange: (aberto: boolean) => void;
  /** Data já visível na grade — o formulário abre no que a usuária está vendo. */
  readonly dataInicial: string;
  readonly horaInicial: string;
  readonly onCriado: () => void;
}

export function NovoEventoDialog({
  aberto,
  onOpenChange,
  dataInicial,
  horaInicial,
  onCriado,
}: NovoEventoDialogProps) {
  const [titulo, setTitulo] = useState("");
  const [data, setData] = useState(dataInicial);
  const [hora, setHora] = useState(horaInicial);
  const [duracao, setDuracao] = useState(60);
  const [camada, setCamada] = useState<LayerId>("faculdade");
  const [compromisso, setCompromisso] = useState<Commitment>("obrigatorio");
  const [repeticao, setRepeticao] = useState<RepeticaoId>("nenhuma");
  const [dias, setDias] = useState<readonly string[]>(["MO"]);
  const [erro, setErro] = useState<string | null>(null);
  const [salvando, setSalvando] = useState(false);

  const salvar = async (): Promise<void> => {
    setErro(null);
    const { payload, rrule } = montarNovoEvento({
      titulo,
      data,
      hora,
      duracaoMin: duracao,
      camada,
      compromisso,
      repeticao,
      dias,
    });

    // A tela evita a ida ao banco no óbvio; **o Rust valida de novo**, porque a
    // regra mora lá. Validação na tela é cortesia, validação no caminho de
    // escrita é a garantia — e é o Rust que recusa o que a tela deixaria passar.
    if (payload.title === "") {
      setErro("O evento precisa de um título.");
      return;
    }

    setSalvando(true);
    try {
      const evento = await createEvent(payload);
      if (rrule !== null) {
        await upsertRecurrence({ eventId: evento.id, rrule, dtstartTz: CALENDAR_ZONE });
      }
      setTitulo("");
      onCriado();
      onOpenChange(false);
    } catch (falha) {
      setErro(falha instanceof Error ? falha.message : String(falha));
    } finally {
      setSalvando(false);
    }
  };

  const alternarDia = (valor: string): void => {
    setDias((atuais) =>
      atuais.includes(valor) ? atuais.filter((d) => d !== valor) : [...atuais, valor],
    );
  };

  return (
    <Dialog open={aberto} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogTitle>Novo evento</DialogTitle>
        <DialogDescription>Onde, quando e com que compromisso.</DialogDescription>

        <div className="flex flex-col gap-3">
          <label className="flex flex-col gap-1 text-[13px]">
            Título
            <input
              autoFocus
              value={titulo}
              onChange={(e) => setTitulo(e.target.value)}
              placeholder="Aula de Cálculo"
              className="border-border/60 bg-transparent rounded-[6px] border px-2 py-1.5 text-[13px]"
            />
          </label>

          <div className="flex gap-2">
            <label className="flex flex-1 flex-col gap-1 text-[13px]">
              Data
              <input
                type="date"
                value={data}
                onChange={(e) => setData(e.target.value)}
                className="border-border/60 bg-transparent rounded-[6px] border px-2 py-1.5 text-[13px]"
              />
            </label>
            <label className="flex flex-1 flex-col gap-1 text-[13px]">
              Hora
              <input
                type="time"
                value={hora}
                onChange={(e) => setHora(e.target.value)}
                className="border-border/60 bg-transparent rounded-[6px] border px-2 py-1.5 text-[13px]"
              />
            </label>
            <label className="flex w-24 flex-col gap-1 text-[13px]">
              Duração (min)
              <input
                type="number"
                min={15}
                step={15}
                value={duracao}
                onChange={(e) => setDuracao(Number.parseInt(e.target.value, 10) || 0)}
                className="border-border/60 bg-transparent rounded-[6px] border px-2 py-1.5 text-[13px]"
              />
            </label>
          </div>

          <fieldset className="flex flex-col gap-1 text-[13px]">
            <legend className="mb-1">Camada</legend>
            <div className="flex flex-wrap gap-1.5">
              {CAMADAS_EDITAVEIS.map((id) => (
                <button
                  key={id}
                  type="button"
                  onClick={() => setCamada(id)}
                  aria-pressed={camada === id}
                  className={`rounded-[6px] border px-2 py-1 text-[12px] ${
                    camada === id ? "border-primary" : "border-border/60 text-muted-foreground"
                  } ${layerClass(layerTone(id))}`}
                >
                  {LAYERS[id].label}
                </button>
              ))}
            </div>
          </fieldset>

          <fieldset className="flex flex-col gap-1 text-[13px]">
            <legend className="mb-1">Compromisso</legend>
            <div className="flex flex-wrap gap-1.5">
              {COMPROMISSOS.map((c) => (
                <button
                  key={c.id}
                  type="button"
                  onClick={() => setCompromisso(c.id)}
                  aria-pressed={compromisso === c.id}
                  className={`rounded-[6px] border px-2 py-1 text-[12px] ${
                    compromisso === c.id
                      ? "border-primary"
                      : "border-border/60 text-muted-foreground"
                  }`}
                >
                  {c.label}
                </button>
              ))}
            </div>
          </fieldset>

          <fieldset className="flex flex-col gap-1 text-[13px]">
            <legend className="mb-1">Repetição</legend>
            <div className="flex flex-wrap gap-1.5">
              {REPETICOES.map((r) => (
                <button
                  key={r.id}
                  type="button"
                  onClick={() => setRepeticao(r.id)}
                  aria-pressed={repeticao === r.id}
                  className={`rounded-[6px] border px-2 py-1 text-[12px] ${
                    repeticao === r.id ? "border-primary" : "border-border/60 text-muted-foreground"
                  }`}
                >
                  {r.label}
                </button>
              ))}
            </div>

            {repeticao === "semanal" && (
              <div className="mt-1 flex gap-1">
                {DIAS_SEMANA.map((d) => (
                  <button
                    key={d.valor}
                    type="button"
                    onClick={() => alternarDia(d.valor)}
                    aria-pressed={dias.includes(d.valor)}
                    aria-label={d.valor}
                    className={`h-7 w-7 rounded-[6px] border text-[12px] ${
                      dias.includes(d.valor)
                        ? "border-primary"
                        : "border-border/60 text-muted-foreground"
                    }`}
                  >
                    {d.curto}
                  </button>
                ))}
              </div>
            )}
          </fieldset>

          {erro !== null && (
            <p role="status" className="text-destructive text-[12px]">
              {erro}
            </p>
          )}

          <div className="flex justify-end gap-2">
            <Button variant="ghost" onClick={() => onOpenChange(false)}>
              Cancelar
            </Button>
            <Button onClick={() => void salvar()} disabled={salvando}>
              {salvando ? "Salvando…" : "Criar evento"}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
