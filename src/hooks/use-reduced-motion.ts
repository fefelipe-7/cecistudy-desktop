/**
 * `prefers-reduced-motion` para código JavaScript (DM6 · R5).
 *
 * Existe porque o reset em CSS **não alcança JavaScript**. `transition-duration:
 * 0.01ms` vale para transições CSS; uma animação feita com a Web Animations API,
 * um `setTimeout` de choreography ou `startViewTransition` continua rodando
 * normalmente. O WebView2 é Chromium e tem o WAAPI, então essa é uma lacuna real
 * e não uma precaução.
 *
 * O hook escuta a mudança em vez de ler uma vez no mount: quem liga o sistema
 * de movimento reduzido enquanto o app está aberto é uma coisa que acontece —
 * é um ajuste do sistema, e ajustá-lo deve valer imediatamente.
 *
 * Sem dependência. `motion` exporta um `useReducedMotion` equivalente, mas
 * `DM1` fecha a tecnologia em CSS + WAAPI, e um hook de 30 linhas não é motivo
 * para 34 kB.
 */

import { useEffect, useState } from "react";

const QUERY = "(prefers-reduced-motion: reduce)";

function currentPreference(): boolean {
  // `matchMedia` não existe em ambiente de teste sem DOM, e `AGENTS.md:17` exige
  // que a regra seja verificável fora do browser: sem `window`, a resposta
  // conservadora é "não reduzir", que é o comportamento de sempre.
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return false;
  return window.matchMedia(QUERY).matches;
}

/**
 * `true` quando a usuária pediu menos movimento.
 *
 * Leia-o **antes** de animar, nunca depois: animar e depois corrigir produz o
 * quadro mais longo, que é exatamente o quadro que se queria evitar.
 */
export function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState(currentPreference);

  useEffect(() => {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") return;
    const list = window.matchMedia(QUERY);
    const onChange = (event: MediaQueryListEvent): void => setReduced(event.matches);
    setReduced(list.matches);
    list.addEventListener("change", onChange);
    return () => list.removeEventListener("change", onChange);
  }, []);

  return reduced;
}
