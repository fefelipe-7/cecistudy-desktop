import { ChevronRight } from "lucide-react";
import { useState, type ReactNode } from "react";

import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

import { larguraDaBarra, type EstadoSidebar } from "./estado-sidebar";

export type SidebarItem = {
  id: string;
  label: string;
  icon: ReactNode;
  badge?: string;
  shortcut?: string;
  disabled?: boolean;
  children?: SidebarItem[];
};

export type SidebarGroup = {
  heading?: ReactNode;
  action?: ReactNode;
  items: SidebarItem[];
};

export type SidebarWorkspace = {
  label: string;
  sublabel?: string;
  icon: ReactNode;
};

export type SidebarNavProps = {
  estado: EstadoSidebar;
  workspace: SidebarWorkspace;
  groups: SidebarGroup[];
  footerItems?: SidebarItem[];
  footer?: ReactNode;
  footerCompacto?: ReactNode;
  activeId: string;
  onSelect: (id: string) => void;
};

const contemAtivo = (item: SidebarItem, activeId: string): boolean =>
  item.id === activeId || (item.children?.some((filho) => contemAtivo(filho, activeId)) ?? false);

/** Recuo do filho em relação ao pai: 16px por nível, no texto e no ícone. */
const RECUO = 16;

/** Marca de seleção da linha selecionada: a barra de 2px à esquerda. */
const MARCA_SELECAO =
  "after:absolute after:top-1/2 after:left-0 after:h-4 after:w-[2.5px] after:-translate-y-1/2 after:rounded-full after:bg-primary";

/** A mesma marca, do lado de dentro da linha: é o que a barra compacta usa. */
const MARCA_SELECAO_DIREITA =
  "after:absolute after:top-1/2 after:right-0 after:h-4 after:w-[2.5px] after:-translate-y-1/2 after:rounded-full after:bg-primary";

