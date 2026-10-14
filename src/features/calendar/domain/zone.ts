/**
 * Fuso do Calendário (D6).
 *
 * D6 diz: armazenamento em UTC, regra de recorrência em IANA, e **nunca** ajuste
 * automático quando a máquina muda de fuso. Isso não significa que o app não
 * tenha um fuso — significa que ele tem *um*, declarado uma vez aqui, em vez de
 * ser lido do `Intl.DateTimeFormat().resolvedOptions()` em cada lugar.
 *
 * O literal vive num módulo só porque dois grupos de código precisam concordar
 * sobre ele e nenhum dos dois pode ser dono: a tela formata horário com ele, e os
 * formatadores de `ui/format.ts` formatam com o mesmo. Duplicar a string nesses
 * dois lugares seria o tipo de divergência que só aparece quando a usuária está
 * em viagem — que é exatamente o caso que D6 existe para proteger.
 */

import { ianaTimeZone, type IanaTimeZone } from "./time.ts";

export const CALENDAR_ZONE: IanaTimeZone = ianaTimeZone("America/Sao_Paulo");
