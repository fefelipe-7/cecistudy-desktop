import {
  Bell,
  CalendarDays,
  Check,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  Circle,
  Clock3,
  FileText,
  GraduationCap,
  LayoutDashboard,
  List,
  MoreHorizontal,
  PanelLeft,
  Plus,
  Search,
  Settings,
  Sparkles,
  Target,
  Zap,
} from "lucide-react";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactNode,
} from "react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { useIsMobile } from "@/hooks/use-mobile";
import { cn } from "@/lib/utils";
import { CalendarScreen } from "@/features/calendar/ui/CalendarScreen.tsx";

type View = "today" | "calendar" | "kanban" | "list" | "subject";
type Subject = { name: string; short: string; dot: string; soft: string };
type Task = { title: string; subject: string; date: string; tag: string; progress: number };

const defaultSubject: Subject = {
  name: "Cálculo II",
  short: "C2",
  dot: "bg-chart-2",
  soft: "bg-chart-2/12 border-chart-2/25",
};
const subjects: Subject[] = [
  defaultSubject,
  {
    name: "Estruturas de Dados",
    short: "ED",
    dot: "bg-chart-3",
    soft: "bg-chart-3/12 border-chart-3/25",
  },
  {
    name: "Banco de Dados",
    short: "BD",
    dot: "bg-chart-1",
    soft: "bg-chart-1/12 border-chart-1/25",
  },
  {
    name: "Engenharia de Software",
    short: "ES",
    dot: "bg-chart-5",
    soft: "bg-chart-5/14 border-chart-5/30",
  },
];

