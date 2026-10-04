/**
 * Registro de módulos do Workspace Acadêmico.
 *
 * D83 — este arquivo é a **fonte única** de nome de módulo, de dono de dado e de
 * rota. Nenhum outro arquivo escreve um rótulo de módulo, e
 * `scripts/check-modules.mjs` falha se um rótulo aparecer fora daqui.
 *
 * Origem: `spec-referencial-workspace-academico.md` §2 linhas 36-55 (entradas e
 * utilitários), §2 linha 58 (nomes fechados), §2 linha 60 (ordem da sidebar,
 * `[P]`), §2 linhas 64-74 (destinos de navegação, `[D]`), §1.3 linha 20 (cada
 * dado tem um dono), §3.1 linhas 84-96 (matriz de dono), §1.9 linha 26 (teto de
 * quatro destinos por módulo, `[D]`).
 *
 * Decisão: `SPEC-D-013` `D83` e `D84`.
 * Onde esta regra é garantida: `scripts/check-modules.mjs` e
 * `src/shell/__tests__/modules.test.ts`.
 *
 * Nomes fechados: "Sala de treino" é o nome do módulo (§2 linha 58). "Sala de
 * atendimento" é do Workspace Profissional, que §2 linha 47 põe fora de escopo, e
 * por isso não aparece aqui. O termo "Laboratório de Testes" foi abandonado
 * (§2 linha 58) e não pode reaparecer.
 */

/** Os nove módulos do Workspace Acadêmico, na ordem da árvore de §2 linhas 37-45. */
export type ModuleId =
  | "home"
  | "calendario"
  | "faculdade"
  | "estudos"
  | "estagio"
  | "tcc"
  | "sala-de-treino"
  | "conhecimento"
  | "biblioteca";

/** Os quatro utilitários globais de §2 linhas 51-55. */
export type UtilityId = "ceci" | "busca" | "notificacoes" | "configuracoes";

/**
 * Dono de dado, de §3.1 linhas 84-96. Um literal, não `string`: um módulo que
 * declara dono que não existe é erro de tipo, não erro de runtime.
 *
 * `nenhum` existe porque Home **não é dona de nada** — §4.1 linha 120 é `[D]`.
 */
export type DataOwner =
  | "nenhum"
  | "semestre"
  | "disciplina"
  | "aula"
  | "frequencia"
  | "avaliacao"
  | "regra-media"
  | "responsabilidade"
  | "evento"
  | "bloco-planejamento"
  | "ocorrencia"
  | "registro-execucao"
  | "conteudo"
  | "sessao-estudo"
  | "flashcard"
  | "questao"
  | "trilha"
  | "historico-fsrs"
  | "item-biblioteca"
  | "referencia"
  | "anexo"
  | "fragmento"
  | "conceito"
  | "relacao"
  | "projeto"
  | "base-pesquisa"
  | "documento"
  | "versao"
  | "saida"
  | "hora-estagio"
  | "supervisao"
  | "entrega-estagio"
  | "registro-clinico-estagio"
  | "personagem-simulado"
  | "sessao-simulada"
  | "feedback-sessao"
  | "desempenho"
  | "identidade"
  | "preferencia";

/**
 * Um destino de navegação. §1.9 linha 26 é `[D]`: teto de **quatro** por módulo,
 * e "detalhes e fluxos (ficha de um item, sessão em andamento, painéis laterais,
 * modos de tela cheia, ações como Importar) **não** contam como destino".
 */
export interface ModuleDestination {
  /** Identificador do destino, literal por módulo. */
  readonly id: string;
  /** Nome do destino, exatamente como §2 linhas 64-74 escreve. */
  readonly rotulo: string;
  /** Caminho de rota, relativo à raiz do shell. */
  readonly rota: string;
  /**
   * `true` quando o destino é um **modo de tela cheia** e, por §1.9 linha 26,
   * não conta para o teto de quatro. Os modos de tela cheia que o referencial
   * descreve são a revisão de Estudos (§4.4 linha 295), a sessão da Sala de
   * treino (§4.9 linha 510) e o editor do TCC (§4.7 linha 416).
   */
  readonly modoTelaCheia?: boolean;
  /**
   * `true` quando o destino **não** é possível entregar agora, porque o produto
   * ainda não decidiu. §4.6 linha 387 é `[P]`, §4.8 linha 479 e §8 linha 606 são
   * `[A]`.
   *
   * Quando o bloqueio é `[A]`, ele **não impede a escrita da `SPEC-D`** — impede a
   * implementação, e o motivo fica escrito. Item `[A]` não é fechado por agente
   * (`ADR-005`), e esconder isso atrás de uma tela vazia seria o oposto de
   * honesto.
   */
  readonly bloqueadoPor?: readonly string[];
  /** A spec `SPEC-D` deste destino, quando ela já existe. `null` = a escrever. */
  readonly spec: string | null;
}

