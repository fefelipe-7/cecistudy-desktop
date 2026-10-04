import { Navigate, type RouteObject } from "react-router";
import { CalendarScreen } from "@/features/calendar/ui/CalendarScreen.tsx";
import { AppShell } from "./AppShell.tsx";
import { MODULES, UTILIDADES, type ModuleDestination, type ModuleEntry } from "./modules.ts";
import { DestinoNaoImplementado } from "./DestinoNaoImplementado.tsx";

/**
 * Rotas derivadas do registro de módulos (`SPEC-D-013` `D84`).
 *
 * Não existe lista de rotas aqui. A lista é montada a partir de `MODULES` e
 * `UTILIDADES`, o que faz a invariante `I3` — toda rota do registro existe, e
 * toda rota declarada existe no registro — verdadeira por **construção**, e não
 * por revisão. É a diferença entre uma lista que alguém mantém e uma lista que
 * não tem como divergir.
 *
 * `react-router` é a decisão de `D84`. A alternativa rejeitada — máquina de
 * estados própria, mantendo o `useState<View>` que o antigo `src/App.tsx` usava —
 * está escrita na spec e é executável.
 */

/**
 * Destinos com implementação, em `módulo/destino`.
 *
 * Só o Calendário · Semana existe: é a única pasta em `src/features/` e a única
 * tela com dado real. Todo o resto renderiza `DestinoNaoImplementado`, que **diz
 * qual `SPEC-D` falta** em vez de mostrar tela vazia com placeholder — a
 * diferença entre "acabou de ser construído" e "existe" é a informação que a
 * usuária precisa para não procurar o que não está lá.
 */
const IMPLEMENTADOS = new Set(["calendario/semana"]);

function estaImplementado(modulo: ModuleEntry, destino: ModuleDestination): boolean {
  return IMPLEMENTADOS.has(`${modulo.id}/${destino.id}`);
}

function elementoDoDestino(modulo: ModuleEntry, destino: ModuleDestination) {
  if (estaImplementado(modulo, destino)) {
    return <CalendarScreen onOpenList={() => undefined} />;
  }
  return <DestinoNaoImplementado modulo={modulo.rotulo} destino={destino} />;
}

function semBarraInicial(rota: string): string {
  return rota.replace(/^\/+/, "");
}

const INICIO = MODULES[0]!.rota;

/**
 * A árvore de rota do app.
 *
 * O `AppShell` é a rota de layout: ela fica **fora** de `RouterProvider` e cada
 * destino é a filha, o que garante `I11` — trocar de destino remonta só a filha, e
 * a barra lateral e a barra do topo ficam de pé.
 */
export const ROTAS: RouteObject[] = [
  {
    element: <AppShell />,
    children: [
      { index: true, element: <Navigate to={INICIO} replace /> },
      ...MODULES.flatMap((modulo) =>
        modulo.destinos.map((destino) => ({
          path: semBarraInicial(destino.rota),
          element: elementoDoDestino(modulo, destino),
        })),
      ),
      ...UTILIDADES.map((utilidade) => ({
        path: semBarraInicial(utilidade.rota),
        element: (
          <DestinoNaoImplementado
            modulo={utilidade.rotulo}
            destino={{
              id: utilidade.id,
              rotulo: utilidade.rotulo,
              rota: utilidade.rota,
              spec: utilidade.spec,
            }}
          />
        ),
      })),
      { path: "*", element: <Navigate to={INICIO} replace /> },
    ],
  },
];
