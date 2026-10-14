/**
 * Testes de apresentação da tela Faculdade.
 *
 * O que se prova aqui não é cálculo — cálculo é do Rust (D74). Prova-se que o
 * rótulo é **total** (situação desconhecida estoura em vez de renderizar
 * silêncio) e que `null` nunca vira `0,0`: média sem nota é travessão.
 */

import assert from "node:assert/strict";
import { test } from "node:test";

import type { Disciplina, MediaResultado, Semestre } from "../data/bridge.ts";
import {
  formatarMedia,
  semestreAberto,
  situacaoRotulo,
  textoFrequencia,
  textoNotaNecessaria,
  tipoRotulo,
} from "./view.ts";

function media(situacao: MediaResultado["situacao"], nota: number | null): MediaResultado {
  return {
    media: null,
    mediaSemArredondamento: null,
    situacao,
    notaNecessariaProxima: nota,
    frequenciaRestante: null,
    regraAplicada: "padrao",
  };
}

test("formatarMedia: null é travessão, e a vírgula é pt-BR", () => {
  assert.equal(formatarMedia(null), "—");
  assert.equal(formatarMedia(7.25), "7,3");
  assert.equal(formatarMedia(6.0), "6,0");
});

test("situacaoRotulo cobre todas as situações do contrato", () => {
  assert.equal(situacaoRotulo("sem-notas"), "sem notas");
  assert.equal(situacaoRotulo("aprovada"), "aprovada");
  assert.equal(situacaoRotulo("precisa-tirar"), "precisa tirar");
  assert.equal(situacaoRotulo("so-com-exame"), "só com exame");
  assert.equal(situacaoRotulo("nao-alcanca"), "não alcança");
});

test("situacaoRotulo estoura em situação desconhecida em vez de mentir", () => {
  // A string vem do Rust; se um dia as duas pontas discordarem, o erro é aqui —
  // alto — e não um rótulo em branco na tela.
  assert.throws(() => situacaoRotulo("aprovadoo" as MediaResultado["situacao"]));
});

test("textoNotaNecessaria só existe quando a situação pede", () => {
  assert.equal(textoNotaNecessaria(media("precisa-tirar", 8.0)), "precisa 8,0 na próxima");
  assert.equal(textoNotaNecessaria(media("so-com-exame", 7.0)), "precisa 7,0 no exame");
  assert.equal(textoNotaNecessaria(media("aprovada", null)), null);
  assert.equal(textoNotaNecessaria(media("precisa-tirar", null)), null);
});

test("textoFrequencia declina falta no singular e mostra o restante derivado", () => {
  const disciplina = {
    id: "d1",
    semestreId: "s1",
    nome: "Neuropsicologia",
    codigo: null,
    professor: null,
    cargaHoraria: null,
    status: "ativa",
    totalAulas: 40,
    faltas: 1,
  } as Disciplina;
  const comRestante = { ...media("aprovada", null), frequenciaRestante: 9 };
  assert.equal(textoFrequencia(disciplina, comRestante), "1 falta de 40 · pode faltar 9");
  const semRestante = media("aprovada", null);
  assert.equal(
    textoFrequencia({ ...disciplina, faltas: 6 }, semRestante),
    "6 faltas de 40 · pode faltar —",
  );
  assert.equal(textoFrequencia({ ...disciplina, totalAulas: 0 }, semRestante), null);
});

test("semestreAberto: encerradoEm null é aberto (D75)", () => {
  const aberto = {
    id: "s1",
    rotulo: "2026-2",
    startsOn: "2026-08-01",
    endsOn: "2026-12-15",
    encerradoEm: null,
  } as Semestre;
  assert.equal(semestreAberto(aberto), true);
  assert.equal(semestreAberto({ ...aberto, encerradoEm: 1790000000000 }), false);
});

test("tipoRotulo conhece os cinco tipos do Rust e devolve o que não conhece", () => {
  assert.equal(tipoRotulo("apresentacao"), "Apresentação");
  assert.equal(tipoRotulo("prova"), "Prova");
  assert.equal(tipoRotulo("desconhecido"), "desconhecido");
});