export interface ModuleEntry {
  readonly id: ModuleId;
  /** Nome fechado de §2. É `[D]` e não é livre. */
  readonly rotulo: string;
  /**
   * Quem é dono do dado do módulo, de §3.1. `readonly dono[]` porque um módulo
   * pode ser dono de mais de um dado — Faculdade é dono de cinco.
   */
  readonly dono: readonly DataOwner[];
  /** Raiz de rota. Todo destino deste módulo começa com ela. */
  readonly rota: string;
  /** Os destinos de navegação, de §2 linhas 64-74. */
  readonly destinos: readonly ModuleDestination[];
  /**
   * `true` quando **não** é possível entregar o módulo agora, e o motivo está
   * `[A]` na spec referencial. Isso é informação de bloqueio, não de produto: o
   * destino continua no registro, e continua sendo rota.
   */
  readonly bloqueadoPor?: readonly string[];
}

/**
 * Os nove módulos, na ordem da árvore de §2 linhas 37-45.
 *
 * Esta ordem **não** é a da sidebar. §2 linha 60 dá a ordem da sidebar e a marca
 * `[P]`, e ela põe Biblioteca e Conhecimento antes de TCC, e TCC antes de
 * Estágio. As duas ordens coexistem e as duas estão no referencial; a sidebar é
 * `[P]`, a árvore é a ordem de leitura do documento.
 */
