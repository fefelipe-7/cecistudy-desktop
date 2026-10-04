import { useEffect, useState } from "react";
import { NavLink, Outlet, useLocation } from "react-router";
import { PanelLeft } from "lucide-react";

import { Button } from "@/components/ui/button";
import { BarraLateral } from "./BarraLateral.tsx";
import { ICONES_POR_UTILIDADE } from "./icones.ts";
import { UTILIDADES, moduloDaRota } from "./modules.ts";

/**
 * O shell do Workspace Acadêmico.
 *
 * `SPEC-D-006` `D63` desenha a sidebar, que está em `BarraLateral.tsx`; o que
 * este arquivo garante é o que `SPEC-D-013` `D83` e `D84` pedem:
 *
 * - **a lista vem do registro** e de nenhum outro lugar (`D83`);
 * - **trocar de rota não remonta o shell** (`I11`), porque o shell fica fora do
 *   `RouterProvider` e cada destino é renderizado pelo `Outlet`;
 * - **a ordem é a de §2 linha 60**, que é `[P]` — e ela é lida do registro como
 *   dado, não escolhida aqui.
 *
 * O estado de recolher a barra mora aqui, e não dentro de `BarraLateral`, por um
 * motivo que é o mesmo do `I11`: quem manda recolher é o botão do topo, e o
 * componente da barra não deve ser dono de um estado que o botão também mexe.
 */
export function AppShell() {
  const [recolhida, setRecolhida] = useState(false);

  useEffect(() => {
    /**
     * `⌘B` / `Ctrl+B` recolhe e expande.
     *
     * O atalho é o que as duas referências convergesem: o shadcn usa `"b"` e o VS
     * Code usa `CtrlCmd + KeyB`. Sem ele, o botão do topo é a única forma de
     * recolher, e recolher é a operação que libera espaço para a grade — logo, ela
     * precisa ser de uma tecla.
     *
     * A guarda de alvo não é defensiva: digitar "b" num campo de busca recolheria a
     * barra, e `event.preventDefault()` incondicional é o que faz isso acontecer. O
     * primitivo do shadcn tem esse bug.
     */
    function aoTeclar(evento: KeyboardEvent) {
      if (!(evento.metaKey || evento.ctrlKey) || evento.key.toLowerCase() !== "b") return;
      const alvo = evento.target as HTMLElement | null;
      const digitando =
        alvo?.tagName === "INPUT" ||
        alvo?.tagName === "TEXTAREA" ||
        alvo?.isContentEditable === true;
      if (digitando) return;
      evento.preventDefault();
      setRecolhida((valor) => !valor);
    }

    window.addEventListener("keydown", aoTeclar);
    return () => window.removeEventListener("keydown", aoTeclar);
  }, []);

  return (
    <div className="flex h-dvh w-full overflow-hidden bg-background text-foreground">
      <BarraLateral recolhida={recolhida} />
      <div className="flex min-w-0 flex-1 flex-col">
        <BarraTopo recolhida={recolhida} aoAlternar={() => setRecolhida((valor) => !valor)} />
        <main className="min-h-0 flex-1 overflow-auto">
          <Outlet />
        </main>
      </div>
    </div>
  );
}

function BarraTopo({ recolhida, aoAlternar }: { recolhida: boolean; aoAlternar: () => void }) {
  return (
    <header className="flex h-11 shrink-0 items-center gap-2 border-b border-border/60 px-3">
      <Button
        aria-label={recolhida ? "Expandir barra lateral" : "Recolher barra lateral"}
        aria-expanded={!recolhida}
        aria-controls="barra-lateral"
        variant="ghost"
        size="icon"
        onClick={aoAlternar}
        title={`${recolhida ? "Expandir" : "Recolher"} barra lateral (⌘B / Ctrl+B)`}
      >
        <PanelLeft className="size-4" aria-hidden />
      </Button>
      <CaminhoDoDestino />
      <div className="ml-auto flex items-center gap-1">
        {/* `posicao: "fora"` — §2 linha 60 põe Configurações / Perfil no topo e
            não no rodapé da sidebar. */}
        {UTILIDADES.filter((u) => u.posicao === "fora").map((utilidade) => {
          const Icone = ICONES_POR_UTILIDADE[utilidade.id];
          return (
            <NavLink key={utilidade.id} to={utilidade.rota}>
              <Button aria-label={utilidade.rotulo} variant="ghost" size="icon">
                <Icone className="size-4" aria-hidden />
              </Button>
            </NavLink>
          );
        })}
      </div>
    </header>
  );
}

/**
 * O caminho do destino atual, lido da rota.
 *
 * O rótulo vem do registro e não de uma segunda lista. É a consequência direta de
 * `D83`: se o nome do destino também morasse aqui, ele voltaria a ter dois donos.
 * Só este componente remonta quando a rota muda — o shell não.
 */
function CaminhoDoDestino() {
  const { pathname } = useLocation();
  const modulo = moduloDaRota(pathname);
  const destino = modulo?.destinos.find((d) => d.rota === pathname);
  const titulo = destino?.rotulo ?? modulo?.rotulo ?? "Campus";
  const temNivel = Boolean(modulo && destino && destino.rotulo !== modulo.rotulo);

  return (
    <div className="flex items-center gap-2 text-sm">
      {temNivel ? (
        <>
          <span className="text-muted-foreground">{modulo?.rotulo}</span>
          <span className="text-muted-foreground/50" aria-hidden>
            /
          </span>
        </>
      ) : null}
      <span className="font-medium">{titulo}</span>
    </div>
  );
}
