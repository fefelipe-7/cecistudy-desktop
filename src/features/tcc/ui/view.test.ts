import assert from "node:assert/strict";
import test from "node:test";

import type { ItemAResolver, NoDocumento, Projeto } from "../data/bridge.ts";
import {
  ESTADOS_DE_PROJETO,
  ESTADOS_ITEM,
  LIMITE_DE_PROJETOS_ATIVOS,
  TIPOS_DE_PROJETO,
  TIPOS_DE_ALVO,
  TIPOS_ITEM,
  assinaturaDoItem,
  contarArvore,
  itemPendente,
  podeCriarProjeto,
  projetosAtivos,
  rotuloAlvoTipo,
  rotuloEstadoItem,
  rotuloEstadoProjeto,
  rotuloTipoItem,
  rotuloTipoProjeto,
  rotuloVersao,
} from "./view.ts";

function item(overrides: Partial<ItemAResolver>): ItemAResolver {
  return {
    id: "i1",
    documentoId: "d1",
    tipo: "comentario",
    autor: "Orientador",
    autorResolvido: true,
    dataAutor: null,
    citado: "trecho",
    substituto: null,
    texto: null,
    origemArquivo: null,
    importadoEm: 1_000,
    estado: "pendente",
    resolvidoEm: null,
    ...overrides,
  };
}

test("os sete tipos de projeto de §4.7 estão na lista, na ordem da spec", () => {
  assert.deepEqual(
    TIPOS_DE_PROJETO.map((t) => t.id),
    [
      "tcc",
      "artigo",
      "iniciacao-cientifica",
      "relatorio-de-estagio",
      "projeto-de-pesquisa",
      "revisao-bibliografica",
      "estudo-de-caso",
    ],
  );
});

test("a lista de estados do projeto é fechada em três", () => {
  assert.deepEqual(
    ESTADOS_DE_PROJETO.map((e) => e.id),
    ["ativo", "encerrado", "arquivado"],
  );
});

test("os quatro estados do item a resolver existem", () => {
  assert.deepEqual(
    ESTADOS_ITEM.map((e) => e.id),
    ["pendente", "resolvido", "nao-encaixou", "duplicado"],
  );
  assert.deepEqual(
    TIPOS_ITEM.map((t) => t.id),
    ["comentario", "sugestao"],
  );
});

test("rótulo conhecido é traduzido; desconhecido volta cru, nunca em branco", () => {
  assert.equal(rotuloTipoProjeto("tcc"), "TCC");
  assert.equal(rotuloTipoProjeto("relatorio-de-estagio"), "Relatório de estágio");
  assert.equal(rotuloTipoProjeto("mestrado"), "mestrado");

  assert.equal(rotuloEstadoProjeto("encerrado"), "Encerrado");
  assert.equal(rotuloTipoItem("sugestao"), "Sugestão");
  assert.equal(rotuloEstadoItem("nao-encaixou"), "Não encaixou");
});

test("os dois alvos da Base de pesquisa são fechados e rotulados", () => {
  assert.deepEqual(
    TIPOS_DE_ALVO.map((t) => t.id),
    ["referencia", "fragmento"],
  );
  assert.equal(rotuloAlvoTipo("referencia"), "Referência");
  assert.equal(rotuloAlvoTipo("fragmento"), "Fragmento");
  assert.equal(rotuloAlvoTipo("unitario" as never), "unitario");
});

test("itemPendente é verdadeiro só no estado pendente", () => {
  assert.equal(itemPendente(item({ estado: "pendente" })), true);
  for (const estado of ["resolvido", "nao-encaixou", "duplicado"] as const) {
    assert.equal(itemPendente(item({ estado })), false);
  }
});

test("assinaturaDoItem diz o autor desconhecido, nunca esconde (D78)", () => {
  assert.equal(assinaturaDoItem(item({ autor: "Orientador" })), "Orientador");
  assert.equal(
    assinaturaDoItem(item({ autor: "Orientador", autorResolvido: false })),
    "Orientador · não identificado",
  );
  assert.equal(
    assinaturaDoItem(item({ autor: "  ", autorResolvido: false })),
    "autor não identificado",
  );
});

test("contarArvore soma os nós, incluindo os aninhados no capítulo", () => {
  const arvore: NoDocumento[] = [
    {
      tipo: "capitulo",
      id: "c1",
      titulo: "Introdução",
      filhos: [
        { tipo: "paragrafo", id: "p1", inline: [{ texto: "oi", marcas: [] }] },
        { tipo: "citacao", id: "c", referenciaId: "ref-1", modo: "vivo" },
      ],
    },
    { tipo: "referencia-viva", id: "r", fragmentoId: "frag-1" },
  ];

  assert.deepEqual(contarArvore(arvore), {
    secoes: 1,
    paragrafos: 1,
    citacoes: 1,
    referencias: 1,
    total: 4,
  });
  assert.deepEqual(contarArvore([]), {
    secoes: 0,
    paragrafos: 0,
    citacoes: 0,
    referencias: 0,
    total: 0,
  });
});

test("contarArvore soma capítulos aninhados", () => {
  const arvore: NoDocumento[] = [
    {
      tipo: "capitulo",
      id: "c1",
      titulo: "Um",
      filhos: [{ tipo: "capitulo", id: "c2", titulo: "Dois", filhos: [] }],
    },
  ];
  assert.equal(contarArvore(arvore).secoes, 2);
  assert.equal(contarArvore(arvore).total, 2);
});

test("rotuloVersao distingue rascunho de versão gravada", () => {
  assert.equal(rotuloVersao(null), "rascunho (sem versão)");
  assert.equal(rotuloVersao(1), "versão 1");
  assert.equal(rotuloVersao(7), "versão 7");
});

function projeto(estado: string): Projeto {
  return {
    id: `p-${estado}-${Math.random()}`,
    titulo: "Projeto",
    tipo: "tcc",
    estado,
    descricao: null,
    createdAt: 0,
    updatedAt: 0,
  };
}

test("o limite de projetos ativos conta só os ativos (§4.7)", () => {
  const lista: Projeto[] = [
    ...Array.from({ length: LIMITE_DE_PROJETOS_ATIVOS }, () => projeto("ativo")),
    projeto("encerrado"),
    projeto("arquivado"),
  ];
  assert.equal(projetosAtivos(lista), LIMITE_DE_PROJETOS_ATIVOS);
  assert.equal(podeCriarProjeto(lista), false, "cinco ativos: a tela avisa que não cabe um sexto");

  const comLugar = [...lista.slice(0, LIMITE_DE_PROJETOS_ATIVOS - 1), projeto("encerrado")];
  assert.equal(podeCriarProjeto(comLugar), true, "encerrar libera o lugar");
});