export const MODULES: readonly ModuleEntry[] = [
  {
    id: "home",
    rotulo: "Home",
    dono: ["nenhum"],
    rota: "/home",
    destinos: [
      {
        id: "tela",
        rotulo: "Home",
        rota: "/home",
        spec: "SPEC-D-014",
      },
    ],
  },
  {
    id: "calendario",
    rotulo: "Calendário",
    // §3.1 linha 87 e 88: Responsabilidade (item sem nota), Evento, Bloco de
    // planejamento, Ocorrência, Registro de execução.
    dono: ["responsabilidade", "evento", "bloco-planejamento", "ocorrencia", "registro-execucao"],
    rota: "/calendario",
    destinos: [
      { id: "semana", rotulo: "Semana", rota: "/calendario/semana", spec: "SPEC-D-015" },
      { id: "mes", rotulo: "Mês", rota: "/calendario/mes", spec: "SPEC-D-016" },
      { id: "agenda", rotulo: "Agenda", rota: "/calendario/agenda", spec: "SPEC-D-017" },
      {
        id: "planejamento",
        rotulo: "Planejamento",
        rota: "/calendario/planejamento",
        spec: "SPEC-D-018",
      },
    ],
  },
  {
    id: "faculdade",
    rotulo: "Faculdade",
    // §3.1 linhas 84, 85 e 86.
    dono: ["semestre", "disciplina", "aula", "frequencia", "avaliacao", "regra-media"],
    rota: "/faculdade",
    destinos: [
      {
        id: "visao-geral",
        rotulo: "Visão geral",
        rota: "/faculdade/visao-geral",
        spec: "SPEC-D-019",
      },
      {
        id: "disciplinas",
        rotulo: "Disciplinas",
        rota: "/faculdade/disciplinas",
        spec: "SPEC-D-020",
      },
      {
        id: "avaliacoes",
        rotulo: "Avaliações",
        rota: "/faculdade/avaliacoes",
        spec: "SPEC-D-021",
      },
      {
        id: "historico",
        rotulo: "Histórico",
        rota: "/faculdade/historico",
        spec: "SPEC-D-022",
      },
    ],
  },
  {
    id: "estudos",
    rotulo: "Estudos",
    // §3.1 linhas 89 e 90.
    dono: ["conteudo", "sessao-estudo", "flashcard", "questao", "trilha", "historico-fsrs"],
    rota: "/estudos",
    destinos: [
      { id: "hoje", rotulo: "Hoje", rota: "/estudos/hoje", spec: "SPEC-D-023" },
      {
        id: "conteudos",
        rotulo: "Conteúdos",
        rota: "/estudos/conteudos",
        spec: "SPEC-D-024",
      },
      { id: "pratica", rotulo: "Prática", rota: "/estudos/pratica", spec: "SPEC-D-025" },
      {
        id: "sessoes-e-historico",
        rotulo: "Sessões e Histórico",
        rota: "/estudos/sessoes-e-historico",
        spec: "SPEC-D-026",
      },
    ],
  },
  {
    id: "estagio",
    rotulo: "Estágio",
    // §3.1 linha 94 (camada acadêmica) e linha 95 (camada clínica, só desktop).
    dono: ["hora-estagio", "supervisao", "entrega-estagio", "registro-clinico-estagio"],
    rota: "/estagio",
    destinos: [
      {
        id: "visao-geral",
        rotulo: "Visão geral",
        rota: "/estagio/visao-geral",
        spec: "SPEC-D-037",
      },
      {
        id: "horas-e-supervisoes",
        rotulo: "Horas e Supervisões",
        rota: "/estagio/horas-e-supervisoes",
        spec: "SPEC-D-038",
      },
      {
        id: "entregas-e-relatorios",
        rotulo: "Entregas e relatórios",
        rota: "/estagio/entregas-e-relatorios",
        spec: "SPEC-D-039",
      },
      {
        // §4.8 linha 479 é [A] e trava retenção, exclusão e nível de sigilo.
        id: "atendimentos",
        rotulo: "Atendimentos",
        rota: "/estagio/atendimentos",
        spec: "SPEC-D-040",
        bloqueadoPor: ["[A] spec referencial §4.8 linha 479 — retenção, exclusão e sigilo"],
      },
    ],
  },
  {
    id: "tcc",
    rotulo: "TCC & Projetos",
    // §3.1 linha 93.
    dono: ["projeto", "base-pesquisa", "documento", "versao", "saida"],
    rota: "/tcc",
    // §2 linha 72: no nível do módulo existe **só** a lista Projetos, e dentro de
    // um projeto aberto são quatro. Por §1.9 linha 26 o limite de quatro é por
    // nível, e por isso esta entrada tem cinco destinos — um na raiz do módulo e
    // quatro sob `/projetos/:id`. Isso é o que a linha 72 descreve, e não é
    // excedente: ver `scripts/check-modules.mjs`, que conta por prefixo de rota.
    destinos: [
      { id: "projetos", rotulo: "Projetos", rota: "/tcc/projetos", spec: "SPEC-D-032" },
      {
        id: "projeto-visao-geral",
        rotulo: "Visão geral",
        rota: "/tcc/projetos/:projetoId/visao-geral",
        spec: "SPEC-D-033",
      },
      {
        id: "projeto-escrita",
        rotulo: "Escrita",
        rota: "/tcc/projetos/:projetoId/escrita",
        spec: "SPEC-D-034",
        modoTelaCheia: true,
      },
      {
        id: "projeto-pesquisa",
        rotulo: "Pesquisa",
        rota: "/tcc/projetos/:projetoId/pesquisa",
        spec: "SPEC-D-035",
      },
      {
        id: "projeto-orientacao-e-entrega",
        rotulo: "Orientação e Entrega",
        rota: "/tcc/projetos/:projetoId/orientacao-e-entrega",
        spec: "SPEC-D-036",
      },
    ],
  },
  {
    id: "sala-de-treino",
    rotulo: "Sala de treino",
    // §3.1 linha 96.
    dono: ["personagem-simulado", "sessao-simulada", "feedback-sessao", "desempenho"],
    rota: "/sala-de-treino",
    bloqueadoPor: [
      "[A] spec referencial §8 linha 609 — revalidar as premissas técnicas com protótipo",
    ],
    destinos: [
      {
        id: "personagens",
        rotulo: "Personagens",
        rota: "/sala-de-treino/personagens",
        spec: "SPEC-D-041",
      },
      {
        id: "novo-atendimento",
        rotulo: "Novo atendimento",
        rota: "/sala-de-treino/novo-atendimento",
        spec: "SPEC-D-042",
      },
      {
        id: "historico-de-sessoes",
        rotulo: "Histórico de sessões",
        rota: "/sala-de-treino/historico-de-sessoes",
        spec: "SPEC-D-043",
      },
      {
        id: "desempenho",
        rotulo: "Desempenho",
        rota: "/sala-de-treino/desempenho",
        spec: "SPEC-D-044",
      },
    ],
  },
  {
    id: "conhecimento",
    rotulo: "Conhecimento",
    // §3.1 linha 92.
    dono: ["fragmento", "conceito", "relacao"],
    rota: "/conhecimento",
    destinos: [
      {
        id: "entrada",
        rotulo: "Entrada",
        rota: "/conhecimento/entrada",
        spec: "SPEC-D-028",
        // §8 linha 606 é [A] e trava o escopo do leitor de PDF, que alimenta a
        // Entrada (§4.6 linha 380).
        bloqueadoPor: ["[A] spec referencial §8 linha 606 — escopo do leitor de PDF"],
      },
      {
        id: "fragmentos",
        rotulo: "Fragmentos",
        rota: "/conhecimento/fragmentos",
        spec: "SPEC-D-029",
      },
      {
        id: "conceitos",
        rotulo: "Conceitos",
        rota: "/conhecimento/conceitos",
        spec: "SPEC-D-030",
      },
      {
        id: "relacoes",
        rotulo: "Relações",
        rota: "/conhecimento/relacoes",
        spec: "SPEC-D-031",
      },
    ],
  },
  {
    id: "biblioteca",
    rotulo: "Biblioteca",
    // §3.1 linha 91.
    dono: ["item-biblioteca", "referencia", "anexo"],
    rota: "/biblioteca",
    destinos: [
      {
        id: "acervo",
        rotulo: "Acervo",
        rota: "/biblioteca/acervo",
        spec: "SPEC-D-027",
      },
    ],
  },
];

