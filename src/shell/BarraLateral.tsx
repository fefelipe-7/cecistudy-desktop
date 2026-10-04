import type { ReactNode } from "react";
import { NavLink } from "react-router";
import { BriefcaseBusiness, ChevronsUpDown, CircleDashed, GraduationCap } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Separator } from "@/components/ui/separator";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

import { ICONES_POR_MODULO, ICONES_POR_UTILIDADE } from "./icones.ts";
import { gruposDaSidebar, UTILIDADES, type ModuleEntry, type UtilityEntry } from "./modules.ts";

/**
 * A barra lateral do Workspace Acadêmico.
 *
 * `SPEC-D-006` `D63` desenha a sidebar; o que este arquivo garante é o que
 * `SPEC-D-013` `D83` e `D84` pedem:
 *
 * - **a lista vem do registro** — `gruposDaSidebar()` para os módulos e
 *   `UTILIDADES` para o rodapé — e de nenhum outro lugar. Os nove nomes são `[D]`
 *   de §2 linha 58, e `scripts/check-modules.mjs` reprova um rótulo declarado fora
 *   de `modules.ts`;
 * - **a ordem dentro das seções é a de §2 linha 60**, que é `[P]`, lida do
 *   registro como dado e não escolhida aqui;
 * - **Configurações / Perfil não aparece aqui** — o registro diz `posicao: "fora"`
 *   porque §2 linha 60 lista três entradas no rodapé e não menciona a quarta.
 *
 * ## As três decisões de desenho, e a medição de cada uma
 *
 * **A superfície é `--sidebar`, e não `--card/40`.** `--card` e `--background` são
 * o mesmo valor oklch (`styles.css:117` e `:132`), então `bg-card/40` sobre
 * `bg-background` dá **1.00:1** — a barra não era "levemente distinta"
 * (`DESIGN.md:17`), era idêntica ao conteúdo. `--sidebar` existe, está registrado
 * como `--color-sidebar` e não era importado em lugar nenhum do app. Medido:
 * 1.06:1 contra o conteúdo, que é distinção de superfície e não de texto.
 *
 * **O repouso é `text-foreground`, e não `text-muted-foreground`.** Medido sobre
 * `--sidebar`: `foreground` **4.62:1** passa AA; `muted-foreground` **2.85:1** falha,
 * e 13px não é "large text". `muted-foreground` é token de eixo e de apoio — ele
 * não pode carregar o nome dos nove módulos. A hierarquia entre repouso e ativo
 * vem de fundo e peso, não de cor.
 *
 * **A largura não anima.** `docs/performance/02-transicoes-animacoes.md:279` (DM4)
 * proíbe transicionar `width`, e o item nº1 do inventário de performance é esta
 * barra, 🔴 "Proibido por DM4": `width` em flex row reflowa o `<main>` inteiro, que
 * na visão Calendário contém a grade de 7 colunas com dois `sticky` e
 * `backdrop-blur`. Recolher é corte seco — a saída 2 do DM4 (:286), que a própria
 * spec chama de preferível a doze quadros de reflow. O que atravessa 150ms é o
 * **rótulo**, em opacidade, que é composite e é o que DM4 permite.
 *
 * ## Onde a identidade da usuária não aparece
 *
 * O rodapé **não** tem nome, e-mail nem avatar, e a ausência é deliberada. A
 * tabela `profile` existe em `src-tauri/migrations/0003_usuario.sql`, mas nenhum
 * comando Rust a lê — `contracts/usuario.json` declara só `get_setting` e
 * `set_setting`, e `npm run gate:contracts` exige que manifest, `invoke_handler` e
 * wrapper TypeScript casem. Nome e e-mail escritos no componente seriam inventados:
 * certos na tela e inexistentes no banco.
 */
