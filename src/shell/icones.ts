import {
  Bell,
  BookOpen,
  Boxes,
  BrainCircuit,
  CalendarDays,
  GraduationCap,
  Home,
  Layers,
  Search,
  Settings,
  Sparkles,
  Stethoscope,
  type LucideIcon,
} from "lucide-react";
import type { ModuleId, UtilityId } from "./modules.ts";

/**
 * Ícone por módulo.
 *
 * O ícone é **apresentação**, não domínio: ele não vai para o registro porque
 * `modules.ts` é importado por `scripts/check-modules.mjs`, que roda em Node sem
 * DOM. Se o ícone entrasse no registro, o gate dependeria de um pacote de
 * React e pararia de rodar onde não há bundler — que é justamente onde ele
 * precisa rodar.
 *
 * `SPEC-D-006` `D63` é quem decide o ícone final.
 */
export const ICONES_POR_MODULO: Record<ModuleId, LucideIcon> = {
  home: Home,
  calendario: CalendarDays,
  faculdade: GraduationCap,
  estudos: Layers,
  estagio: Stethoscope,
  tcc: BookOpen,
  "sala-de-treino": Stethoscope,
  conhecimento: BrainCircuit,
  biblioteca: Boxes,
};

/**
 * Ícone por utilitário global, pelo mesmo motivo do mapa de módulos.
 *
 * `configuracoes` entra no mapa mesmo estando `posicao: "fora"` na sidebar: o
 * registro decide **onde** a entrada aparece, não se ela tem ícone, e um mapa com
 * cobertura apenas do que hoje está desenhado é um mapa que divergem no primeiro
 * dia em que a entrada muda de lugar.
 */
export const ICONES_POR_UTILIDADE: Record<UtilityId, LucideIcon> = {
  ceci: Sparkles,
  busca: Search,
  notificacoes: Bell,
  configuracoes: Settings,
};