export interface UtilityEntry {
  readonly id: UtilityId;
  readonly rotulo: string;
  readonly rota: string;
  /** Onde fica na interface. §2 linha 60 põe os três primeiros no rodapé. */
  readonly posicao: "rodape" | "fora";
  readonly spec: string | null;
}

/**
 * Os quatro utilitários globais de §2 linhas 51-55.
 *
 * `configuracoes` não tem posição na sidebar em §2 linha 60 — a linha lista o
 * rodapé com três itens e não menciona Configurações. Deixá-lo como `"fora"` é
 * registrar a lacuna do documento, e não inventar um lugar.
 */
export const UTILIDADES: readonly UtilityEntry[] = [
  {
    id: "ceci",
    rotulo: "Ceci",
    rota: "/ceci",
    posicao: "rodape",
    spec: "SPEC-D-045",
  },
  {
    id: "busca",
    rotulo: "Busca",
    rota: "/busca",
    posicao: "rodape",
    spec: "SPEC-D-046",
  },
  {
    id: "notificacoes",
    rotulo: "Notificações",
    rota: "/notificacoes",
    posicao: "rodape",
    spec: "SPEC-D-047",
  },
  {
    id: "configuracoes",
    rotulo: "Configurações / Perfil",
    rota: "/configuracoes",
    posicao: "fora",
    spec: "SPEC-D-048",
  },
];

/**
 * A ordem da sidebar do Acadêmico, de §2 linha 60, com a marca `[P]` do próprio
 * documento. Ela difere da ordem da árvore em três pares: Biblioteca e
 * Conhecimento vêm antes de TCC, e TCC vem antes de Estágio.
 *
 * `[P]` significa que esta ordem **não** é implementável como fato sem
 * confirmação da dona do produto. Ela está escrita para que a substituição seja
 * de uma linha, e não para que seja tratada como decidida.
 */
export const ORDEM_SIDEBAR: readonly ModuleId[] = [
  "home",
  "calendario",
  "faculdade",
  "estudos",
  "biblioteca",
  "conhecimento",
  "tcc",
  "estagio",
  "sala-de-treino",
];

