import assert from "node:assert/strict";
import test from "node:test";

import type { NoDocumento } from "../data/bridge.ts";
import {
  adicionarFilho,
  atualizarTexto,
  atualizarTitulo,
  capituloVazio,
  inserirIrmao,
  mover,
  novoId,
  paragrafoVazio,
  remover,
  textoDoNo,
} from "./editar.ts";

function capitulo(id: string, titulo: string, filhos: NoDocumento[] = []): NoDocumento {
  return { tipo: "capitulo", id, titulo, filhos };
}

function paragrafo(id: string, texto: string): NoDocumento {
  return { tipo: "paragrafo", id, inline: [{ texto, marcas: [] }] };
}

/** Uma árvore de referência: dois capítulos no topo, um com filhos. */
function arvoreDeReferencia(): NoDocumento[] {
  return [
    capitulo("c1", "Introdução", [paragrafo("p1", "Primeiro"), paragrafo("p2", "Segundo")]),
    capitulo("c2", "Método", []),
  ];
}

test("novoId gera dois ids diferentes e não vazios", () => {
  const a = novoId();
  const b = novoId();
  assert.notEqual(a, b);
  assert.ok(a.length > 0);
  assert.ok(b.length > 0);
});

test("capituloVazio e paragrafoVazio nascem com id próprio e conteúdo neutro", () => {
  const cap = capituloVazio();
  assert.equal(cap.tipo, "capitulo");
  assert.deepEqual(cap.tipo === "capitulo" ? cap.filhos : null, []);
  assert.notEqual(cap.id, capituloVazio().id);

  const par = paragrafoVazio("oi");
  assert.equal(par.tipo, "paragrafo");
  assert.equal(textoDoNo(par), "oi");
  assert.deepEqual(par.tipo === "paragrafo" ? par.inline : null, [{ texto: "oi", marcas: [] }]);
});

test("textoDoNo concatena os trechos e devolve vazio fora do parágrafo", () => {
  const composto: NoDocumento = {
    tipo: "paragrafo",
    id: "p",
    inline: [
      { texto: "Bom ", marcas: ["negrito"] },
      { texto: "dia", marcas: [] },
    ],
  };
  assert.equal(textoDoNo(composto), "Bom dia");
  assert.equal(textoDoNo(capitulo("c", "Título")), "");
});

test("atualizarTitulo troca só o capítulo com o id, em qualquer profundidade", () => {
  const arvore = arvoreDeReferencia();
  const nova = atualizarTitulo(arvore, "c2", "Metodologia");
  assert.equal((nova[1] as NoDocumento & { titulo: string }).titulo, "Metodologia");
  assert.equal((nova[0] as NoDocumento & { titulo: string }).titulo, "Introdução");

  // a árvore antiga não mutou (é imutável)
  assert.equal((arvore[1] as NoDocumento & { titulo: string }).titulo, "Método");

  // id inexistente devolve a árvore igual
  assert.deepEqual(atualizarTitulo(arvore, "nao-existe", "X"), arvore);
});

test("atualizarTexto troca o parágrafo por um único trecho sem marcas (v1)", () => {
  const arvore = arvoreDeReferencia();
  const nova = atualizarTexto(arvore, "p1", "Parágrafo reescrito");
  const raiz = nova[0];
  assert.equal(raiz?.tipo, "capitulo");
  const filho = raiz?.tipo === "capitulo" ? raiz.filhos[0] : undefined;
  assert.equal(filho ? textoDoNo(filho) : null, "Parágrafo reescrito");
  assert.deepEqual(filho?.tipo === "paragrafo" ? filho.inline : null, [
    { texto: "Parágrafo reescrito", marcas: [] },
  ]);
});

test("atualizarTexto descarta marcas inline antigas, de forma declarada", () => {
  const comMarca: NoDocumento[] = [
    {
      tipo: "paragrafo",
      id: "p",
      inline: [{ texto: "importante", marcas: ["negrito", "italico"] }],
    },
  ];
  const nova = atualizarTexto(comMarca, "p", "importante");
  const no = nova[0];
  assert.deepEqual(no?.tipo === "paragrafo" ? no.inline : null, [
    { texto: "importante", marcas: [] },
  ]);
});

test("remover tira o nó com o id, inclusive aninhado", () => {
  const arvore = arvoreDeReferencia();
  const semP1 = remover(arvore, "p1");
  const cap1 = semP1[0];
  assert.equal(cap1?.tipo === "capitulo" ? cap1.filhos.length : -1, 1);
  assert.equal(cap1?.tipo === "capitulo" ? cap1.filhos[0]?.id : null, "p2");

  const semC2 = remover(arvore, "c2");
  assert.equal(semC2.length, 1);
});

test("mover troca irmãos de lugar em qualquer profundidade", () => {
  const arvore = arvoreDeReferencia();

  const cap2Primeiro = mover(arvore, "c2", "cima");
  assert.deepEqual(
    cap2Primeiro.map((n) => n.id),
    ["c2", "c1"],
  );

  const p2Primeiro = mover(arvore, "p2", "cima");
  const cap = p2Primeiro[0];
  assert.deepEqual(cap?.tipo === "capitulo" ? cap.filhos.map((n) => n.id) : null, ["p2", "p1"]);
});

test("mover na ponta não muda nada, e o primeiro não sobe", () => {
  const arvore = arvoreDeReferencia();
  const primeiroParaCima = mover(arvore, "c1", "cima");
  assert.deepEqual(
    primeiroParaCima.map((n) => n.id),
    ["c1", "c2"],
  );

  const ultimoParaBaixo = mover(arvore, "c2", "baixo");
  assert.deepEqual(
    ultimoParaBaixo.map((n) => n.id),
    ["c1", "c2"],
  );
});

test("inserirIrmao põe o nó logo depois do referenciado, no mesmo nível", () => {
  const arvore = arvoreDeReferencia();
  const novo = paragrafoVazio("No meio");
  const nova = inserirIrmao(arvore, "p1", novo);
  const cap = nova[0];
  assert.deepEqual(cap?.tipo === "capitulo" ? cap.filhos.map((n) => n.id) : null, [
    "p1",
    novo.id,
    "p2",
  ]);
});

test("adicionarFilho anexa ao capítulo, sem mexer nos outros", () => {
  const arvore = arvoreDeReferencia();
  const novo = paragrafoVazio("Aberto");
  const nova = adicionarFilho(arvore, "c2", novo);
  const c2 = nova[1];
  assert.equal(c2?.tipo === "capitulo" ? c2.filhos.length : -1, 1);
  assert.equal(c2?.tipo === "capitulo" ? c2.filhos[0]?.id : null, novo.id);
  assert.equal(
    nova[0]?.tipo === "capitulo" ? nova[0].filhos.length : -1,
    2,
    "o outro capítulo não muda",
  );
});
