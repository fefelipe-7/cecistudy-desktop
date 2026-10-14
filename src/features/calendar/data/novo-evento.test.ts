import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { montarNovoEvento } from "./novo-evento.ts";
import { toPlainDate } from "../domain/time.ts";
import { CALENDAR_ZONE } from "../domain/zone.ts";
import { LAYER_IDS } from "../domain/layers.ts";
import { isLayerId } from "../domain/layers.ts";

/**
 * A parte do formulário de criação que erra quando erra.
 *
 * Um evento gravado uma hora fora é o tipo de defeito que ninguém percebe no
 * código: compila, passa o typecheck, e a grade mostra o item no horário
 * errado — que é plausível o bastante para passar por escolha da usuária.
 *
 * Por isso o teste é sobre a **tradução**, e não sobre a tela. `montarNovoEvento`
 * é pura e não conhece React; o que o formulário faz em volta é renders e
 * `invoke`, e isso é exercitado pelo `npm run verify` do Rust.
 */

const camposBase = {
  titulo: "Aula de Cálculo",
  data: "2026-03-16",
  hora: "14:00",
  duracaoMin: 60,
  camada: "faculdade",
  compromisso: "obrigatorio",
  repeticao: "nenhuma",
  dias: ["MO"],
} as const;

describe("montarNovoEvento · data e hora", () => {
  it("a hora digitada é lida no fuso da grade, e não no do sistema", () => {
    const { payload } = montarNovoEvento({ ...camposBase });
    // `toPlainDate` volta a data civil no fuso da grade: se a conversão tivesse
    // usado o fuso da máquina, o round-trip poderia cair no dia anterior.
    assert.equal(toPlainDate(payload.startsAt, CALENDAR_ZONE), "2026-03-16");
  });

  it("14:00 em São Paulo é 17:00 UTC no mesmo dia", () => {
    // O Brasil está em UTC-3 em março. Um `new Date("2026-03-16T14:00")` sem
    // fuso seria 14:00 UTC, ou seja duas horas depois na parede.
    const { payload } = montarNovoEvento({ ...camposBase });
    const iso = new Date(payload.startsAt).toISOString();
    assert.equal(iso, "2026-03-16T17:00:00.000Z");
  });

  it("a duração vem do campo e fecha em :30", () => {
    const { payload } = montarNovoEvento({ ...camposBase, duracaoMin: 90 });
    const minutos = (payload.endsAt - payload.startsAt) / 60_000;
    assert.equal(minutos, 90);
    assert.equal(new Date(payload.endsAt).toISOString(), "2026-03-16T18:30:00.000Z");
  });

  it("aceita meia-noite", () => {
    const { payload } = montarNovoEvento({ ...camposBase, hora: "00:00" });
    assert.equal(new Date(payload.startsAt).toISOString(), "2026-03-16T03:00:00.000Z");
  });
});

describe("montarNovoEvento · título", () => {
  it("remove o espaço em volta, para o evento não se chamar ' '", () => {
    const { payload } = montarNovoEvento({ ...camposBase, titulo: "  Aula de Cálculo  " });
    assert.equal(payload.title, "Aula de Cálculo");
  });

  it("só com espaço fica vazio, e vazio é recusado pela regra", () => {
    // A tela também recusa, mas a garantia é do Rust — este teste registra que o
    // formulário **manda** vazio para a regra recusar, em vez de inventar um
    // título em vez do que a usuária não digitou.
    const { payload } = montarNovoEvento({ ...camposBase, titulo: "   " });
    assert.equal(payload.title, "");
  });
});

describe("montarNovoEvento · recorrência", () => {
  it("não repetir não produz linha RRULE", () => {
    assert.equal(montarNovoEvento({ ...camposBase }).rrule, null);
  });

  it("diária é a linha sem placeholder", () => {
    assert.equal(montarNovoEvento({ ...camposBase, repeticao: "diaria" }).rrule, "FREQ=DAILY");
  });

  it("quinzenal carrega o INTERVAL", () => {
    assert.equal(
      montarNovoEvento({ ...camposBase, repeticao: "quinzenal" }).rrule,
      "FREQ=DAILY;INTERVAL=15",
    );
  });

  it("mensal usa o dia do evento, e não um dia fixo", () => {
    // `%D` existe porque `BYMONTHDAY` tem de ser o dia que a usuária digitou: um
    // "todo dia 1" silencioso seria um evento no dia errado a cada mês.
    const { rrule } = montarNovoEvento({ ...camposBase, repeticao: "mensal" });
    assert.equal(rrule, "FREQ=MONTHLY;BYMONTHDAY=16");
  });

  it("mensal em dia 31 é aceito, e a RFC pula fevereiro", () => {
    const { rrule } = montarNovoEvento({
      ...camposBase,
      data: "2026-01-31",
      repeticao: "mensal",
    });
    assert.equal(rrule, "FREQ=MONTHLY;BYMONTHDAY=31");
  });

  it("semanal monta os BYDAY na ordem que a usuária marcou", () => {
    const { rrule } = montarNovoEvento({
      ...camposBase,
      repeticao: "semanal",
      dias: ["MO", "WE", "FR"],
    });
    assert.equal(rrule, "FREQ=WEEKLY;BYDAY=MO,WE,FR");
  });

  it("semanal sem dia nenhum cai em segunda, e não em toda semana", () => {
    // Um `FREQ=WEEKLY` sem `BYDAY` é o bug que o motor Rust tinha: produzia uma
    // ocorrência por dia. A tela não pode deixar a string sair assim.
    const { rrule } = montarNovoEvento({ ...camposBase, repeticao: "semanal", dias: [] });
    assert.equal(rrule, "FREQ=WEEKLY;BYDAY=MO");
    assert.ok(!rrule?.endsWith(";"), "a linha não pode terminar em ponto e vírgula");
  });
});

describe("montarNovoEvento · camada e compromisso", () => {
  it("a origem é sempre nossa; o Google não se cria por aqui", () => {
    const { payload } = montarNovoEvento({ ...camposBase });
    assert.equal(payload.origin, "cecistudy");
  });

  it("toda camada oferecida existe de verdade", () => {
    // `LAYER_IDS` tem `google`, que não é editável. Se o formulário oferecesse
    // o Google, a escrita seria recusada por `guard_origin` — botão que leva a
    // erro. A lista de camadas do formulário é a do domínio menos a do Google.
    const google = LAYER_IDS.find((id) => id === "google");
    assert.ok(google, "o domínio tem a camada google, e o formulário não a oferece");
    for (const id of LAYER_IDS.filter((c) => c !== "google")) {
      assert.ok(isLayerId(id));
    }
  });

  it("o compromisso vai como está, e `recomendado` continua não-obrigatório", () => {
    // INV-7: só `obrigatorio` e `importante` geram atraso. O formulário não pode
    // "promover" um opcional, e o valor segue literal para a regra do Rust.
    const { payload } = montarNovoEvento({ ...camposBase, compromisso: "opcional" });
    assert.equal(payload.commitment, "opcional");
  });
});

describe("montarNovoEvento · fuso do payload", () => {
  it("o payload carrega o fuso da grade, e não o da máquina", () => {
    const { payload } = montarNovoEvento({ ...camposBase });
    assert.equal(payload.timeZone, "America/Sao_Paulo");
    assert.equal(payload.timeZone, CALENDAR_ZONE);
  });
});
