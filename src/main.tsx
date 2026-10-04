import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider, createBrowserRouter } from "react-router";

import { ErrorBoundary } from "./components/ErrorBoundary";
import { ROTAS } from "./shell/routes.tsx";
import "./styles.css";

const container = document.getElementById("root");
if (!container) throw new Error('Elemento "#root" não encontrado em index.html');

/**
 * `RouterProvider` fica **fora** de `ROTAS`, e o `AppShell` é a rota de layout
 * dentro de `ROTAS`. É o que garante `I11` da `SPEC-D-013`: trocar de destino
 * remonta só a árvore de rota, e o shell — barra lateral, barra do topo — não.
 *
 * `react-router` é a decisão de `D84`. A alternativa rejeitada, e por quê, estão
 * em `SPEC-D-013` `## 2`.
 */
const router = createBrowserRouter(ROTAS);

createRoot(container).render(
  <StrictMode>
    <ErrorBoundary>
      <RouterProvider router={router} />
    </ErrorBoundary>
  </StrictMode>,
);
