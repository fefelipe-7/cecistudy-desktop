import {
  Bell,
  BookOpen,
  Briefcase,
  Building2,
  CalendarDays,
  ChevronDown,
  Construction,
  Dumbbell,
  FileText,
  GraduationCap,
  LayoutDashboard,
  Library,
  Lightbulb,
  PanelLeftClose,
  PanelLeftOpen,
  Search,
  Sparkles,
} from "lucide-react";
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { CecinhoTip } from "@/components/CecinhoTip";
import { HeroCard } from "@/components/HeroCard";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { useIsMobile } from "@/hooks/use-mobile";
import { VAZIO } from "@/lib/copy";
import { pickTip } from "@/lib/tips";
import { cn } from "@/lib/utils";
import { SidebarNav, type SidebarGroup, type SidebarItem } from "@/shell/SidebarNav";
import { alternarSidebar, estadoAoMudarViewport, type EstadoSidebar } from "@/shell/estado-sidebar";
import {
  MODULOS,
  UTILITARIOS,
  WORKSPACES,
  moduloPorId,
  type ModuleId,
  type Modulo,
  type UtilitarioId,
  type Workspace,
} from "@/shell/navigation";
import { CalendarScreen } from "@/features/calendar/ui/CalendarScreen.tsx";
import { FaculdadeScreen } from "@/features/faculdade/ui/FaculdadeScreen.tsx";
import { EstudosScreen } from "@/features/estudos/ui/EstudosScreen.tsx";
import { TccScreen } from "@/features/tcc/ui/TccScreen.tsx";

/*
 * A shell lê a navegação de `shell/navigation.ts` e não monta lista própria. Um
 * único lugar declara quantos módulos existem; a sidebar, o título, a busca e o
 * seletor leem de lá. É o que `SPEC-D-006` D62 chama de tabela declarativa, e é a
 * razão pela qual a sidebar e o roteamento não podem mais discordar.
 *
 * A exceção é o ícone, que é apresentação: ele mora aqui e não na tabela, porque
 * `navigation.ts` é `.ts` puro, testável sem DOM, e um `ReactNode` quebraria isso.
 */
const ICONE: Record<ModuleId, ReactNode> = {
  home: <LayoutDashboard />,
  calendario: <CalendarDays />,
  faculdade: <GraduationCap />,
  estudos: <BookOpen />,
  biblioteca: <Library />,
  conhecimento: <Lightbulb />,
  tcc: <FileText />,
  estagio: <Briefcase />,
  "sala-de-treino": <Dumbbell />,
};

const ICONE_UTILITARIO: Record<UtilitarioId, ReactNode> = {
  cecinho: <Sparkles />,
  busca: <Search />,
  notificacoes: <Bell />,
};

const ROTULO_UTILITARIO: Record<UtilitarioId, string> = {
  cecinho: "cecinho",
  busca: "Busca",
  notificacoes: "Notificações",
};

