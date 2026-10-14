/**
 * A tabela de navegação do shell (`SPEC-D-006`, `D62` e `D63`).
 *
 * Um lugar, e é o único. A sidebar, o breadcrumb, o `⌘K` e a rota atual leem
 * daqui — que é o que elimina a classe de bug em que a sidebar e a navegação
 * interna discordam sobre quantos módulos existem.
 *
 * **Isto é dado, não componente.** Não tem React, não tem ícone, não tem cor. O
 * ícone é apresentação e mora no componente, porque `AGENTS.md` separa as duas
 * coisas e porque um `.ts` puro é verificável por `node --test` sem DOM, sem
 * bundler e sem o toolchain inteiro — e a regra de §1.2 ("uma entidade só vira
 * tela quando há atividade recorrente") precisa de **teste**, não de leitura.
 *
 * A origem é a spec referencial §2 e §3.1, com os nomes fechados e o dono do
 * dado de cada módulo. A **ordem** é `[P]` na spec referencial e continua sendo
 * decisão de produto: esta tabela a reproduz, e mudar a ordem aqui é ato de
 * produto, com data `D<n>` e quem decidiu — não ajuste de implementação.
 */

export type ModuleId =
  | "home"
  | "calendario"
  | "faculdade"
  | "estudos"
  | "biblioteca"
  | "conhecimento"
  | "tcc"
  | "estagio"
  | "sala-de-treino";

export type DestinoId = string;

/**
 * `pronto` significa que o destino lê e aceita dado real. `em-construcao` que
 * a rota existe e a tela é honesta sobre não existir.
 *
 * A distinção é o que impede a navegação de ser efeito colateral do estado do
 * código: um módulo em construção **aparece**, com o estado declarado. Entrada que
 * some quando o build falha é pior que entrada incompleta.
 */
export type EstadoDestino = "pronto" | "em-construcao";

export interface Destino {
  readonly id: DestinoId;
  /** Rótulo fechado. "Novo atendimento" e "Sala de treino" não têm sinônimo. */
  readonly titulo: string;
  readonly estado: EstadoDestino;
  /**
   * O que a tela vai ser, em uma frase. Existe porque o estado vazio de um módulo
   * que não existe precisa **dizer** o que ele vai ser — uma tela em branco com
   * "em construção" é indistinguível de um bug de render.
   */
  readonly descricao: string;
}

export interface Modulo {
  readonly id: ModuleId;
  /** Nome fechado de §2. Verificado por teste: o nome antigo abandonado não volta. */
  readonly titulo: string;
  /**
   * O módulo dono do dado principal deste módulo (`SPEC-D-007`, `D65`). Uma
   * entrada de navegação que representa dado sem dono declarado não é entrada.
   * `null` é só a Home, que por §4.1 **não é dona de nada**.
   */
  readonly dono: ModuleId | null;
  /**
   * `compartilhado` marca o módulo que existe nos dois workspaces e aparece aqui
   * **com filtro de contexto** (§2). Não é cosmético: é o que impede o dado do
   * Profissional de aparecer no Acadêmico com um nome que só o Profissional pode
   * ver.
   */
  readonly contexto: "academico" | "compartilhado";
  readonly estado: EstadoDestino;
  /** O que o módulo vai ser, em uma frase. Alimenta o estado vazio honesto. */
  readonly descricao: string;
  readonly destinos: readonly Destino[];
}

/**
 * A ordem é a da sidebar do §2 (`[P]`), que é a decisão de produto pendente. Esta
 * tabela a **reproduz**; alterar a ordem aqui é ato de produto, não de código.
 */
export const MODULOS: readonly Modulo[] = [
  {
    id: "home",
    titulo: "Home",
    dono: null,
    contexto: "academico",
    estado: "em-construcao",
    descricao:
      "A porta de entrada do Workspace Acadêmico: o que precisa da sua atenção hoje e o Continuar da última atividade.",
    destinos: [],
  },
  {
    id: "calendario",
    titulo: "Calendário",
    dono: "calendario",
    contexto: "compartilhado",
    estado: "pronto",
    descricao:
      "Onde vive tudo que é agendado, a fazer e executado: aulas, prazos, blocos de planejamento e o histórico do que aconteceu.",
    destinos: [
      {
        id: "semana",
        titulo: "Semana",
        estado: "pronto",
        descricao: "A grade da semana com as sete colunas de dia.",
      },
      {
        id: "agenda",
        titulo: "Agenda",
        estado: "pronto",
        descricao: "Os itens do dia em lista.",
      },
      {
        id: "mes",
        titulo: "Mês",
        estado: "pronto",
        descricao: "O mês inteiro, com a marca de cada dia.",
      },
      {
        id: "lista",
        titulo: "Lista",
        estado: "pronto",
        descricao: "As responsabilidades em aberto e o que está atrasado.",
      },
      {
        id: "planejamento",
        titulo: "Planejamento",
        estado: "pronto",
        descricao: "Quanto tempo foi reservado e quanto foi cumprido.",
      },
    ],
  },
  {
    id: "faculdade",
    titulo: "Faculdade",
    dono: "faculdade",
    contexto: "academico",
    estado: "pronto",
    descricao:
      "Semestre, disciplinas, aulas, frequência, avaliações e a regra de média que decide a aprovação.",
    destinos: [],
  },
  {
    id: "estudos",
    titulo: "Estudos",
    dono: "estudos",
    contexto: "compartilhado",
    estado: "pronto",
    descricao: "Conteúdo, sessões de estudo, flashcards e questões, com a fila de revisão do FSRS.",
    destinos: [],
  },
  {
    id: "biblioteca",
    titulo: "Biblioteca",
    dono: "biblioteca",
    contexto: "compartilhado",
    estado: "em-construcao",
    descricao:
      "Itens de biblioteca, Referências e anexos — o acervo que alimenta os outros módulos.",
    destinos: [],
  },
  {
    id: "conhecimento",
    titulo: "Conhecimento",
    dono: "conhecimento",
    contexto: "compartilhado",
    estado: "em-construcao",
    descricao:
      "Fragmentos, Conceitos e Relações — a rede de entendimento construída ao longo do curso.",
    destinos: [],
  },
  {
    id: "tcc",
    titulo: "TCC & Projetos",
    dono: "tcc",
    contexto: "academico",
    estado: "pronto",
    descricao:
      "Projetos, bases de pesquisa, documentos com versões e as saídas — o editor paginado do TCC mora aqui.",
    destinos: [],
  },
  {
    id: "estagio",
    titulo: "Estágio",
    dono: "estagio",
    contexto: "academico",
    estado: "em-construcao",
    descricao:
      "Horas, supervisões e entregas da camada acadêmica; a camada clínica é isolada e mora só no desktop.",
    destinos: [],
  },
  {
    id: "sala-de-treino",
    titulo: "Sala de treino",
    dono: "sala-de-treino",
    contexto: "academico",
    estado: "em-construcao",
    descricao: "Personagens simulados e sessões de treino, com feedback e desempenho.",
    destinos: [],
  },
];