const tasks: Task[] = [
  {
    title: "Lista 4 — Integrais",
    subject: "Cálculo II",
    date: "Hoje, 23:59",
    tag: "Urgente",
    progress: 75,
  },
  {
    title: "Revisar árvores AVL",
    subject: "Estruturas de Dados",
    date: "Amanhã, 10:00",
    tag: "Prova",
    progress: 40,
  },
  {
    title: "Modelo entidade-relacionamento",
    subject: "Banco de Dados",
    date: "Sex, 18:00",
    tag: "Trabalho",
    progress: 20,
  },
  {
    title: "Ler capítulo 6",
    subject: "Engenharia de Software",
    date: "Seg, 08:00",
    tag: "Leitura",
    progress: 0,
  },
];

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
  const [view, setView] = useState<View>("today");
  const [activeSubject, setActiveSubject] = useState<Subject>(defaultSubject);
  const [sidebar, setSidebar] = useState(true);
  const [searchOpen, setSearchOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [done, setDone] = useState<string[]>([]);
  const isMobile = useIsMobile();

  useEffect(() => {
    if (isMobile) setSidebar(false);
  }, [isMobile]);

  useEffect(() => {
    // O listener vive sempre: se ficasse condicionado a `searchOpen`, o atalho
    // só existiria com a busca já aberta, e Cmd+K nunca abriria nada.
    const handler = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setSearchOpen(true);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const title =
    view === "today"
      ? "Hoje"
      : view === "calendar"
        ? "Grade & Calendário"
        : view === "kanban"
          ? "Entregas & Provas"
          : view === "list"
            ? "Todas as atividades"
            : activeSubject.name;
  const filtered = useMemo(
    () =>
      tasks.filter((task) =>
        `${task.title} ${task.subject}`.toLowerCase().includes(query.toLowerCase()),
      ),
    [query],
  );

  return (
    <TooltipProvider delayDuration={250}>
      <div className="flex h-dvh w-full min-w-0 overflow-hidden bg-background">
        {isMobile && sidebar && (
          <button
            aria-label="Fechar barra lateral"
            className="fixed inset-0 z-30 cursor-default scrim bg-foreground/20"
            onClick={() => setSidebar(false)}
          />
        )}
        {/*
          A barra recolhe **por corte** no tamanho e **por transform** no conteúdo.

          Antes isto era `transition-[width] duration-200`: 200ms × ~12 quadros de
          reflow do `<main>` inteiro, que na visão Calendário contém a grade de 7
          colunas. Animar `width` é a transição de layout mais cara que existe
          (DM4). O conteúdo interno já é de largura fixa (`w-[248px]`), então ele
          desliza com `translate` — que é composto, e o layout é calculado uma
          vez. O corte no tamanho é o que sobra, e é um quadro.
        */}
        <aside
          className={cn(
            "vibrancy z-40 shrink-0 overflow-hidden border-r border-border/70 bg-sidebar/95 max-md:fixed max-md:inset-y-0 max-md:left-0",
            sidebar ? "w-[248px]" : "w-0 border-r-0 lg:w-[68px] lg:border-r",
          )}
        >
          <div
            className={cn(
              "flex h-full w-[248px] flex-col px-3 pb-3 transition-transform duration-200 ease-[cubic-bezier(0,0,0.2,1)] motion-reduce:transition-none",
              !sidebar && "-translate-x-full",
            )}
          >
            <button className="mt-2 flex h-11 items-center gap-2.5 rounded-[9px] px-2 text-left transition-colors hover:bg-accent/50">
              <span className="grid size-7 shrink-0 place-items-center rounded-[7px] bg-primary text-primary-foreground">
                <GraduationCap className="size-4" />
              </span>
              <span className={cn("min-w-0 flex-1", !sidebar && "lg:hidden")}>
                <span className="block truncate text-[13px] font-semibold leading-tight">
                  Campus
                </span>
                <span className="block truncate text-[11px] leading-tight text-muted-foreground">
                  2026.2 · 5º período
                </span>
              </span>
              <ChevronDown
                className={cn("size-3.5 text-muted-foreground", !sidebar && "lg:hidden")}
              />
            </button>

            <button
              onClick={() => setSearchOpen(true)}
              className={cn(
                "mt-3 flex h-8 cursor-pointer items-center gap-2 rounded-[8px] border border-border/70 bg-card/70 px-2 text-[12px] text-muted-foreground shadow-xs transition-colors hover:bg-card focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                !sidebar && "lg:w-9 lg:justify-center lg:px-0",
              )}
            >
              <Search className="size-3.5 shrink-0" />
              <span className={cn("flex-1 text-left", !sidebar && "lg:hidden")}>Busca rápida</span>
              <kbd
                className={cn(
                  "rounded-[5px] border border-border/70 bg-background/60 px-1.5 py-px font-sans text-[10px]",
                  !sidebar && "lg:hidden",
                )}
              >
                ⌘K
              </kbd>
            </button>

            <nav className="mt-5 space-y-0.5" aria-label="Principal">
              <NavItem
                active={view === "today"}
                icon={<LayoutDashboard />}
                label="Hoje"
                onClick={() => {
                  setView("today");
                  if (isMobile) setSidebar(false);
                }}
                collapsed={!sidebar}
              />
              <NavItem
                active={view === "calendar"}
                icon={<CalendarDays />}
                label="Grade & Calendário"
                onClick={() => {
                  setView("calendar");
                  if (isMobile) setSidebar(false);
                }}
                collapsed={!sidebar}
              />
              <NavItem
                active={view === "kanban" || view === "list"}
                icon={<Target />}
                label="Entregas & Provas"
                count={String(tasks.length)}
                onClick={() => {
                  setView("kanban");
                  if (isMobile) setSidebar(false);
                }}
                collapsed={!sidebar}
              />
            </nav>

            <div
              className={cn(
                "mt-6 flex items-center justify-between px-2",
                !sidebar && "lg:justify-center",
              )}
            >
              <span
                className={cn(
                  "text-[10px] font-semibold uppercase tracking-[0.08em] text-muted-foreground/80",
                  !sidebar && "lg:hidden",
                )}
              >
                Disciplinas
              </span>
              <Plus className={cn("size-3.5 text-muted-foreground", !sidebar && "lg:hidden")} />
            </div>
            <div className="mt-1.5 space-y-0.5">
              {subjects.map((subject) => (
                <NavItem
                  key={subject.short}
                  active={view === "subject" && activeSubject.name === subject.name}
                  icon={<span className={cn("size-2 rounded-full", subject.dot)} />}
                  label={subject.name}
                  onClick={() => {
                    setActiveSubject(subject);
                    setView("subject");
                    if (isMobile) setSidebar(false);
                  }}
                  collapsed={!sidebar}
                />
              ))}
            </div>

            <div className="mt-auto space-y-1 border-t border-border/70 pt-3">
              <NavItem disabled icon={<Settings />} label="Preferências" collapsed={!sidebar} />
              <div className="flex items-center gap-2.5 rounded-[9px] px-2 py-2">
                <span className="grid size-7 shrink-0 place-items-center rounded-full bg-primary/15 text-[11px] font-semibold text-primary">
                  FM
                </span>
                <span className={cn("min-w-0", !sidebar && "lg:hidden")}>
                  <span className="block truncate text-[12px] font-medium leading-tight">
                    Felipe Martins
                  </span>
                  <span className="block truncate text-[10px] leading-tight text-muted-foreground">
                    Ciência da Computação
                  </span>
                </span>
              </div>
            </div>
          </div>
        </aside>

        <main className="flex min-w-0 flex-1 flex-col bg-background">
          <header className="vibrancy sticky top-0 z-20 grid h-[52px] shrink-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 border-b border-border/70 bg-card/70 px-3 sm:px-4">
            <div className="flex items-center gap-1.5">
              <IconButton
                label={sidebar ? "Recolher barra lateral" : "Abrir barra lateral"}
                onClick={() => setSidebar(!sidebar)}
              >
                <PanelLeft />
              </IconButton>
              <span className="hidden h-5 w-px bg-border/80 sm:block" />
            </div>
            <div className="flex min-w-0 items-baseline gap-2">
              <h1 className="truncate text-[13px] font-semibold">{title}</h1>
              <p className="hidden truncate text-[11px] text-muted-foreground sm:block">
                Quinta-feira, 25 de setembro
              </p>
            </div>
            <div className="flex shrink-0 items-center gap-1.5">
              <IconButton label="Notificações" disabled>
                <Bell />
              </IconButton>
              <Button
                className="h-8 rounded-[8px] px-2.5 text-[12px] shadow-xs sm:px-3"
                onClick={() => setView("list")}
              >
                <Plus className="size-3.5" />
                <span className="hidden sm:inline">Nova tarefa</span>
              </Button>
            </div>
          </header>
          <div className="scrollbar-slim min-h-0 flex-1 overflow-auto">
            {view === "today" && (
              <TodayView
                done={done}
                toggleDone={(t) =>
                  setDone((c) => (c.includes(t) ? c.filter((i) => i !== t) : [...c, t]))
                }
                setView={setView}
              />
            )}
            {view === "calendar" && <CalendarScreen onOpenList={() => setView("list")} />}
            {view === "kanban" && <KanbanView setView={setView} />}
            {view === "list" && <ListView setView={setView} />}
            {view === "subject" && <SubjectView subject={activeSubject} />}
          </div>
        </main>
      </div>

      {searchOpen && (
        // Fechar a busca limpa o texto: reabrir com a consulta anterior faz a
        // usuária achar que a busca já filtrou quando ela não recarregou nada.
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
                placeholder="Buscar tarefas ou disciplinas…"
                aria-label="Buscar tarefas ou disciplinas"
                className="h-12 min-w-0 flex-1 bg-transparent text-[13px] outline-none placeholder:text-muted-foreground"
              />
              <kbd className="rounded-[5px] border border-border/70 bg-muted px-1.5 py-px text-[10px] text-muted-foreground">
                esc
              </kbd>
            </div>
            <div className="max-h-80 overflow-auto p-1.5">
              <p className="px-2.5 py-1.5 text-[10px] font-semibold uppercase tracking-[0.08em] text-muted-foreground/80">
                Resultados
              </p>
              {filtered.map((task) => (
                <button
                  key={task.title}
                  onClick={() => {
                    setView("list");
                    setSearchOpen(false);
                    setQuery("");
                  }}
                  className="flex w-full cursor-pointer items-center gap-3 rounded-[8px] px-2.5 py-2 text-left hover:bg-primary hover:text-primary-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                  <FileText className="size-4 opacity-70" />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-[13px] font-medium">{task.title}</span>
                    <span className="block text-[11px] opacity-70">{task.subject}</span>
                  </span>
                  <span className="text-[11px] tabular-nums opacity-70">{task.date}</span>
                </button>
              ))}
              {filtered.length === 0 && (
                <p className="px-2.5 py-6 text-center text-[12px] text-muted-foreground">
                  Nada encontrado para "{query}".
                </p>
              )}
            </div>
          </DialogContent>
        </Dialog>
      )}
    </TooltipProvider>
  );
}

function NavItem({
  icon,
  label,
  active,
  count,
  onClick,
  disabled,
  collapsed,
}: {
  icon: ReactNode;
  label: string;
  active?: boolean;
  count?: string;
  onClick?: () => void;
  disabled?: boolean;
  collapsed?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={cn(
        "flex h-8 w-full items-center gap-2.5 rounded-[8px] px-2 text-[12.5px] text-sidebar-foreground/80 transition-colors hover:bg-accent/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring [&_svg]:size-4",
        !disabled && "cursor-pointer",
        disabled && "cursor-not-allowed opacity-45 hover:bg-transparent",
        active && "bg-primary text-primary-foreground hover:bg-primary",
        collapsed && "lg:w-9 lg:justify-center lg:px-0",
      )}
    >
      <span className="grid size-4 shrink-0 place-items-center">{icon}</span>
      <span className={cn("truncate", collapsed && "lg:hidden")}>{label}</span>
      {count && (
        <span
          className={cn(
            "ml-auto rounded-full px-1.5 text-[10px] tabular-nums",
            active ? "bg-primary-foreground/20" : "bg-accent text-accent-foreground",
            collapsed && "lg:hidden",
          )}
        >
          {count}
        </span>
      )}
    </button>
  );
}

function SectionTitle({ children, action }: { children: ReactNode; action?: ReactNode }) {
  return (
    <div className="mb-3 flex items-center justify-between gap-3">
      <h2 className="text-[13px] font-semibold">{children}</h2>
      {action}
    </div>
  );
}

function Panel({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div
      className={cn(
        "shadow-float overflow-hidden rounded-[10px] border border-border/70 bg-card",
        className,
      )}
    >
      {children}
    </div>
  );
}

function TodayView({
  done,
  toggleDone,
  setView,
}: {
  done: string[];
  toggleDone: (title: string) => void;
  setView: (view: View) => void;
}) {
  return (
    <div className="mx-auto max-w-[1440px] p-4 sm:p-6">
      <div className="mb-6 flex flex-col items-start justify-between gap-3 sm:flex-row sm:items-end">
        <div>
          <p className="text-[11px] uppercase tracking-[0.08em] text-muted-foreground">
            Bom dia, Felipe
          </p>
          <h2 className="mt-1.5 text-[26px] font-semibold leading-tight tracking-[-0.02em]">
            O que precisa da sua atenção.
          </h2>
        </div>
        <div className="shadow-float flex items-center gap-2.5 rounded-[10px] border border-border/70 bg-card px-3 py-2">
          <Zap className="size-4 text-chart-5" />
          <div>
            <p className="text-[12px] font-medium">3 prioridades</p>
            <p className="text-[10.5px] text-muted-foreground">nas próximas 48 horas</p>
          </div>
        </div>
      </div>

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.45fr)_minmax(300px,0.75fr)]">
        <section>
          <SectionTitle
            action={
              <Button
                variant="ghost"
                className="h-7 rounded-[7px] px-2 text-[12px] text-muted-foreground"
                onClick={() => setView("kanban")}
              >
                Ver todas <ChevronRight className="size-3.5" />
              </Button>
            }
          >
            Prazos próximos
          </SectionTitle>
          <div className="grid gap-3 md:grid-cols-3">
            {[
              {
                date: "Hoje · 23:59",
                title: "Lista 4 — Integrais",
                sub: "Cálculo II",
                tone: "border-destructive/30 bg-destructive/8",
                label: "text-destructive",
              },
              {
                date: "Amanhã · 10:00",
                title: "Prova P1",
                sub: "Estruturas de Dados",
                tone: "border-chart-5/35 bg-chart-5/12",
                label: "text-chart-5",
              },
              {
                date: "Sex · 18:00",
                title: "Modelo ER",
                sub: "Banco de Dados",
                tone: "border-border/70 bg-card",
                label: "text-muted-foreground",
              },
            ].map((item) => (
              <article
                key={item.title}
                className={cn("shadow-float rounded-[10px] border p-3.5", item.tone)}
              >
                <div className="flex items-center justify-between">
                  <span
                    className={cn(
                      "text-[10.5px] font-semibold uppercase tracking-[0.06em]",
                      item.label,
                    )}
                  >
                    {item.date}
                  </span>
                  <MoreHorizontal className="size-4 text-muted-foreground" />
                </div>
                <h3 className="mt-6 text-[13.5px] font-semibold leading-snug">{item.title}</h3>
                <p className="mt-1 text-[11.5px] text-muted-foreground">{item.sub}</p>
              </article>
            ))}
          </div>

          <div className="mt-6">
            <SectionTitle
              action={
                <Button
                  variant="ghost"
                  className="h-7 rounded-[7px] px-2 text-[12px] text-muted-foreground"
                  onClick={() => setView("calendar")}
                >
                  Semana <ChevronRight className="size-3.5" />
                </Button>
              }
            >
              Sua linha do tempo
            </SectionTitle>
            <Panel>
              <div className="grid grid-cols-[72px_1fr] border-b border-border/70 bg-muted/60 px-3 py-2 text-[10px] font-semibold uppercase tracking-[0.06em] text-muted-foreground">
                <span>Horário</span>
                <span>Quinta, 25</span>
              </div>
              {[
                {
                  time: "08:00",
                  end: "09:40",
                  title: "Cálculo II",
                  room: "Bloco B · Sala 204",
                  color: "bg-chart-2",
                },
                {
                  time: "10:00",
                  end: "11:40",
                  title: "Estruturas de Dados",
                  room: "Lab. 03",
                  color: "bg-chart-3",
                },
                {
                  time: "14:00",
                  end: "15:30",
                  title: "Bloco de estudo · Prova P1",
                  room: "Biblioteca",
                  color: "bg-chart-5",
                },
                {
                  time: "16:00",
                  end: "17:40",
                  title: "Banco de Dados",
                  room: "Bloco A · Sala 112",
                  color: "bg-chart-1",
                },
              ].map((event) => (
                <div
                  key={event.time}
                  className="grid grid-cols-[72px_1fr] border-b border-border/60 px-3 py-2.5 transition-colors last:border-b-0 hover:bg-accent/40"
                >
                  <div>
                    <p className="text-[12px] font-medium tabular-nums">{event.time}</p>
                    <p className="text-[10.5px] tabular-nums text-muted-foreground">{event.end}</p>
                  </div>
                  <div className="flex min-w-0 items-center gap-3">
                    <span className={cn("h-8 w-[3px] rounded-full", event.color)} />
                    <div className="min-w-0">
                      <p className="truncate text-[12.5px] font-medium">{event.title}</p>
                      <p className="truncate text-[10.5px] text-muted-foreground">{event.room}</p>
                    </div>
                  </div>
                </div>
              ))}
            </Panel>
          </div>
        </section>

        <aside>
          <SectionTitle>
            <span className="flex items-center gap-2">
              Para avançar hoje{" "}
              <span className="rounded-full bg-accent px-2 py-0.5 text-[10px] font-medium text-accent-foreground">
                4
              </span>
            </span>
          </SectionTitle>
          <Panel>
            {tasks.map((task) => {
              const checked = done.includes(task.title);
              return (
                <button
                  key={task.title}
                  onClick={() => toggleDone(task.title)}
                  aria-pressed={checked}
                  className="flex w-full cursor-pointer items-start gap-3 border-b border-border/60 p-3 text-left transition-colors last:border-b-0 hover:bg-accent/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring"
                >
                  <span
                    aria-hidden="true"
                    className={cn(
                      "mt-0.5 grid size-[15px] shrink-0 place-items-center rounded-[5px] border transition-colors",
                      checked
                        ? "border-primary bg-primary text-primary-foreground"
                        : "border-input bg-card",
                    )}
                  >
                    {checked && <Check className="size-2.5" />}
                  </span>
                  <span className="min-w-0 flex-1">
                    <span
                      className={cn(
                        "block text-[12.5px] font-medium",
                        checked && "text-muted-foreground line-through",
                      )}
                    >
                      {task.title}
                    </span>
                    <span className="mt-1 flex items-center justify-between gap-2 text-[10.5px] text-muted-foreground">
                      <span className="truncate">{task.subject}</span>
                      <span className="shrink-0 tabular-nums">{task.date}</span>
                    </span>
                  </span>
                </button>
              );
            })}
          </Panel>

          <div className="mt-6">
            <SectionTitle>Progresso da semana</SectionTitle>
            <div className="shadow-float rounded-[10px] bg-primary p-4 text-primary-foreground">
              <div className="flex items-start justify-between">
                <div>
                  <p className="text-[11px] text-primary-foreground/75">Tempo focado</p>
                  <p className="mt-1 text-[22px] font-semibold tabular-nums leading-none">
                    12h 40min
                  </p>
                </div>
                <Sparkles className="size-5 text-primary-foreground/75" />
              </div>
              <div className="mt-4 h-1.5 rounded-full bg-primary-foreground/25">
                <div className="h-full w-[68%] rounded-full bg-primary-foreground" />
              </div>
              <p className="mt-2 text-[10.5px] text-primary-foreground/75">
                68% da meta semanal de 18h
              </p>
            </div>
          </div>
        </aside>
      </div>
    </div>
  );
}