export function BarraLateral({ recolhida }: { recolhida: boolean }) {
  return (
    <TooltipProvider delayDuration={200}>
      <nav
        id="barra-lateral"
        aria-label="Workspace Acadêmico"
        className={cn(
          "flex shrink-0 flex-col overflow-hidden border-r border-border/60 bg-sidebar",
          recolhida ? "w-(--rail-recolhida)" : "w-(--rail-expandida)",
        )}
      >
        <SeletorDeWorkspace recolhida={recolhida} />

        <div className="scrollbar-slim min-h-0 flex-1 overflow-y-auto px-2 py-3">
          {gruposDaSidebar().map((secao, indice) => (
            <section key={secao.id} className={cn(indice > 0 && "mt-5")}>
              {/*
                O cabeçalho da seção é `sr-only` no estado recolhido em vez de
                removido: o `h2` continua anunciando a agrupagem para leitor de
                tela, e o que some é só a linha visual. `aria-hidden` resolveria o
                visual e perderia o agrupamento.

                A cor é `text-foreground`, e não `text-muted-foreground` como um
                cabeçalho parece pedir: medido sobre `--sidebar`, `muted-foreground`
                dá 2.85:1 e reprova AA, e 11px está longe de "large text". A
                hierarquia vem do **tamanho** — 11px uppercase com tracking contra
                13px regular — e não de baixar o contraste de um texto que a usuária
                precisa ler.
              */}
              <h2
                className={cn(
                  "px-2 pb-1 text-[11px] font-medium uppercase tracking-[0.06em]",
                  recolhida && "sr-only",
                )}
              >
                {secao.nome}
              </h2>
              <ul className="flex flex-col gap-0.5">
                {secao.modulos.map((modulo) => (
                  <li key={modulo.id}>
                    <ItemDeModulo modulo={modulo} recolhida={recolhida} />
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>

        <div className="px-2 pb-1">
          <Separator />
        </div>

        {/* Rodapé: §2 linha 60 põe Ceci, Busca e Notificações. */}
        <ul className="flex flex-col gap-0.5 px-2 py-2">
          {UTILIDADES.filter((u) => u.posicao === "rodape").map((utilidade) => (
            <li key={utilidade.id}>
              <ItemDeUtilidade utilidade={utilidade} recolhida={recolhida} />
            </li>
          ))}
        </ul>
      </nav>
    </TooltipProvider>
  );
}

/**
 * O seletor de workspace do topo.
 *
 * §2 linha 60 põe um seletor aqui, e `SPEC-D-006` `D63` exige que ele exista desde
 * o primeiro dia **com um workspace funcional** — e o Profissional está fora de
 * escopo (§2 linha 47). Ele aparece desabilitado e marcado como "em construção",
 * que é o caso de §7: "seletor que oferece o que não existe é pior que a ausência
 * dele".
 *
 * A altura do bloco é `h-11` (44px), e não 48px: 44px é a altura da barra de topo e
 * é o que `scroll-margin-top` do container de rolagem da grade reserva
 * (`styles.css:314`). Com 48px a borda de baixo da barra caía 4px abaixo da borda
 * de baixo do topo, e as duas linhas horizontais não se conversavam.
 */
function SeletorDeWorkspace({ recolhida }: { recolhida: boolean }) {
  return (
    <div className="flex h-11 shrink-0 items-center px-2">
      <DropdownMenu modal={false}>
        <DropdownMenuTrigger asChild>
          <Button
            variant="ghost"
            aria-label="Selecionar workspace"
            className={cn("h-8 w-full justify-start gap-2 px-2", recolhida && "w-8 px-0")}
          >
            <GraduationCap className="shrink-0" aria-hidden />
            <Rotulo recolhida={recolhida}>Acadêmico</Rotulo>
            {recolhida ? null : (
              /*
                Sem `/70`: o chevron é elemento de interface e WCAG 1.4.11 pede
                3:1, e `muted-foreground/70` sobre `--sidebar` dá bem menos que isso.
                O `/70` que estava aqui era herança de ícone decorativo.
              */
              <ChevronsUpDown className="ml-auto text-muted-foreground" aria-hidden />
            )}
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="w-(--rail-expandida)">
          <DropdownMenuItem className="gap-2 font-medium">
            <GraduationCap aria-hidden />
            Acadêmico
          </DropdownMenuItem>
          <DropdownMenuItem disabled className="gap-2">
            <BriefcaseBusiness aria-hidden />
            Profissional
            <span className="ml-auto text-xs text-muted-foreground">em construção</span>
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}

/**
 * Um módulo da sidebar.
 *
 * `CircleDashed` marca **decisão em aberto** (`[A]` da spec referencial). É forma,
 * não cor: um ponto colorido seria lido como decoração — e `PRODUCT.md:27` diz que
 * cor comunica estado, "nunca decoração" — e uma cor nova para "bloqueado"
 * competiria com o `--primary` que já significa "onde você está" na barra.
 */
function ItemDeModulo({ modulo, recolhida }: { modulo: ModuleEntry; recolhida: boolean }) {
  const Icone = ICONES_POR_MODULO[modulo.id];
  const bloqueado = Boolean(modulo.bloqueadoPor);

  return (
    <ItemDeNavegacao
      rota={modulo.rota}
      recolhida={recolhida}
      descricao={bloqueado ? `${modulo.rotulo} · decisão em aberto` : modulo.rotulo}
    >
      <Icone aria-hidden />
      <Rotulo recolhida={recolhida}>{modulo.rotulo}</Rotulo>
      {bloqueado ? (
        <>
          <CircleDashed
            className={cn("ml-auto shrink-0 text-muted-foreground", recolhida && "sr-only")}
            aria-hidden
          />
          <span className="sr-only"> · decisão em aberto</span>
        </>
      ) : null}
    </ItemDeNavegacao>
  );
}

function ItemDeUtilidade({
  utilidade,
  recolhida,
}: {
  utilidade: UtilityEntry;
  recolhida: boolean;
}) {
  const Icone = ICONES_POR_UTILIDADE[utilidade.id];
  return (
    <ItemDeNavegacao rota={utilidade.rota} recolhida={recolhida} descricao={utilidade.rotulo}>
      <Icone aria-hidden />
      <Rotulo recolhida={recolhida}>{utilidade.rotulo}</Rotulo>
    </ItemDeNavegacao>
  );
}

/**
 * O item de navegação, com a etiqueta que o nomeia quando a barra está recolhida.
 *
 * A `Tooltip` só é montada no estado recolhido: etiqueta sobre um rótulo que já
 * está visível é ruído, e é a condição que o primitivo do shadcn usa. Ela substitui
 * o `title` nativo que estava aqui antes, que **não aparece no foco de teclado** —
 * e `PRODUCT.md:28` exige que ações recorrentes funcionem bem por teclado. A
 * primitive já existia no set curado sem ser importada em lugar nenhum do app.
 */
function ItemDeNavegacao({
  rota,
  recolhida,
  descricao,
  children,
}: {
  rota: string;
  recolhida: boolean;
  descricao: string;
  children: ReactNode;
}) {
  const item = (
    <NavLink to={rota} className={({ isActive }) => itemDeNavegacao(isActive, recolhida)}>
      {({ isActive }) => (
        <>
          {/*
            A barra de 2px do item selecionado é a técnica do Activity Bar do VS
            Code: geométrica, não cromática. Ela sobrevive a daltonismo, a modo de
            alto contraste e a impressão em P&B, e não compete com nenhuma cor de
            estado. Fica dentro do item (`left-0`), então a borda externa da barra
            não é duplicada por ela.
          */}
          {isActive ? (
            <span className="absolute inset-y-1 left-0 w-0.5 rounded-full bg-primary" aria-hidden />
          ) : null}
          {children}
        </>
      )}
    </NavLink>
  );

  if (!recolhida) return item;

  return (
    <Tooltip>
      <TooltipTrigger asChild>{item}</TooltipTrigger>
      <TooltipContent side="right">{descricao}</TooltipContent>
    </Tooltip>
  );
}

/**
 * O texto do item.
 *
 * `w-0` + `overflow-hidden` quando recolhida, e não `hidden`: some com a largura
 * para o ícone centralizar no trilho de 48px, e **mantém o texto na árvore de
 * acessibilidade** — quem navega por leitor de tela continua ouvindo "Faculdade"
 * com a barra recolhida.
 *
 * E é o `w-0` que resolve o desalinhamento: o `gap-2` do item continua aplicando
 * entre o ícone e este span de largura zero, então sem ele o ícone de 16px ficaria
 * 4px à esquerda do centro do trilho de 48px.
 */
function Rotulo({ recolhida, children }: { recolhida: boolean; children: string }) {
  return (
    <span
      className={cn(
        "motion-rail-label truncate",
        recolhida ? "w-0 flex-none overflow-hidden opacity-0" : "min-w-0 flex-1",
      )}
    >
      {children}
    </span>
  );
}

/**
 * A aparência do item, nos dois estados e nos dois formatos da barra.
 *
 * O `hover` é `accent/40` e o selecionado é `secondary`, e a diferença de
 * intensidade é deliberada: hover e selecionado compartilhando token faz cada item
 * parecer meio selecionado enquanto o ponteiro se move, que é o defeito mais
 * comum de sidebar que existe — e aqui ele seria pior, porque `secondary` e
 * `accent` são **o mesmo valor oklch** (`styles.css:121` e `:131`).
 *
 * O `font-medium` no selecionado é o terceiro canal, e é o que faz o estado
 * sobreviver a quem não distingue a cor: ativo só com fundo falha WCAG 1.4.1 e
 * 1.3.3. O anel de foco é o mesmo `ring-1 ring-ring` de `button`, `checkbox`,
 * `input`, `select` e `textarea` — a sidebar era o único alvo de clique do app
 * sem estado de foco.
 */
function itemDeNavegacao(isActive: boolean, recolhida: boolean) {
  return cn(
    "relative flex h-8 items-center gap-2 rounded-md px-2 text-[13px] motion-state",
    "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
    recolhida && "justify-center px-0",
    isActive ? "bg-secondary font-medium text-foreground" : "text-foreground hover:bg-accent/40",
  );
}
