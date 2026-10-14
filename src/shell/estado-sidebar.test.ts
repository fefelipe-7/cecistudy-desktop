import assert from "node:assert/strict";
import { describe, it } from "node:test";

import {
  LARGURA_ABERTA,
  LARGURA_COMPACTA,
  alternarSidebar,
  estadoAoMudarViewport,
  larguraDaBarra,
  type EstadoSidebar,
} from "./estado-sidebar.ts";

const TODOS: EstadoSidebar[] = ["aberta", "compacta", "fechada"];

describe("larguraDaBarra", () => {
  it("aberta mede a largura cheia e compacta mede a do ícone", () => {
    assert.equal(larguraDaBarra("aberta"), LARGURA_ABERTA);
    assert.equal(larguraDaBarra("compacta"), LARGURA_COMPACTA);
  });

  it("compacta é mais estreita que aberta e ainda cabe um alvo de clique", () => {
    assert.ok(LARGURA_COMPACTA < LARGURA_ABERTA);
    // Abaixo de ~24px o ícone deixa de ser clicável com o mouse, que é o
    // dispositivo-alvo do desktop. 68px sobra e é o compasso medido em
    // `docs/performance/02-transicoes-animacoes.md`.
    assert.ok(LARGURA_COMPACTA >= 24);
  });

  it("fechada é 0 no desktop porque o desktop não tem estado fechado", () => {
    // O valor é o do drawer do mobile. Ver `alternarSidebar`: nenhuma transição
    // do desktop chega aqui.
    assert.equal(larguraDaBarra("fechada"), 0);
  });
});

describe("alternarSidebar", () => {
  it("no desktop nunca chega em fechada, e a partir de qualquer estado converge", () => {
    for (const inicial of TODOS) {
      let atual = inicial;
      // Quatro voltas a partir de `fechada` precisam ser o bastante para cair no
      // ciclo de dois, e é isso que o teste mede: a regra não pode produzir
      // "fechada" **e** não pode ficar nela. Um dead-end aqui é a pessoa sem
      // barra e sem botão para recuperar.
      for (let i = 0; i < 4; i++) {
        atual = alternarSidebar(atual, false);
        assert.notEqual(atual, "fechada", `desktop chegou em fechada a partir de ${inicial}`);
      }
      assert.ok(atual === "aberta" || atual === "compacta", `dead-end ${atual} de ${inicial}`);
      // E o ciclo é de período 2: mais duas voltas não mudam nada.
      assert.equal(alternarSidebar(alternarSidebar(atual, false), false), atual);
    }
  });

  it("no desktop compacta continua navigável: a largura não é zero", () => {
    assert.ok(larguraDaBarra(alternarSidebar("aberta", false)) > 0);
    assert.ok(larguraDaBarra(alternarSidebar("compacta", false)) > 0);
  });

  it("no mobile nunca produz compacta, e a partir de qualquer estado converge", () => {
    for (const inicial of TODOS) {
      let atual = inicial;
      for (let i = 0; i < 4; i++) {
        atual = alternarSidebar(atual, true);
        assert.notEqual(atual, "compacta", `mobile chegou em compacta a partir de ${inicial}`);
      }
      assert.ok(atual === "aberta" || atual === "fechada", `dead-end ${atual} de ${inicial}`);
      assert.equal(alternarSidebar(alternarSidebar(atual, true), true), atual);
    }
  });

  it("aberta no mobile fecha, e fechada no mobile abre", () => {
    assert.equal(alternarSidebar("aberta", true), "fechada");
    assert.equal(alternarSidebar("fechada", true), "aberta");
  });

  it("compacta no desktop abre: é a recuperação de quem apertou o botão", () => {
    assert.equal(alternarSidebar("compacta", false), "aberta");
  });
});

describe("estadoAoMudarViewport", () => {
  it("entrar no mobile fecha a gaveta, em qualquer estado", () => {
    for (const inicial of TODOS) {
      assert.equal(estadoAoMudarViewport(inicial, true), "fechada");
    }
  });

  it("voltar ao desktop abre, porque fechada não tem desenho lá", () => {
    assert.equal(estadoAoMudarViewport("fechada", false), "aberta");
  });

  it("voltar ao desktop preserva compacta: quem recolheu continua recolhido", () => {
    assert.equal(estadoAoMudarViewport("compacta", false), "compacta");
    assert.equal(estadoAoMudarViewport("aberta", false), "aberta");
  });

  it("cruzar o breakpoint não deixa o estado com a largura do viewport errado", () => {
    // A invariante que a barra inteira depende: depois de reconciliar, a
    // largura corresponde ao desenho do viewport, e não ao do estado anterior.
    const estados = TODOS.map((inicial) => estadoAoMudarViewport(inicial, true));
    for (const estado of estados) {
      assert.ok(larguraDaBarra(estado) <= LARGURA_ABERTA);
    }
  });
});