function ViewSwitch({ active, setView }: { active: View; setView: (view: View) => void }) {
  const options: { key: View; label: string; icon: ReactNode }[] = [
    { key: "calendar", label: "Grade", icon: <CalendarDays className="size-3.5" /> },
    { key: "kanban", label: "Kanban", icon: <LayoutDashboard className="size-3.5" /> },
    { key: "list", label: "Lista", icon: <List className="size-3.5" /> },
  ];
  return (
    <div
      role="tablist"
      aria-label="Alternar visão"
      className="flex rounded-[9px] border border-border/70 bg-muted p-[3px]"
    >
      {options.map((option) => (
        <button
          key={option.key}
          role="tab"
          aria-selected={active === option.key}
          onClick={() => setView(option.key)}
          className={cn(
            "flex h-[26px] cursor-pointer items-center gap-1.5 rounded-[6px] px-2.5 text-[12px] text-muted-foreground transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
            active === option.key && "bg-card font-medium text-foreground shadow-xs",
          )}
        >
          {option.icon}
          {option.label}
        </button>
      ))}
    </div>
  );
}

const columns = [
  { name: "A fazer", tone: "bg-muted-foreground", items: tasks.slice(2, 3) },
  { name: "Em andamento", tone: "bg-chart-5", items: tasks.slice(0, 2) },
  {
    name: "Em revisão",
    tone: "bg-chart-3",
    items: tasks.slice(3, 4).filter((task): task is Task => Boolean(task)),
  },
  {
    name: "Concluído",
    tone: "bg-chart-1",
    items: [
      {
        ...(tasks[3] ??
          tasks[0] ?? {
            title: "Resumo",
            subject: "Banco de Dados",
            date: "Seg, 08:00",
            tag: "Leitura",
            progress: 0,
          }),
        title: "Resumo — Normalização",
        progress: 100,
      },
    ],
  },
];