/**
 * As seções em que a sidebar agrupa os nove módulos.
 *
 * **Os três nomes são proposta, e é por isso que esta tabela é `[P]` junto com
 * `ORDEM_SIDEBAR`.** A spec referencial não tem cabeçalho de seção: §2 linha 60 dá
 * a **ordem** dos nove e a marca `[P]`, e nada mais. Nomear seções é decisão de
 * produto na mesma proporção que ordenar a lista é, e um agente que escreve
 * "Dia a dia" está criando um nome que a dona do produto não escreveu.
 *
 * O que **não** é proposta é o agrupamento em si, e a razão é de leitura, não de
 * gosto: com nove destinos em linha única não há hierarquia nenhuma, e a lista só
 * piora a cada módulo novo. A regra é que os **nomes** é que são `[P]`.
 *
 * Três seções, e não uma por módulo nem uma por camada do dado: a §3.1 (dono de
 * dado) é um recorte de **sistema**, e navegar por ele é o mesmo erro de
 * agrupar por organograma. O corte é por **tarefa**, na ordem que `ORDEM_SIDEBAR`
 * já define — o que faz `gruposDaSidebar()` ser derivado e não escolhido.
 */
export interface SecaoSidebar {
  readonly id: string;
  readonly nome: string;
  readonly modulos: readonly ModuleId[];
}

export const SECOES_SIDEBAR: readonly SecaoSidebar[] = [
  { id: "dia-a-dia", nome: "Dia a dia", modulos: ["home", "calendario", "faculdade", "estudos"] },
  {
    id: "consulta",
    nome: "Consulta",
    modulos: ["biblioteca", "conhecimento"],
  },
  {
    id: "producao",
    nome: "Produção",
    modulos: ["tcc", "estagio", "sala-de-treino"],
  },
];

/**
 * Uma seção com os módulos já resolvidos em entradas do registro.
 *
 * Tipo **separado** de `SecaoSidebar` e não uma interseção: as duas têm `modulos`
 * com tipos diferentes (`ModuleId` e `ModuleEntry`), e interseção de propriedade
 * com tipo incompatível é erro de tipo, não de valor.
 */
export interface SecaoSidebarResolvida {
  readonly id: string;
  readonly nome: string;
  readonly modulos: readonly ModuleEntry[];
}

/**
 * As seções com os módulos resolvidos, na ordem de `ORDEM_SIDEBAR`.
 *
 * Derivada, nunca escrita: a seção traz o `id` do módulo e o `rotulo` vem do
 * registro por `moduloPorId`. Uma lista de rótulos aqui dentro seria uma **segunda**
 * fonte de nome de módulo — que é exatamente o que `D83` proíbe e o que
 * `scripts/check-modules.mjs` reprova na regra 5.
 */
export function gruposDaSidebar(): readonly SecaoSidebarResolvida[] {
  return GRUPOS_RESOLVIDOS;
}

/**
 * A mesma lista, resolvida uma vez.
 *
 * `SECOES_SIDEBAR` e `MODULES` são constantes de módulo, então resolver uma a partir
 * da outro é trabalho de módulo e não de render. Sem isto, `BarraLateral` refazia
 * três `map` e nove `find` a cada tecla digitada no campo de busca.
 */
const GRUPOS_RESOLVIDOS: readonly SecaoSidebarResolvida[] = SECOES_SIDEBAR.map((secao) => ({
  id: secao.id,
  nome: secao.nome,
  modulos: secao.modulos.map((id) => moduloPorId(id)),
}));

/** Todos os destinos de todos os módulos, na ordem do registro. */
export function todosOsDestinos(): readonly ModuleDestination[] {
  return MODULES.flatMap((m) => m.destinos);
}

/** Módulo que contém a rota dada, ou `undefined` se a rota não é de módulo. */
export function moduloDaRota(rota: string): ModuleEntry | undefined {
  return MODULES.find((m) => rota === m.rota || rota.startsWith(`${m.rota}/`));
}

/** Entrada do registro com o `id` dado. */
export function moduloPorId(id: ModuleId): ModuleEntry {
  const encontrado = MODULES.find((m) => m.id === id);
  if (!encontrado) throw new Error(`moduleId inexistente no registro: ${id}`);
  return encontrado;
}

/**
 * Total de destinos de navegação, para a métrica da `SPEC-D-013` §9.
 *
 * A soma é 31 e vem de §2 linhas 64-74. Ela é **derivada**, nunca escrita: o
 * comando que a produz é o gate `npm run gate:modules`.
 */
export function totalDeDestinos(): number {
  return todosOsDestinos().length;
}
