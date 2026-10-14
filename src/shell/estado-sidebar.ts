/**
 * Os três estados da barra lateral, e só as duas transições que existem.
 *
 * **"Compacta" é o estado em que os rótulos saem e os ícones ficam.** Ele existe
 * porque uma barra recolhida até o nada não poupa largura, só tira o acesso: a
 * pessoa passa a depender do botão do topo para saber onde está e o que existe.
 * Ícone é navegação, e esconder o ícone junto com o rótulo troca largura por
 * controle. `SPEC-D-006` D63 já previa a mitigação — "CSS de sidebar, com rótulo
 * recolhível" — para as nove entradas caberem no `minWidth: 1024`, e
 * `docs/performance/02-transicoes-animacoes.md` já media o compasso do estado
 * intermediário em 68px.
 *
 * "Fechada" é do **drawer** do mobile (`max-width: 767px`, o mesmo corte de
 * `useIsMobile`), e é por isso que ela não existe no desktop: a gaveta precisa
 * de um `z-30` de scrim e de um botão de fechar por baixo da mão; a barra do
 * desktop não tem nenhum dos dois, e sem os dois "fechada" é só um vão morto
 * entre a borda da janela e o conteúdo.
 */
export type EstadoSidebar = "aberta" | "compacta" | "fechada";

/** Largura cheia: a de hoje, com rótulo, atalho, badge e rodapé em texto. */
export const LARGURA_ABERTA = 264;

/** Só ícone. 68px é o compasso medido em `02-transicoes-animacoes.md`. */
export const LARGURA_COMPACTA = 68;

/**
 * Largura da barra em pixels, por estado. `0` é o vão morto do drawer fechado,
 * e não uma largura de leitura: a coluna interna continua com 264px para poder
 * deslizar (ver `SidebarNav`).
 *
 * Este número é a **fonte** da largura, e vai para o `<aside>` por `style` e não
 * por classe utilitária. Uma classe `w-[${n}px]` montada por template não é
 * vista pelo Tailwind na varredura do fonte, e a barra viraria `w-` sem valor —
 * isto é, 0px nos três estados, sem erro e sem aviso.
 */
export function larguraDaBarra(estado: EstadoSidebar): number {
  if (estado === "aberta") return LARGURA_ABERTA;
  if (estado === "compacta") return LARGURA_COMPACTA;
  return 0;
}

/**
 * O que o botão do topo faz, e a invariante que ele não pode quebrar:
 * **no desktop, nenhuma transição chega em `"fechada"`.** A pessoa aperta o
 * botão para recuperar espaço de leitura e recebe de volta os ícones; nunca
 * recebe uma borda sem nada dentro.
 */
export function alternarSidebar(estado: EstadoSidebar, isMobile: boolean): EstadoSidebar {
  if (isMobile) return estado === "aberta" ? "fechada" : "aberta";
  return estado === "compacta" ? "aberta" : "compacta";
}

/**
 * Ao cruzar o corte de viewport, o estado precisa ser reconciliado com o
 * desenho: no mobile a barra é gaveta e nasce fechada; ao voltar para o desktop
 * ela nasce aberta, porque "fechada" não tem desenho lá.
 */
export function estadoAoMudarViewport(estado: EstadoSidebar, isMobile: boolean): EstadoSidebar {
  if (isMobile) return "fechada";
  return estado === "fechada" ? "aberta" : estado;
}