function SidebarRow({
  item,
  activeId,
  onSelect,
  compacta,
  level = 0,
}: {
  item: SidebarItem;
  activeId: string;
  onSelect: (id: string) => void;
  compacta: boolean;
  level?: number;
}) {
  /*
   * Compactada, a barra não tem largura para duas linhas: a disciplina abre o
   * filho pelo mesmo clique que seleciona a disciplina, e a aba vai pelo
   * detalhe. Sem isto, `aria-expanded` prometeria um painel que não existe.
   */
  const filhos = compacta ? [] : (item.children ?? []);
  const temFilhos = filhos.length > 0;
  const ativo = item.id === activeId;
  const ramoAtivo = contemAtivo(item, activeId);
  const filho = level > 0;
  /*
   * `null` = ninguém decidiu ainda. Sem isto, o grupo nasce fechado mesmo quando a
   * seleção está dentro dele, e a disciplina ativa aparece fechada no primeiro
   * render. O ramo da seleção é o estado inicial; a partir do primeiro clique o
   * grupo obedece à pessoa.
   */
  const [aberto, setAberto] = useState<boolean | null>(null);
  const expandido = aberto ?? (ramoAtivo && temFilhos);

  /*
   * Compactada não há linha filha, então o `id` da seleção (`<disciplina>:<aba>`)
   * nunca casa com o `id` da disciplina, e a barra compacta marcaria **nada** na
   * hora em que a pessoa está dentro de uma disciplina. O ramo passa a valer como
   * seleção: sem filho visível, ele é a única linha que pode dizer onde a pessoa
   * está. Este é o tipo de bug que só aparece na hora de apertar o botão — e
   * "ícone visível mas nenhuma marca de onde estou" é exatamente o que a barra
   * compacta não pode ser.
   */
  const selecionado = compacta && !filho ? ativo || ramoAtivo : ativo;

  /*
   * O rótulo é o que a barra esconde quando compacta, e esconder o rótulo sem
   * devolver nada no lugar dele troca largura por acesso: um ícone sem nome não
   * é navegação, é adivinhação. O tooltip devolve o nome no hover, e o
   * `aria-label` devolve no leitor de tela — os dois, porque um não cobre o
   * outro. `SPEC-D-006` D63 é quem autoriza esta mitigação ("rótulo recolhível").
   */
  const botao = (
    <button
      type="button"
      disabled={item.disabled}
      aria-current={selecionado ? "page" : undefined}
      aria-expanded={temFilhos ? expandido : undefined}
      aria-label={compacta ? item.label : undefined}
      onClick={() => {
        onSelect(item.id);
        if (temFilhos) setAberto(!expandido);
      }}
      style={filho ? { paddingLeft: 26 + level * RECUO } : undefined}
      className={cn(
        "group relative flex h-[34px] w-full items-center gap-2.5 rounded-[8px] text-left motion-state",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60",
        filho ? "pr-2 pl-0 text-[12.5px]" : "px-2.5 text-[13px]",
        item.disabled
          ? "cursor-not-allowed text-muted-foreground opacity-50"
          : cn(
              "cursor-pointer",
              selecionado
                ? cn(
                    "bg-foreground/[0.07] font-medium text-foreground",
                    !compacta && (filho ? "text-primary" : MARCA_SELECAO),
                    /*
                     * A marca de seleção da barra compacta fica à direita, não à
                     * esquerda. À esquerda ela encostaria na borda de 68px e
                     * pareceria parte da moldura; à direita ela fica dentro do
                     * botão, junto do ícone, que é o que ela marca.
                     */
                    compacta && MARCA_SELECAO_DIREITA,
                  )
                : ramoAtivo && !compacta
                  ? "bg-foreground/[0.035] font-medium text-foreground"
                  : "text-muted-foreground hover:bg-foreground/[0.045] hover:text-foreground",
            ),
      )}
    >
      <span
        className={cn(
          "relative grid size-4 shrink-0 place-items-center transition-opacity [&_svg]:size-[15px]",
          selecionado ? "opacity-100" : "opacity-80 group-hover:opacity-100",
          /*
           * O ícone é o que **fica** na barra compacta, então é ele que se move.
           * Recolhendo ele desliza 4px do recuo do layout largo para o centro da
           * coluna de 44px, e cresce um passo; expandindo volta.
           *
           * O movimento é `translate`, e não realinhamento. A linha mantém
           * `px-2.5` e o mesmo alinhamento nos dois estados **de propósito**:
           * realinhar por `justify-content` ou por `padding` moveria o ícone por
           * **layout**, num quadro e sem interpolação, e a transição não teria o
           * que interpolar — o salto viria antes do primeiro quadro.
           *
           * Só o ícone se move. A linha não, senão o pílula de hover e a marca de
           * seleção andariam 4px junto e o item pareceria ganhar foco.
           *
           * 4px é a diferença entre o recuo (10px) e o centro (14px): medido,
           * não chutado — a coluna útil é `68 − 12` de `px-3`, e o ícone 16px.
           *
           * `motion-rail-icon` e não `motion-rail` porque este elemento já tinha
           * `transition-opacity`; duas utilities disputando o atalho `transition`
           * fariam o hover depender da ordem de geração do Tailwind. Ver
           * `styles.css` e DM4 — `width` continua sendo corte.
           *
           * O ícone sem glifo é um ponto de 2px, não um desenho: deslocá-lo seria
           * visível e o `scale` não (2px → 2.1px). Fica parado, que é o certo —
           * ele já é um ponto, e um ponto que anda na barra lê como conteúdo.
           */
          compacta && item.icon && "motion-rail-icon translate-x-1 scale-105",
        )}
      >
        {item.icon}
        {/*
         * O badge some quando o número não cabe, mas o **fato** de haver badge
         * continua sendo informação: sem isto, "Entregas & Provas" ficaria
         * visualmente igual às entradas sem contagem, e o ícone não diz o
         * contrário. Vira um ponto, não o número — 68px não comporta o número.
         */}
        {compacta && item.badge && (
          <span
            aria-hidden="true"
            className={cn(
              "motion-rail-in absolute -top-0.5 -right-1 size-1.5 rounded-full",
              selecionado ? "bg-primary" : "bg-muted-foreground/70",
            )}
          />
        )}
      </span>
      {!compacta && (
        <>
          <span className="motion-rail-in min-w-0 flex-1 truncate tracking-[-0.005em]">
            {item.label}
          </span>

          {item.shortcut && (
            <kbd className="rounded-[5px] border border-border/70 bg-background/70 px-1.5 py-px font-sans text-[10px] text-muted-foreground opacity-0 motion-state group-hover:opacity-100 group-focus-within:opacity-100">
              {item.shortcut}
            </kbd>
          )}
          {item.badge && (
            <span
              className={cn(
                "min-w-[19px] rounded-full px-1.5 py-px text-center text-[10.5px] tabular-nums",
                selecionado
                  ? "bg-primary/15 text-primary"
                  : "bg-foreground/[0.07] text-muted-foreground",
              )}
            >
              {item.badge}
            </span>
          )}
          {temFilhos && (
            <ChevronRight
              className={cn(
                "size-[14px] shrink-0 opacity-35 motion-spatial group-hover:opacity-70",
                expandido && "rotate-90 opacity-70",
              )}
            />
          )}
        </>
      )}
    </button>
  );

  return (
    <div className="flex flex-col">
      {compacta ? (
        /*
         * O tooltip é o rótulo devolvido. `delayDuration` fica no
         * `TooltipProvider` da raiz (250ms), o mesmo contrato dos botões do topo:
         * a barra não inventa tempo de espera próprio.
         */
        <Tooltip>
          <TooltipTrigger asChild>{botao}</TooltipTrigger>
          <TooltipContent side="right">{item.label}</TooltipContent>
        </Tooltip>
      ) : (
        botao
      )}

      {temFilhos && (
        /*
         * A animação de abrir/fechar é `grid-template-rows`, de `0fr` para `1fr`:
         * `height` e `max-height` animariam para um número que não é a altura do
         * conteúdo, e `translate` esconderia o grupo sem revelar os filhos.
         *
         * `inert` no conteúdo recolhido não é decoração: com `overflow: hidden` os
         * filhos continuam focalizáveis pelo teclado, e o `grid-rows-[0fr]` só os
         * tira da vista. Enter dentro de um grupo fechado moveria a seleção sem
         * mostrar para onde.
         */
        <div
          className={cn(
            "grid motion-disclosure",
            expandido ? "grid-rows-[1fr] opacity-100" : "grid-rows-[0fr] opacity-0",
          )}
        >
          <div inert={!expandido} className="relative flex min-h-0 flex-col overflow-hidden">
            {/* A guia fica alinhada ao centro do ícone do pai, e não à borda do
                container: é ela que diz que os filhos pertencem a esta linha. */}
            <span
              aria-hidden="true"
              className="absolute inset-y-0 border-l border-foreground/10"
              style={{ left: 18 + level * RECUO }}
            />
            {filhos.map((filho) => (
              <SidebarRow
                key={filho.id}
                item={filho}
                activeId={activeId}
                onSelect={onSelect}
                compacta={compacta}
                level={level + 1}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export function SidebarNav({
  estado,
  workspace,
  groups,
  footerItems = [],
  footer,
  footerCompacto,
  activeId,
  onSelect,
}: SidebarNavProps) {
  /*
   * "Fechada" é o drawer do mobile e corta tudo; "compacta" é 68px de ícone e
   * não corta nada. A distinção mora **aqui**, em `overflow-hidden`, e não em
   * `hidden` no conteúdo: os ícones continuam no DOM nos dois estados, e é por
   * isso que o botão do topo nunca deixa a pessoa sem navegação.
   */
  const fechada = estado === "fechada";
  const compacta = estado === "compacta";

  return (
    /*
     * A barra recolhe **por corte** no tamanho e **por transform** no conteúdo.
     *
     * Antes isto era `transition-[width] duration-200`: 200ms × ~12 quadros de
     * reflow do `<main>` inteiro, que na visão Calendário contém a grade de 7
     * colunas. Animar `width` é a transição de layout mais cara que existe
     * (DM4). O conteúdo interno tem largura fixa, então ele desliza com
     * `translate` — que é composto, e o layout é calculado uma vez. O corte no
     * tamanho é o que sobra, e é um quadro.
     *
     * A largura vai por `style` porque o valor é o da **fonte** (`estado-sidebar.ts`),
     * e uma classe `w-[${n}px]` montada por template não é vista pelo Tailwind na
     * varredura do fonte: a barra viraria `w-` sem valor, que é 0px nos três
     * estados, sem erro e sem aviso. `larguraDaBarra` é a única que escreve.
     */
    <aside
      className={cn(
        "vibrancy z-40 shrink-0 overflow-hidden border-r border-border/60 bg-sidebar/95 max-md:fixed max-md:inset-y-0 max-md:left-0",
        fechada && "border-r-0",
      )}
      style={{ width: larguraDaBarra(estado) }}
      aria-label="Navegação principal"
    >
      {/*
       * A coluna interna mede a largura **do estado**, e não sempre 264px. São
       * duas coisas diferentes e as duas são necessárias:
       *
       * - No drawer (`fechada`) ela precisa medir 264px para ter um alvo no
       *   `translate`. Medindo 0 não haveria o que deslizar, e a gaveta apareceria
       *   sem animação.
       * - Compactada ela precisa medir 68px, porque o conteúdo é recortado logo
       *   abaixo do `px-3`, e uma coluna de 264px recortada em 68px mostraria
       *   metade do nada.
       *
       * E o motivo de o conteúdo não ser **recentrado** ao compactar é o que
       * decide se a animação existe: recentrar é `justify-content`, que é
       * layout. Layout é calculado uma vez e pintado no quadro seguinte, sem
       * valor intermediário — o item iria de 10px a 14px de distância da borda
       * num salto, e nenhuma transição sobre ele apareceria. O deslocamento de
       * 4px é então um `translate` explícito, com alvo medido, e é ele que
       * interpola. O corte da largura, esse sim, é seco — DM4.
       *
       * O corte em si é sempre o mesmo mecanismo: o `overflow-hidden` do `<aside>`
       * recorta a coluna. Por isso a largura do `<aside>` vai por `style` e não
       * por classe — ver a nota em `estado-sidebar.ts`.
       */}
      <div
        className={cn(
          "flex h-full flex-col px-3 pb-3",
          compacta ? "w-[68px]" : "w-[264px]",
          /*
           * A gaveta (`fechada`) anima em `motion-drawer`, e o `motion-reduce:`
           * saiu daqui porque a utility já trata `reduce` no `styles.css` —
           * mesma transição, mesma curva, mesma duração, num lugar só (DM8).
           */
          fechada && "motion-drawer -translate-x-full",
        )}
      >
        {/*
         * No workspace compacto sobra o emblema: ele é a única pista de *onde*
         * a pessoa está, e um emblema de 36px sobrevive em 68px. O nome e o
         * subtítulo saem, e saem juntos — meio nome sem subtítulo é pior que
         * nenhum.
         *
         * O emblema também não centraliza: recebe um `translate` de −2px, o
         * mesmo truque da linha. Aqui o alvo é o **centro** da coluna de 44px
         * (44 − 36 = 4px), e o recuo dele é `px-1.5` = 6px — daí 2px para trás.
         * Recentralizar por `justify-content` moveria o elemento por layout e o
         * movimento viraria corte.
         */}
        <div
          className={cn(
            "mt-3 mb-1 flex shrink-0 items-center gap-3 px-1.5 py-1.5",
            compacta && "motion-rail -translate-x-0.5",
          )}
        >
          <span className="shadow-float grid size-9 shrink-0 place-items-center rounded-[10px] bg-primary text-primary-foreground [&_svg]:size-[18px]">
            {workspace.icon}
          </span>
          {!compacta && (
            <span className="min-w-0 flex-1">
              <span className="motion-rail-in block truncate text-[13.5px] font-semibold leading-tight tracking-[-0.01em]">
                {workspace.label}
              </span>
              {workspace.sublabel && (
                <span className="motion-rail-in mt-0.5 block truncate text-[11px] leading-tight text-muted-foreground">
                  {workspace.sublabel}
                </span>
              )}
            </span>
          )}
        </div>

        <nav
          aria-label="Principal"
          className="scrollbar-slim mt-2 min-h-0 flex-1 space-y-6 overflow-y-auto pb-2"
        >
          {groups.map((group, index) => (
            <div key={index}>
              {group.heading && !compacta && (
                <div className="mb-1.5 flex items-center justify-between px-2.5">
                  <span className="text-[10.5px] font-semibold tracking-[0.09em] text-muted-foreground/70 uppercase">
                    {group.heading}
                  </span>
                  {group.action}
                </div>
              )}
              {/*
               * O título do grupo ("Atividades", "Disciplinas") não cabe em 68px,
               * e perdê-lo sem substituto junta todos os grupos num bloco só. O
               * traço é o que sobrou da informação: onde o grupo termina e o
               * próximo começa. Só no segundo grupo — o primeiro é o topo da lista
               * e um traço lá seria borda.
               */}
              {compacta && group.heading && index > 0 && (
                <div
                  aria-hidden="true"
                  className="motion-rail-in mx-auto mb-2.5 h-px w-5 bg-border/70"
                />
              )}
              <div className={cn("space-y-0.5", compacta && "space-y-1.5")}>
                {group.items.map((item) => (
                  <SidebarRow
                    key={item.id}
                    item={item}
                    activeId={activeId}
                    onSelect={onSelect}
                    compacta={compacta}
                  />
                ))}
              </div>
            </div>
          ))}
        </nav>

        {(footerItems.length > 0 || footer) && (
          <div className="mt-2 shrink-0 space-y-0.5 border-t border-border/60 pt-2.5">
            {footerItems.map((item) => (
              <SidebarRow
                key={item.id}
                item={item}
                activeId={activeId}
                onSelect={onSelect}
                compacta={compacta}
              />
            ))}
            {!compacta ? footer : footerCompacto}
          </div>
        )}
      </div>
    </aside>
  );
}