function IconButton({
  label,
  children,
  onClick,
  disabled,
  className,
}: {
  label: string;
  children: ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  className?: string;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          aria-label={label}
          variant="ghost"
          size="icon"
          onClick={onClick}
          disabled={disabled}
          className={cn(
            "size-8 rounded-[7px] text-muted-foreground hover:bg-accent/60 hover:text-foreground",
            className,
          )}
        >
          {children}
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}

export default function StudyApp() {
  /*
   * Os **três** estados de navegação de D62, e nenhum outro: módulo ativo
   * (`modulo`), painel lateral (`sidebar`) e camada de sobreposição (`searchOpen`).
   * Antes eram seis `useState` misturados com o dashboard (`App.tsx:137-142`); o
   * workspace entra aqui como quarto estado porque o seletor é decisão de D63 e não
   * é conteúdo de módulo.
   */
  const [workspaceId, setWorkspaceId] = useState<Workspace["id"]>("academico");
  const [modulo, setModulo] = useState<ModuleId>("home");
  const [sidebar, setSidebar] = useState<EstadoSidebar>("aberta");
  const [searchOpen, setSearchOpen] = useState(false);
  const [query, setQuery] = useState("");
  const isMobile = useIsMobile();

  const workspace = WORKSPACES.find((item) => item.id === workspaceId) ?? WORKSPACES[0]!;
  const academico = workspaceId === "academico";
  const ativo = moduloPorId(modulo);

  useEffect(() => {
    setSidebar((atual) => estadoAoMudarViewport(atual, isMobile));
  }, [isMobile]);

  useEffect(() => {
    // O listener vive sempre: condicioná-lo a `searchOpen` faria o atalho só
    // existir com a busca já aberta — e `⌘K` nunca abriria nada.
    const handler = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setSearchOpen(true);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const grupos = useMemo<SidebarGroup[]>(
    () =>
      academico
        ? [
            {
              items: MODULOS.map((item) => ({
                id: item.id,
                label: item.titulo,
                icon: ICONE[item.id],
              })),
            },
          ]
        : [],
    [academico],
  );

  const rodapeItens = useMemo<SidebarItem[]>(
    () =>
      UTILITARIOS.map((id) => ({
        id: `util:${id}`,
        label: ROTULO_UTILITARIO[id],
        icon: ICONE_UTILITARIO[id],
        disabled: id !== "busca",
        ...(id === "busca" ? { shortcut: "⌘K" } : {}),
      })),
    [],
  );

  const activeId = searchOpen ? "util:busca" : modulo;

  const selecionar = (id: string) => {
    if (isMobile) setSidebar("fechada");
    if (id.startsWith("util:")) {
      if (id === "util:busca") setSearchOpen(true);
      return;
    }
    setModulo(id as ModuleId);
  };

  const selecionarWorkspace = (id: Workspace["id"]) => {
    setWorkspaceId(id);
    if (id === "academico") setModulo("home");
  };

  const resultados = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return MODULOS;
    return MODULOS.filter((item) => `${item.titulo} ${item.descricao}`.toLowerCase().includes(q));
  }, [query]);

  return (
    <TooltipProvider delayDuration={250}>
      <div className="flex h-dvh w-full min-w-0 overflow-hidden bg-background">
        {isMobile && sidebar === "aberta" && (
          <button
            aria-label="Fechar barra lateral"
            className="scrim fixed inset-0 z-30 cursor-default bg-foreground/20"
            onClick={() => setSidebar("fechada")}
          />
        )}
        <SidebarNav
          estado={sidebar}
          workspace={{
            label: workspace.titulo,
            sublabel: workspace.subtitulo,
            icon: workspaceId === "academico" ? <GraduationCap /> : <Building2 />,
          }}
          groups={grupos}
          footerItems={rodapeItens}
          footer={
            <div className="mt-1 flex items-center gap-2.5 rounded-[8px] px-2 py-1.5 transition-colors hover:bg-foreground/[0.045]">
              <span className="grid size-7 shrink-0 place-items-center rounded-full bg-primary/12 text-[11px] font-semibold text-primary">
                FM
              </span>
              <span className="min-w-0">
                <span className="block truncate text-[12.5px] font-medium leading-tight tracking-[-0.005em]">
                  Felipe Martins
                </span>
                <span className="mt-0.5 block truncate text-[11px] leading-tight text-muted-foreground">
                  Ciência da Computação
                </span>
              </span>
            </div>
          }
          footerCompacto={
            <div
              title="Felipe Martins · Ciência da Computação"
              className="mt-1 flex justify-center rounded-[8px] px-2 py-1.5 transition-colors hover:bg-foreground/[0.045]"
            >
              <span className="grid size-7 shrink-0 place-items-center rounded-full bg-primary/12 text-[11px] font-semibold text-primary">
                FM
              </span>
            </div>
          }
          activeId={activeId}
          onSelect={selecionar}
        />

        <main className="flex min-w-0 flex-1 flex-col bg-background">
          <header className="vibrancy sticky top-0 z-20 grid h-[52px] shrink-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 border-b border-border/70 bg-card/70 px-3 sm:px-4">
            <div className="flex items-center gap-1.5">
              {/*
               * O rótulo do botão diz para onde ele vai, não de onde veio: no
               * desktop, "compactar"/"expandir"; no mobile, o alvo é a gaveta.
               */}
              <IconButton
                label={
                  isMobile
                    ? sidebar === "aberta"
                      ? "Fechar barra lateral"
                      : "Abrir barra lateral"
                    : sidebar === "compacta"
                      ? "Expandir barra lateral"
                      : "Compactar barra lateral"
                }
                onClick={() => setSidebar((atual) => alternarSidebar(atual, isMobile))}
              >
                <span key={sidebar} className="motion-rail-in grid place-items-center">
                  {sidebar === "aberta" ? <PanelLeftClose /> : <PanelLeftOpen />}
                </span>
              </IconButton>
              <span className="hidden h-5 w-px bg-border/80 sm:block" />
              {/*
               * O seletor de workspace (§2, [D]). O Profissional existe desde o
               * primeiro dia e é selecionável, mas o estado "em construção" é
               * explícito — um clique que leva a lugar nenhum é o que §7 proíbe.
               */}
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button
                    variant="ghost"
                    className="h-8 gap-1.5 rounded-[7px] px-2 text-[12.5px] font-medium"
                  >
                    {workspace.titulo}
                    <ChevronDown className="size-3.5 text-muted-foreground" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="start" className="w-60">
                  <DropdownMenuLabel>Workspace</DropdownMenuLabel>
                  <DropdownMenuSeparator />
                  {WORKSPACES.map((item) => (
                    <DropdownMenuItem
                      key={item.id}
                      onSelect={() => selecionarWorkspace(item.id)}
                      className="flex-col items-start gap-0.5"
                    >
                      <span className="flex w-full items-center justify-between gap-2">
                        <span className="text-[12.5px] font-medium">{item.titulo}</span>
                        {item.estado === "em-construcao" && (
                          <span className="rounded-full bg-muted px-1.5 py-px text-[10px] text-muted-foreground">
                            em construção
                          </span>
                        )}
                      </span>
                      <span className="text-[11px] text-muted-foreground">{item.subtitulo}</span>
                    </DropdownMenuItem>
                  ))}
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
            <div className="flex min-w-0 items-baseline gap-2">
              <h1 className="truncate text-[13px] font-semibold">
                {academico ? ativo.titulo : workspace.titulo}
              </h1>
            </div>
            <div className="flex shrink-0 items-center gap-1.5">
              <IconButton label="Notificações" disabled>
                <Bell />
              </IconButton>
            </div>
          </header>

          <div className="scrollbar-slim min-h-0 flex-1 overflow-auto">
            {academico ? (
              ativo.id === "calendario" ? (
                <CalendarScreen />
              ) : ativo.id === "faculdade" ? (
                <FaculdadeScreen />
              ) : ativo.id === "estudos" ? (
                <EstudosScreen />
              ) : ativo.id === "tcc" ? (
                <TccScreen />
              ) : (
                <ModuloEmConstrucao
                  modulo={ativo}
                  onIrParaCalendario={() => setModulo("calendario")}
                />
              )
            ) : (
              <WorkspaceEmConstrucao workspace={workspace} />
            )}
          </div>
        </main>
      </div>

      {searchOpen && (
        // Fechar limpa a consulta: reabrir com o texto anterior faria a busca
        // parecer já filtrada quando ela recomeçou do zero.
        <Dialog
          open
          onOpenChange={(open) => {
            setSearchOpen(open);
            if (!open) setQuery("");
          }}
        >
          <DialogContent
            showCloseButton={false}
            className="vibrancy top-[14vh] max-w-xl translate-y-0 gap-0 overflow-hidden rounded-[14px] border-foreground/10 bg-popover/90 p-0 shadow-window"
          >
            <DialogTitle className="sr-only">Busca rápida</DialogTitle>
            <div className="flex items-center gap-3 border-b border-border/70 px-4">
              <Search className="size-4 text-muted-foreground" />
              <input
                autoFocus
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Buscar módulos…"
                aria-label="Buscar módulos"
                className="h-12 min-w-0 flex-1 bg-transparent text-[13px] outline-none placeholder:text-muted-foreground"
              />
              <kbd className="rounded-[5px] border border-border/70 bg-muted px-1.5 py-px text-[10px] text-muted-foreground">
                esc
              </kbd>
            </div>
            <div className="max-h-80 overflow-auto p-1.5">
              <p className="px-2.5 py-1.5 text-[10px] font-semibold uppercase tracking-[0.08em] text-muted-foreground/80">
                Módulos
              </p>
              {resultados.map((item) => (
                <button
                  key={item.id}
                  onClick={() => {
                    setWorkspaceId("academico");
                    setModulo(item.id);
                    setSearchOpen(false);
                    setQuery("");
                  }}
                  className="flex w-full cursor-pointer items-center gap-3 rounded-[8px] px-2.5 py-2 text-left hover:bg-primary hover:text-primary-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                  <span className="grid size-5 shrink-0 place-items-center [&_svg]:size-4 opacity-70">
                    {ICONE[item.id]}
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-[13px] font-medium">{item.titulo}</span>
                    <span className="block truncate text-[11px] opacity-70">{item.descricao}</span>
                  </span>
                </button>
              ))}
              {resultados.length === 0 && (
                <p className="px-2.5 py-6 text-center text-[12px] text-muted-foreground">
                  {VAZIO.nadaEncontrado(query)}
                </p>
              )}
            </div>
          </DialogContent>
        </Dialog>
      )}
    </TooltipProvider>
  );
}

function ModuloEmConstrucao({
  modulo,
  onIrParaCalendario,
}: {
  modulo: Modulo;
  onIrParaCalendario: () => void;
}) {
  /*
   * A Home é a tela de destaque do shell (D92: "hero só em telas de destaque") —
   * é onde o cecinho acolhe a usuária quando o app abre. Nos outros módulos, o
   * empty state continua ícone + estado, e todos ganham a dica no fim.
   */
  const destaque = modulo.id === "home";
  return (
    <div className="mx-auto flex max-w-lg flex-col items-center gap-3 px-6 py-16 text-center">
      {destaque ? (
        <HeroCard
          eyebrow="workspace acadêmico"
          title={modulo.titulo}
          summary={modulo.descricao}
          action={{ label: "Ir para o Calendário", onClick: onIrParaCalendario }}
          expression="welcome-wave"
        />
      ) : (
        <>
          <span className="grid size-12 place-items-center rounded-[12px] bg-muted text-muted-foreground [&_svg]:size-5">
            {ICONE[modulo.id]}
          </span>
          <h2 className="text-[18px] font-semibold tracking-[-0.01em]">{modulo.titulo}</h2>
          <p className="text-[12.5px] text-muted-foreground">{modulo.descricao}</p>
        </>
      )}
      <span className="flex items-center gap-1.5 rounded-full bg-muted px-2.5 py-1 text-[10.5px] font-medium text-muted-foreground">
        <Construction className="size-3" />
        em construção
      </span>
      <p className="text-[11px] text-muted-foreground/80">
        A entrada existe por decisão de produto (`SPEC-D-006`); a tela é `SPEC-D-007`..`012`.
      </p>
      {!destaque && (
        <Button variant="outline" size="sm" className="mt-2" onClick={onIrParaCalendario}>
          <CalendarDays className="size-3.5" />
          Ir para o Calendário
        </Button>
      )}
      <CecinhoTip tip={pickTip({})} />
    </div>
  );
}

function WorkspaceEmConstrucao({ workspace }: { workspace: Workspace }) {
  return (
    <div className="mx-auto flex max-w-lg flex-col items-center gap-3 px-6 py-16 text-center">
      <span className="grid size-12 place-items-center rounded-[12px] bg-muted text-muted-foreground [&_svg]:size-5">
        <Building2 />
      </span>
      <span className="flex items-center gap-1.5 rounded-full bg-muted px-2.5 py-1 text-[10.5px] font-medium text-muted-foreground">
        <Construction className="size-3" />
        em construção
      </span>
      <h2 className="text-[18px] font-semibold tracking-[-0.01em]">{workspace.titulo}</h2>
      <p className="text-[12.5px] text-muted-foreground">
        Consultório, Prontuário, Financeiro, Documentos e Marketing Studio. Fora do escopo desta
        rodada (§2 da spec referencial, spec própria).
      </p>
      <CecinhoTip tip={pickTip({})} />
    </div>
  );
}