const kanbanTotal = columns.reduce((total, column) => total + column.items.length, 0);

function KanbanView({ setView }: { setView: (view: View) => void }) {
  return (
    <div className="p-4 sm:p-6">
      <div className="mb-5 flex flex-wrap items-center justify-between gap-3">
        <p className="text-[12px] text-muted-foreground">
          {kanbanTotal} atividades · {subjects.length} disciplinas
        </p>
        <ViewSwitch active="kanban" setView={setView} />
      </div>
      <div className="grid min-w-[900px] grid-cols-4 gap-3">
        {columns.map((column) => (
          <section key={column.name} className="min-w-0 rounded-[10px] bg-muted/50 p-2">
            <div className="mb-2 flex items-center gap-2 px-1.5 py-1">
              <span className={cn("size-2 rounded-full", column.tone)} />
              <h2 className="text-[12px] font-semibold">{column.name}</h2>
              <span className="text-[10.5px] tabular-nums text-muted-foreground">
                {column.items.length}
              </span>
              <MoreHorizontal className="ml-auto size-4 text-muted-foreground" />
            </div>
            <div className="space-y-2">
              {column.items.map((task) => (
                <article
                  key={task.title}
                  /* Realce por cor, não por elevating: `hover:-translate-y-px`
                     promovia uma camada nova por quadro em 20 cards, e 1px de
                     deslocamento é subliminar demais para justificar o custo
                     (X7, X8). */
                  className="shadow-float rounded-[9px] border border-border/70 bg-card p-3 transition-colors duration-150 hover:border-ring/60 hover:bg-accent/20"
                >
                  <div className="flex items-center justify-between">
                    <span
                      className={cn(
                        "rounded-[5px] px-1.5 py-0.5 text-[9.5px] font-medium",
                        task.tag === "Urgente"
                          ? "bg-destructive/12 text-destructive"
                          : task.tag === "Prova"
                            ? "bg-chart-5/16 text-chart-5"
                            : "bg-accent text-accent-foreground",
                      )}
                    >
                      {task.tag}
                    </span>
                    <MoreHorizontal className="size-3.5 text-muted-foreground" />
                  </div>
                  <h3 className="mt-4 text-[12.5px] font-semibold leading-5">{task.title}</h3>
                  <p className="mt-1 text-[10.5px] text-muted-foreground">{task.subject}</p>
                  <div className="mt-4 flex items-center justify-between border-t border-border/60 pt-2">
                    <span className="flex items-center gap-1 text-[10px] tabular-nums text-muted-foreground">
                      <Clock3 className="size-3" />
                      {task.date}
                    </span>
                    <span className="grid size-5 place-items-center rounded-full bg-primary/15 text-[8.5px] font-semibold text-primary">
                      FM
                    </span>
                  </div>
                </article>
              ))}
            </div>
            <Button
              variant="ghost"
              disabled
              className="mt-2 h-8 w-full justify-start rounded-[8px] text-[12px] text-muted-foreground"
            >
              <Plus className="size-3.5" />
              Adicionar
            </Button>
          </section>
        ))}
      </div>
    </div>
  );
}