/** Índice por id, para a shell não varrer a lista a cada render. */
export function moduloPorId(id: ModuleId): Modulo {
  const modulo = MODULOS.find((item) => item.id === id);
  if (!modulo) throw new Error(`módulo desconhecido: ${id}`);
  return modulo;
}

/**
 * Os utilitários globais (§5) vivem no rodapé, não na sidebar. A distinção é o
 * que impede a sidebar de crescer sem limite: utilitário não pertence a módulo.
 *
 * O utilitário de IA é `cecinho` (`SPEC-D-014` D94 / `ADR-011`): o rótulo antigo
 * era ambíguo — confundia a usuária com a assistente — e perpetuava a
 * ambiguidade em busca e teste.
 */
export const UTILITARIOS = ["cecinho", "busca", "notificacoes"] as const;
export type UtilitarioId = (typeof UTILITARIOS)[number];

/**
 * Seletor de workspace (§2). O Profissional existe desde o primeiro dia **sem
 * destino** e com estado explícito — seletor que oferece o que não existe é pior
 * que a ausência dele (§7).
 */
export interface Workspace {
  readonly id: "academico" | "profissional";
  readonly titulo: string;
  readonly subtitulo: string;
  readonly estado: EstadoDestino;
}

export const WORKSPACES: readonly Workspace[] = [
  {
    id: "academico",
    titulo: "Acadêmico",
    subtitulo: "Faculdade, estudos e estágio",
    estado: "pronto",
  },
  {
    id: "profissional",
    titulo: "Profissional",
    subtitulo: "Consultório e gestão — em construção",
    estado: "em-construcao",
  },
];

/**
 * D64 — a lista de candidatos que a regra de §1.2 **recusa** como tela, com o
 * módulo que os contém. Existe porque sem lista a recusa vira implícita, e a
 * primeira tela criada por conveniência passa sem ninguém revisar. Cada recusa
 * carrega o motivo; revisar é ato de produto.
 */
export interface CandidatoRecusado {
  readonly titulo: string;
  /** Módulo que **contém** o candidato na tela (`SPEC-D-006` §1.2). */
  readonly contidoEm: ModuleId;
  /** Módulo **dono** do dado, o mesmo de `SPEC-D-007` D65. */
  readonly dono: ModuleId;
  readonly motivo: string;
}

export const CANDIDATOS_RECUSADOS: readonly CandidatoRecusado[] = [
  {
    titulo: "Evento",
    contidoEm: "calendario",
    dono: "calendario",
    motivo: "Não é módulo: aparece dentro das telas que o referenciam (§1.2).",
  },
  {
    titulo: "Responsabilidade",
    contidoEm: "calendario",
    dono: "calendario",
    motivo: "Não é módulo: é o item a fazer dentro do Calendário e da Home (§1.2).",
  },
  {
    titulo: "Regra de recorrência",
    contidoEm: "calendario",
    dono: "calendario",
    motivo: "Não é módulo: é atributo de um evento, não uma atividade da usuária (§1.2).",
  },
  {
    titulo: "Capítulo",
    contidoEm: "biblioteca",
    dono: "biblioteca",
    motivo: "Não é módulo: é estrutura interna de um item de biblioteca (§1.2).",
  },
  {
    titulo: "Flashcard avulso",
    contidoEm: "estudos",
    dono: "estudos",
    motivo: "Não é módulo: é unidade dentro da sessão de estudo (§1.2).",
  },
  {
    titulo: "Avaliação isolada",
    contidoEm: "faculdade",
    dono: "faculdade",
    motivo: "Não é módulo: aparece dentro de Faculdade, Calendário e Home (§1.2).",
  },
  {
    titulo: "Nota isolada",
    contidoEm: "faculdade",
    dono: "faculdade",
    motivo: "Não é módulo: compõe a média do módulo dono (§1.2).",
  },
  {
    titulo: "Conceito isolado",
    contidoEm: "conhecimento",
    dono: "conhecimento",
    motivo: "Não é módulo: compõe a rede de Conhecimento (§1.2).",
  },
];