function ListView({ setView }: { setView: (view: View) => void }) {
  return (
    <div className="p-4 sm:p-6">
      <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <p className="text-[12px] text-muted-foreground">
          {tasks.length} atividades em {subjects.length} disciplinas
        </p>
        <ViewSwitch active="list" setView={setView} />
      </div>
      <Panel className="overflow-x-auto">
        <table className="w-full min-w-[800px] border-collapse text-left">
          <thead className="bg-muted/60 text-[10px] font-semibold uppercase tracking-[0.06em] text-muted-foreground">
            <tr>
              <th className="px-4 py-2.5 font-semibold">Atividade</th>
              <th className="px-4 py-2.5 font-semibold">Disciplina</th>
              <th className="px-4 py-2.5 font-semibold">Tipo</th>
              <th className="px-4 py-2.5 font-semibold">Prazo</th>
              <th className="px-4 py-2.5 font-semibold">Progresso</th>
              <th className="w-10" />
            </tr>
          </thead>
          <tbody>
            {tasks.map((task) => (
              <tr
                key={task.title}
                className="border-t border-border/60 transition-colors hover:bg-accent/40"
              >
                <td className="px-4 py-2.5 text-[12.5px] font-medium">{task.title}</td>
                <td className="px-4 py-2.5 text-[12px] text-muted-foreground">{task.subject}</td>
                <td className="px-4 py-2.5">
                  <span className="rounded-[5px] bg-accent px-2 py-0.5 text-[10.5px] text-accent-foreground">
                    {task.tag}
                  </span>
                </td>
                <td className="px-4 py-2.5 text-[12px] tabular-nums text-muted-foreground">
                  {task.date}
                </td>
                <td className="px-4 py-2.5">
                  <div className="flex items-center gap-2">
                    <div className="h-1.5 w-20 rounded-full bg-muted">
                      <div
                        className="h-full rounded-full bg-primary"
                        style={{ width: `${task.progress}%` }}
                      />
                    </div>
                    <span className="text-[10.5px] tabular-nums text-muted-foreground">
                      {task.progress}%
                    </span>
                  </div>
                </td>
                <td>
                  <MoreHorizontal className="size-4 text-muted-foreground" />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </Panel>
    </div>
  );
}

const subjectTabs = [
  { key: "atividades", label: "Tarefas & trabalhos" },
  { key: "notas", label: "Notas & média" },
  { key: "anotacoes", label: "Anotações" },
] as const;

type SubjectTabKey = (typeof subjectTabs)[number]["key"];

function SubjectView({ subject }: { subject: Subject }) {
  const [tab, setTab] = useState<SubjectTabKey>("atividades");
  const tabRefs = useRef<Record<string, HTMLButtonElement | null>>({});

  /**
   * Padrão de tabs completo: setas movem a seleção, `Home`/`End` vão às pontas.
   * O `aria-selected` sozinho não é acessível — sem `tabpanel` vinculado, um
   * leitor de tela anuncia "aba" sem nunca dizer o que a aba contém.
   */
  const onTabKeyDown = (event: ReactKeyboardEvent<HTMLButtonElement>) => {
    const step =
      event.key === "ArrowRight"
        ? 1
        : event.key === "ArrowLeft"
          ? -1
          : event.key === "Home"
            ? -99
            : event.key === "End"
              ? 99
              : 0;
    if (step === 0) return;
    event.preventDefault();
    const current = subjectTabs.findIndex((item) => item.key === tab);
    const next =
      step === -99
        ? 0
        : step === 99
          ? subjectTabs.length - 1
          : (current + step + subjectTabs.length) % subjectTabs.length;
    const target = subjectTabs[next];
    if (!target) return;
    setTab(target.key);
    tabRefs.current[target.key]?.focus();
  };

  return (
    <div className="p-4 sm:p-6">
      <div className="mb-5 flex items-center gap-3">
        <div
          className={cn(
            "grid size-11 place-items-center rounded-[10px] border text-[13px] font-semibold",
            subject.soft,
          )}
        >
          {subject.short}
        </div>
        <div>
          <h2 className="text-[20px] font-semibold tracking-[-0.015em]">{subject.name}</h2>
          <p className="text-[12px] text-muted-foreground">Prof. Marina Costa · Seg/Qui, 08:00</p>
        </div>
      </div>
      <div
        className="mb-5 flex border-b border-border/70"
        role="tablist"
        aria-label="Seções da disciplina"
      >
        {subjectTabs.map((item) => (
          <button
            key={item.key}
            ref={(node) => {
              tabRefs.current[item.key] = node;
            }}
            id={`subject-tab-${item.key}`}
            role="tab"
            aria-selected={tab === item.key}
            aria-controls={`subject-panel-${item.key}`}
            // Só a aba ativa entra na ordem de tabulação; as outras são
            // alcançadas pelas setas, que é o esperado para um tablist.
            tabIndex={tab === item.key ? 0 : -1}
            onKeyDown={onTabKeyDown}
            onClick={() => setTab(item.key)}
            className={cn(
              "-mb-px cursor-pointer border-b-2 border-transparent px-3 py-2 text-[12.5px] text-muted-foreground transition-colors hover:text-foreground",
              tab === item.key && "border-primary font-medium text-foreground",
            )}
          >
            {item.label}
          </button>
        ))}
      </div>

      <div
        role="tabpanel"
        id={`subject-panel-${tab}`}
        aria-labelledby={`subject-tab-${tab}`}
        tabIndex={0}
      >
        {tab === "atividades" && (
          <div className="max-w-4xl">
            <SectionTitle>Atividades da disciplina</SectionTitle>
            <Panel>
              {tasks.slice(0, 3).map((task) => (
                <div
                  key={task.title}
                  className="flex items-center gap-3 border-b border-border/60 p-3 last:border-b-0"
                >
                  <Circle className="size-4 text-muted-foreground" />
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-[12.5px] font-medium">{task.title}</p>
                    <p className="text-[10.5px] tabular-nums text-muted-foreground">{task.date}</p>
                  </div>
                  <span className="rounded-[5px] bg-accent px-2 py-0.5 text-[10.5px] text-accent-foreground">
                    {task.tag}
                  </span>
                </div>
              ))}
            </Panel>
          </div>
        )}

        {tab === "notas" && (
          <div className="grid max-w-4xl gap-4 md:grid-cols-3">
            <Metric label="Média atual" value="8,2" detail="Meta: 7,0" />
            <Metric label="Frequência" value="92%" detail="6 de 20 faltas usadas" />
            <Metric label="Próxima avaliação" value="P2" detail="08 de outubro" />
            <Panel className="p-4 md:col-span-3">
              <SectionTitle>Composição da média</SectionTitle>
              {[
                ["Lista 1", "9,0"],
                ["Prova P1", "7,8"],
                ["Projeto", "8,5"],
              ].map(([name = "", grade = ""]) => (
                <div
                  key={name}
                  className="flex items-center justify-between border-b border-border/60 py-2.5 last:border-b-0"
                >
                  <span className="text-[12.5px]">{name}</span>
                  <span className="text-[14px] font-semibold tabular-nums">{grade}</span>
                </div>
              ))}
            </Panel>
          </div>
        )}

        {tab === "anotacoes" && (
          <div className="grid max-w-4xl gap-3 md:grid-cols-2">
            {[
              "Integrais por partes",
              "Métodos de substituição",
              "Revisão para P1",
              "Exercícios resolvidos",
            ].map((note, i) => (
              <Panel key={note} className="p-4">
                <FileText className="size-4 text-muted-foreground" />
                <h3 className="mt-6 text-[13.5px] font-semibold">{note}</h3>
                <p className="mt-1 text-[11.5px] text-muted-foreground">
                  Aula {12 - i} · atualizado há {i + 1} dias
                </p>
              </Panel>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function Metric({ label, value, detail }: { label: string; value: string; detail: string }) {
  return (
    <Panel className="p-4">
      <p className="text-[11.5px] text-muted-foreground">{label}</p>
      <p className="mt-2 text-[24px] font-semibold tabular-nums leading-none">{value}</p>
      <p className="mt-2 text-[10.5px] text-muted-foreground">{detail}</p>
    </Panel>
  );
}
