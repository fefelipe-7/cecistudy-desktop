# Spec 05 — Integração bidirecional com Google Calendar

Data: 2026-09-28
Escopo: conectar uma conta Google a um calendário dedicado, sincronizar nos dois sentidos com a política de permissão de `calendario.md` §10, resolver conflito sem sobrescrita silenciosa, tratar exclusão nos dois lados e exibir o estado de sincronização.
Status: proposta fechada, **bloqueada por credenciais** até que exista um client OAuth (ver §4).

**Rastreabilidade — `calendario.md`:** §5 (a ocorrência remarcada precisa permitir "sincronização correta com o Google Calendar"), §7 (a camada `google` dentro da mesma tela, com cor própria), §10 (integral: permissões, identidade dos dois lados, conflito, exclusão), §11 §146 (a sync "deve entrar cedo, mas não deve impedir a evolução do calendário interno").
**Entregas de `calendario.md` §11 cobertas:** **8** (integração bidirecional com calendário dedicado do Google) e **9** (conflitos, exclusões e indicador de sincronização).
**Depende de:** spec 00 (A2, A5, A6, A8, D3, D5, D6) e spec 01 (domínio, `external_link`, `sync_cursor`, `google_connection`).
**Deixa deliberadamente para:**

- spec 06 — a sugestão de bloco pode, no futuro, sugerir um item no Google; o contrato de sugestão é da spec 06, e o `events.insert` do Google não é usado por ela;
- specs 02 e 03 — a interface (a aba de sincronização, o indicador, as ações de conflito e exclusão) é delas; aqui ficam definidos o estado, o contrato de comando e a política, e a spec 02/03 só renderiza;
- spec 04 — Google **não** é um módulo de origem e não registra `OriginAdapter` (D31 da spec 04). Esta spec cria o seu próprio adaptador de sync, com regras diferentes.

---

## 1. Diagnóstico do estado atual

| Requisito                                               | Onde encosta hoje                                            | Situação                                                                                   |
| ------------------------------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------ |
| §10 conexão de conta                                    | —                                                            | Ausente — `@tauri-apps/api` nem instalado até a spec 00                                    |
| §10 calendário dedicado                                 | —                                                            | Ausente                                                                                    |
| §10(item criado no cecistudy)                           | —                                                            | Ausente                                                                                    |
| §10(item criado no Google, importado)                   | —                                                            | Ausente                                                                                    |
| §10(responsabilidade ligada a evento externo)           | —                                                            | Ausente                                                                                    |
| §10 identidade dos dois lados, etag, estado de conflito | —                                                            | Ausente — o schema `external_link` é criado na spec 01, vazio                              |
| §10 conflito sem sobrescrita silenciosa                 | —                                                            | Ausente                                                                                    |
| §10 exclusão nos dois lados                             | —                                                            | Ausente — não há `AlertDialog` em uso (`SPEC.md:325` removeu o set; a spec 02 o reinstala) |
| §10(camada `google` na tela)                            | `src/App.tsx:709-758` — 6 itens, todos com cor de disciplina | Ausente como camada                                                                        |
| §5 remarcação sincronizável                             | —                                                            | Ausente                                                                                    |
| §11 sincronização manual e indicador de estado          | —                                                            | Ausente                                                                                    |

Fatos verificados que condicionam o desenho:

- `"withGlobalTauri": false` (`src-tauri/tauri.conf.json:13`) e `@tauri-apps/api` ausente de `package.json:23-43` → a spec 00 Fase 0.2 instala o pacote e a spec 00 Fase 0.3 cria `src/lib/ipc.ts`.
- A CSP de produção é `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost` (`src-tauri/tauri.conf.json:31`) — **sem** `https://www.googleapis.com` e **sem** `https://oauth2.googleapis.com`. `devCsp` é `null` (`:32`). Decisão A6 da spec 00 já resolveu: **toda** chamada sai do Rust e a CSP não é tocada.
- `reqwest` está em `Cargo.lock:2512` como transitivo do Tauri; `src-tauri/Cargo.toml:20-25` ainda não o declara.
- `capabilities/default.json:8-10` só concede `core:default` — comandos `#[tauri::command]` próprios não exigem permissão adicional no Tauri v2, mas **abrir uma janela de navegador do sistema** e **escrever no app data dir** exigem cuidado de configuração.

---

## 2. Decisões fechadas

### 2.1 Herdadas

A2 (SQLite em Rust) · A5 (`invoke`) · A6 (**todo** o HTTP sai do Rust; CSP intacta; token nunca chega ao webview) · A8 (INV-5, INV-6) · D3 (allowlist por campo) · D5 (conexão única, singleton) · D6 (UTC + IANA).

### 2.2 Novas

**D32 — OAuth 2.0 "Desktop app" com _loopback_ em porta dinâmica, PKCE, `access_type=offline`, feito em Rust.**
Decisão: Rust sobe um listener HTTP efêmero em `127.0.0.1:0` (porta escolhida pelo SO), monta a URL de autorização com `code_challenge` PKCE e `access_type=offline&prompt=consent`, abre o navegador do sistema, espera o callback, valida `state`, troca o código por `access_token` + `refresh_token` e **fecha o listener**. Scopes: `https://www.googleapis.com/auth/calendar.events` e `https://www.googleapis.com/auth/calendar.readonly` (o segundo não é pedido no mesmo fluxo; a lista de calendários usa `calendarList.list`, que exige `calendar.readonly`). O **primeiro** item de `calendarList.list` que possa ser escrito é sugerido como destino, mas a usuária **escolhe** (§10 fala em "calendário dedicado").
Justificativa: o _loopback_ com porta dinâmica continua suportado pelo Google **para clientes do tipo Desktop app** — a depreciação de 2022 vale para iOS, Android e Chrome app, não para desktop. Isso é melhor que esquema URI customizado no Windows, que exige registro no SO. PKCE + `state` fecham o fluxo; `prompt=consent` garante que o `refresh_token` venha na primeira vez — sem isso o Google só o emite uma vez, e a spec perderia a sync automática.
Alternativa rejeitada: `tauri-plugin-oauth` de terceiro (dependência externa com manutenção fora do repo, para um listener de 60 linhas); esquema URI customizado (registro de protocolo no SO por plataforma, e a depreciação do loopback _não_ se aplica ao desktop, ou seja, trocar não compra nada).
Consequência: `src-tauri/src/oauth.rs` com `begin_authorization` / `complete_authorization` / `refresh_access_token` / `revoke`. O `client_id` e o `client_secret` vêm de `setting` (ou de `.env` em dev) e **nunca** são embutidos no bundle. O `refresh_token` fica em `google_connection` e **só** o Rust o lê.

**D33 — Sincronização é incremental por `syncToken`, com _full sync_ de recuperação; não há push.**
Decisão: o primeiro sync faz `events.list(singleEvents=true, timeMin=…, timeMax=…, showDeleted=true)` e guarda o `nextSyncToken` em `sync_cursor`. Depois, só `events.list(syncToken=…)`. Se a API responder **410 GONE**, o `syncToken` é descartado e um _full sync_ roda de novo. O sync roda (a) ao abrir a janela, (b) quando o app ganha foco, (c) no botão "Sincronizar agora", (d) a cada 15 min enquanto a janela estiver visível. **Não** se registra `events.watch`.
Justificativa: `events.watch` exige um endpoint HTTPS **publicamente alcançável** para o Google entregar a notificação — um app desktop offline não tem isso, e apontar para um servidor terceiro seria enviar o conteúdo do calendário dela para fora. Poll por `syncToken` é a via correta para desktop, e o `syncToken` é a otimização que o próprio modelo do Google oferece (inclusive sem `singleEvents`, ele é quem gerencia a expansão de recorrência). `§11:146` pede sync manual e indicador; a periodicidade vem junto, sem custo.
Alternativa rejeitada: `events.watch` contra um relay próprio (exige servidor, conta o calendário de uma usuária e cria um canal de dados fora do produto); sincronização completa a cada 15 min (ignora o `syncToken` que a API dá de graça e é a resposta certa a 410).
Consequência: `sync_cursor(system, calendar_id, sync_token, last_full_sync_at)` ganha uso; a rotina de poll é um `tokio::spawn` no Rust, com `CancellationToken` no shutdown.

**D34 — Identidade dos dois lados: `extendedProperties.private` com três chaves, nunca pelo `id` cru.**
Decisão: todo evento criado no cecistudy grava, no `extendedProperties.private`:
`campus_entity` = `calendar_event:<uuid>` · `campus_layer` = `faculdade` · `campus_schema` = `1`.
Na leitura, um evento é "nosso" se tiver `campus_entity`; a afiliação é por `iCalUID` + `recurringEventId` + `originalStartTime`, nunca pelo `id` opaco do Google (que muda ao mover entre calendários).
Justificativa: §10 exige "identidade dos dois lados". O `id` do Google é opaco, pode mudar e não sobrevive a uma cópia; `extendedProperties.private` foi feito exatamente para isto — metadados da aplicação, privados à cópia local do evento, e consultáveis com `privateExtendedProperty` (o que permite um _full sync_ só dos nossos). `iCalUID` + `recurringEventId` + `originalStartTime` é a chave de instância que o Google usa, e é o que §5 pede para a remarcação.
Alternativa rejeitada: prefixar o título (`"[Campus] "`), que é a receita ingênua e contamina o calendário dela.
Consequência: o esquema versionado (`campus_schema = 1`) permite migrar a representation sem quebrar o vínculo.

**D35 — Tabela de permissão por campo, aplicada em Rust, e nenhum payload com campo fora dela.**
Decisão: `SYNCABLE_FIELDS` é uma lista fechada em Rust. `events.insert` / `events.patch` são montados **campo a campo** a partir dessa lista; um valor que não está na lista simplesmente não entra no corpo. A tabela completa:

| Item no cecistudy                            | Campo no Google                                       |                  `summary`                  | `description` | `start`/`end` | `location` | `status` | `recurrence` | `extendedProperties.private` |
| -------------------------------------------- | ----------------------------------------------------- | :-----------------------------------------: | :-----------: | :-----------: | :--------: | :------: | :----------: | :--------------------------: |
| **Criado no cecistudy**, vinculado           | `calendar_event`                                      |                      ⇅                      |       ⇅       |       ⇅       |     ⇅      |    ⇅¹    |      ⇅       |              ⇅               |
| **Criado no cecistudy**, sem vínculo         | —                                                     |                      —                      |       —       |       —       |     —      |    —     |      —       |              —               |
| **Criado no Google**, importado              | `calendar_event` `origin="google"`                    |                      ⊘                      |       ⊘       |       ⊘       |     ⊘      |    ⊘     |      ⊘       |              ⊘               |
| **Responsabilidade ligada a evento externo** | `responsibility` + `calendar_event` `origin="google"` | ⊘ (só o do cecistudy, no item do cecistudy) |       ⊘       |       ⊘       |     ⊘      |    ⊘     |      ⊘       |              ⊘               |

¹ `status` só é escrito como `cancelled` quando a ocorrência foi cancelada **no cecistudy**; um `confirmed` vindo do Google nunca é reescrito.

Justificativa: D3 já decidiu a allowlist; aqui está a tabela completa, e ela é **executada em Rust** porque é o único lugar que constrói o corpo HTTP. `calendario.md` §10 é uma tabela literal de permissões; implementá-la como um `match` no binding torna impossível escrever um campo proibido por engano.
Alternativa rejeitada: espelhar o objeto inteiro com `If-Match` e revisar o diff depois (a §10 linha 3 proíbe).
Consequência: `rg "SYNCABLE_FIELDS" src-tauri/src/google` existe e é a **única** fonte de chaves de corpo; nenhum `json!({ "summary": ... })` avulso no crate `google`.

**D36 — Remarcação vira `events.patch` na instância, com `extendedProperties.private.campus_original_start`; cancelamento de ocorrência vira `status: "cancelled"` só naquela instância.**
Decisão: §5 diz que a ocorrência remarcada "mantém referência à regra e à data original". No Google, isso é um patch da **instância** (identificada por `recurringEventId` + `originalStartTime`), não da série. Ao remarcar, o cecistudy grava `campus_original_start` no `private` do evento remoto, para poder casar de volta na próxima leitura.
Justificativa: é exatamente a semântica de §4 ("a alteração deve ocorrer na ocorrência específica por padrão; a regra recorrente permanece intacta"), e é o único jeito de o Google devolver a mesma instância em vez de criar uma série nova.
Alternativa rejeitada: `events.move` ou recriar a série (quebra a regra e o histórico — INV-5).
Consequência: `move_occurrence` da spec 01 vira, quando há vínculo, um comando composto local + `patch` remoto, na ordem **local primeiro** (§10 e D-seguranca: se o remoto falhar, o item local tem `sync_state = "pendente"` e a interface mostra "Sincronizar agora", nunca o contrário).

**D37 — Conflito: as duas versões são preservadas e a usuária escolhe; "combinar" só onde é seguro.**
Decisão: quando um `PATCH` com `If-Match: <etag>` volta **412**, o cecistudy **não** re-tenta. Ele grava `conflict_local_json` e `conflict_remote_json` na linha de `external_link`, muda `sync_state` para `"conflito"`, e emite o item para a fila de conflitos. A tela de conflito oferece três ações: **Manter o do cecistudy** (`PATCH` com o etag novo, sem `If-Match`), **Manter o do Google** (`PATCH` local aplicando o remoto, registrando a última origem como `google`), e **Combinar campos** — oferecida **apenas** para os campos de `SYNCABLE_FIELDS` cujo valor remoto **não** mudou desde o último sync conhecido; para qualquer outro, as duas primeiras são as únicas opções.
Justificativa: §10 é literal: "Conflitos não serão sobrescritos silenciosamente. O sistema deve preservar temporariamente as versões e permitir manter a versão do cecistudy, manter a do Google ou combinar campos **quando a combinação for segura**." A palavra "segura" é o que D35 restringe: a combinação só é segura onde nenhum lado mexeu desde a base comum.
Alternativa rejeitada: última-escrita-vence (sobrescrita silenciosa, proibida); sempre oferecer "combinar" (perde dados silenciosamente, que é a mesma coisa com outro nome).
Consequência: a fila de conflitos é uma lista; um conflito **não bloqueia** a agenda, o painel ou a edição de outros itens.

**D38 — Exclusão nos dois lados é sempre confirmada, e exclusão remota não apaga histórico.**
Decisão: excluir um item vinculado abre confirmação que nomeia **os dois** sistemas e exige confirmação explícita. Se o item for excluído no Google, o cecistudy marca `sync_state = "removido_remoto"`, esconde o item da grade e **mantém** `execution_record` (D12) e `conflict_*`. O painel oferece "Recriar no Google" e "Descartar do cecistudy". Excluir no cecistudy envia `events.delete` **só** se a usuária confirmar, e o `events.delete` é sempre chamado com `sendUpdates=none`.
Justificativa: §10 pede exatamente isso, palavra por palavra ("pedir confirmação sobre a exclusão no outro sistema"; "o histórico acadêmico será preservado e a Ceci poderá recriar o evento"). `sendUpdates=none` evita notificar os convidados do evento sobre uma exclusão que a usuária não pediu para propagar.
Alternativa rejeitada: espelhar a exclusão sem perguntar (a usuária pode ter um compromisso que ela mesma criou no cecistudy e quer manter no Google).
Consequência: `AlertDialog` (instalado na spec 02, Fase 2.2) é o componente do aviso; o botão de excluir no item de `origin: "google"` diz "Excluir no Google".

**D39 — Indicador de estado é um triângulo, sempre visível, sem poluir.**
Decisão: um ícone pequeno por item, no canto superior direito: `Check` (sincronizado, sem nada a fazer), `ArrowUpDown` (pendente — há mudança local a enviar), `AlertTriangle` (conflito), `Ban` (removido no remoto). Mais um indicador **global** na `CalendarToolbar`: "Sincronizado há 4 min" / "Sincronizando…" / "Erro: token expirou — reconectar", com o botão "Sincronizar agora" e o contador de conflitos.
Justificativa: §11.9 pede "indicador de sincronização". `DESIGN.md:9` — a cor comunica estado; `PRODUCT.md:21` pune excesso. Um triângulo por item é denso e não compete com a cor de camada.
Alternativa rejeitada: uma cor de estado em todo item que tem vínculo (mudaria a leitura da camada e brigaria com D4); só um ícone global (não responde "por que este item não atualizou").
Consequência: `layer/google` é neutra (D4 da spec 00) e **não** recebe cor de estado; o estado é o ícone.

---

## 3. Comandos Rust novos

De 25 (spec 03) para **34**.

| Comando                    | Assinatura                                                                                    | Observação                                                                                |
| -------------------------- | --------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `google_begin_auth`        | `(): Promise<{ port: number; authUrl: string }>`                                              | sobe o listener e abre o navegador do sistema                                             |
| `google_complete_auth`     | `({ state, code }): Promise<ConnectionStatus>`                                                | valida `state`, troca o código, grava `google_connection`                                 |
| `google_list_calendars`    | `(): Promise<RemoteCalendar[]>`                                                               | `calendarList.list`                                                                       |
| `google_select_calendar`   | `({ calendarId }): Promise<ConnectionStatus>`                                                 | §10: dedicado, escolhido pela usuária                                                     |
| `google_disconnect`        | `(): Promise<void>`                                                                           | revoga, zera `sync_cursor`, marca os vínculos `"desconectado"`                            |
| `google_sync_now`          | `(): Promise<SyncReport>`                                                                     | full ou incremental conforme `sync_cursor`                                                |
| `google_conflict_resolve`  | `({ linkId, choice: "cecistudy" \| "google" \| "combinar", fields? }): Promise<ExternalLink>` | D37                                                                                       |
| `google_recreate_remote`   | `({ linkId }): Promise<ExternalLink>`                                                         | §10: recriar depois de remoção remota                                                     |
| `google_purge_local`       | `({ linkId }): Promise<void>`                                                                 | "Descartar do cecistudy", **sem** apagar `execution_record`                               |
| `google_connection_status` | `(): Promise<ConnectionStatus>`                                                               | `{ connected, email, calendarId, calendarName, lastSyncedAt, pending, conflicts, error }` |

`google_delete_remote` **não** existe como comando: exclusão passa por `delete_event` da spec 01, que detecta o `external_link` e devolve um `requiresConfirmation` com a lista de efeitos. O botão do diálogo é quem confirma.

Módulos Rust: `src-tauri/src/google/{mod.rs, calendar.rs, oauth.rs, conflict.rs, payload.rs}`. `payload.rs` é o **único** lugar do crate onde se escreve corpo de request, e ele só monta chaves de `SYNCABLE_FIELDS` (D35).

---

## 4. Bloqueios

- [ ] **BLOQUEIO — credenciais do Google.** Sem um projeto no Google Cloud Console, esta spec não é verificável. Desbloqueia: (a) criar o projeto; (b) ativar a **Google Calendar API**; (c) criar um **OAuth 2.0 Client ID do tipo Desktop app** (o tipo que continua aceitando _loopback_); (d) configurar a tela de consentimento como **Interno** (conta de teste), porque um app desktop em produção passaria por verificação do Google. Registrar `http://127.0.0.1` (sem porta fixa) como URI de redirecionamento autorizado.
- [ ] **BLOQUEIO — MSVC ausente** (`SPEC.md:10`). `reqwest` e `tiny_http`/listener compilam sem C extra, mas nada roda sem o toolchain. _Mitigação:_ a Fase 5.1 (mapeamento puro em TypeScript) é verificável com `node --test`.
- [ ] **BLOQUEIO — a conta de teste precisa de um calendário dedicado.** §10 fala em "um calendário dedicado no Google". Desbloqueia: a usuária cria/autoriza um calendário "Campus" na sua conta, ou aceita que o app sugira o principal.
- [ ] **BLOQUEIO (de produto) — escopo de leitura do Google.** Falta decidir se o Calendário só lê do calendário dedicado que ele mesmo escreve, ou também de outros da conta. **Decisão desta spec: só do calendário dedicado.** Os demais calendários dela não entram. Isso é D5 + §11.8 ("calendário dedicado") e é a opção reversível: acrescentar outros calendários é um checkbox, não uma mudança de modelo. Se o produto quiser "ler todos os meus eventos", é uma reabertura de D5.
- [ ] _(Risco aceito)_ Token em claro no SQLite local (A6). Mitigação futura: chaveiro do SO. Notificação opcional de novos itens sincronizados — fora de escopo.

---

## 5. Fases de implementação

### Fase 5.1 — Mapeamento puro (sem rede)

| Ação  | Detalhe                                                                    |
| ----- | -------------------------------------------------------------------------- |
| Criar | `src/features/calendar/sync/{payload,identity,permission}.ts` + `.test.ts` |

`payload.ts` monta/parseia o `events` resource a partir de um `CalendarEvent`; `identity.ts` decide se um evento remoto é nosso (`campus_entity`, `iCalUID`, `recurringEventId`, `originalStartTime`); `permission.ts` espelha a tabela de D35 em TypeScript, **para teste e para a UI desabilitar botões** — a autoridade continua sendo o Rust (D35).

**Aceite e teste:** `node --test src/features/calendar/sync/` verde com: payload só contém chaves de `SYNCABLE_FIELDS` (assert de lista de chaves do objeto, não de ausência); `identity` reconhece o próprio, o de outro app e um evento recurrence; item `origin: "google"` produz payload **vazio** (D35 linha 3); `npm run typecheck` → 0; `npm run lint` → `0 problems`; `rg "summary" src/features/calendar/sync/payload.ts` → só dentro da allowlist.

### Fase 5.2 — OAuth em Rust

| Ação   | Detalhe                                                                                                                                                                                |
| ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Editar | `src-tauri/Cargo.toml` — `reqwest = { version = "0.12", features = ["json", "rustls-tls"] }`, `tokio = { version = "1", features = ["full"] }`, `base64`, `sha2`, `uuid` (já com `v4`) |
| Criar  | `src-tauri/src/oauth.rs` (D32) + `src-tauri/src/google/oauth.rs`                                                                                                                       |
| Criar  | `src-tauri/migrations/0004_google_connection.sql` — a tabela singleton de D5                                                                                                           |
| Editar | `src-tauri/tauri.conf.json` — **só** `app.security.devCsp` se necessário; a `csp` de produção fica **intacta** (A6)                                                                    |

**Aceite e teste:** `rg "reqwest" src-tauri/Cargo.toml` → 1; `rg "access_type=offline" src-tauri/src/google/oauth.rs` → 1; `rg "prompt=consent" src-tauri/src/google/oauth.rs` → 1; `rg "code_challenge_method" src-tauri/src/google/oauth.rs` → 1; **`rg "googleapis" src-tauri/tauri.conf.json` continua vazio** — a CSP não foi tocada; `npm run typecheck` e `npm run lint` em 0; **[bloqueado em §4]** `google_begin_auth` abre o navegador, o consentimento mostra o nome "Campus", e `google_complete_auth` grava `google_connection` com `refresh_token` presente. O `refresh_token` **não** aparece em nenhum retorno de comando: `rg "refresh_token" src/lib/ipc.ts` → vazio.

### Fase 5.3 — Leitura incremental e materialização

| Ação   | Detalhe                                                                                                                        |
| ------ | ------------------------------------------------------------------------------------------------------------------------------ |
| Criar  | `src-tauri/src/google/calendar.rs` (D33) — `list_full`, `list_incremental`, tratamento de 410                                  |
| Criar  | `src-tauri/src/google/identity.rs` (D34)                                                                                       |
| Editar | `src-tauri/src/commands.rs` — `google_list_calendars`, `google_select_calendar`, `google_sync_now`, `google_connection_status` |
| Editar | `src-tauri/src/db.rs` — writer de `sync_cursor`                                                                                |

Regras: só o calendário dedicado; `singleEvents=true` nas leituras; evento remoto sem `campus_entity` entra como `CalendarEvent` com `origin = "google"` e `commitment = "recomendado"` (D32: nunca obrigação sem ação — INV-7); `status: "cancelled"` remoto entra como ocorrência cancelada, **não** como evento apagado; `recurrence` remota vira `RecurrenceRule` com `rrule` copiado verbatim (A3).

**Aceite e teste:** `node --test` verde no mapeamento; `rg "singleEvents" src-tauri/src/google/calendar.rs` → 1; `rg "410\|GONE" src-tauri/src/google/calendar.rs` → presente; `rg "is_basic_auth\|access_type" src/lib/ipc.ts` → vazio; `npm run typecheck` e `npm run lint` em 0; **[bloqueado em §4]** com uma conta de teste: um evento criado no Google aparece na grade com a cor neutra de D4, um evento criado no cecistudy aparece no navegador do Google, e `sync_cursor` tem `sync_token` não-nulo após o primeiro sync.

### Fase 5.4 — Envio: `payload.rs` e a allowlist como autoridade

| Ação   | Detalhe                                                                                                                |
| ------ | ---------------------------------------------------------------------------------------------------------------------- |
| Criar  | `src-tauri/src/google/payload.rs` (D35) e `src-tauri/src/google/mod.rs`                                                |
| Editar | `src-tauri/src/commands.rs` — `create_event` / `update_event` / `delete_event` chamam o sync quando há `external_link` |

Ordem: grava local **primeiro**; se há vínculo, `PATCH` com `If-Match`; sucesso → `sync_state = "sincronizado"` e `remote_etag` guardado; **412** → D37; erro de rede → `sync_state = "pendente"` e a interface mostra "Sincronizar agora". A fila de pendentes é reprocessada no início do próximo sync.

**Aceite e teste:** `rg "SYNCABLE_FIELDS" src-tauri/src/google/payload.rs` → 1 e é a única fonte de chaves; `rg "json!" src-tauri/src/google/` → nenhum corpo escrito à mão; `rg "If-Match" src-tauri/src/google/` → presente; `node --test src/features/calendar/sync/payload.test.ts` verde; `npm run typecheck` e `npm run lint` em 0; **[bloqueado em §4]** editar `location` no cecistudy altera o evento no Google; editar `notes` não altera nada no Google (só existe no cecistudy); editar um item importado **não** emite nenhum request.

### Fase 5.5 — Recorrência remota e remarcação

| Ação   | Detalhe                                                                                             |
| ------ | --------------------------------------------------------------------------------------------------- |
| Editar | `src-tauri/src/google/calendar.rs` — `patch_instance`, `cancel_instance` (D36)                      |
| Editar | `src-tauri/src/commands.rs` — `move_occurrence` e `set_occurrence_state` com vínculo chamam os dois |

**Aceite e teste:** `rg "recurringEventId" src-tauri/src/google/calendar.rs` → presente; `rg "campus_original_start" src-tauri/src` → 1; `npm run typecheck` e `npm run lint` em 0; **[bloqueado em §4]** remarcar uma terça no cecistudy move **só** aquela instância no Google; o próximo sync não recria a série; cancelar a ocorrência de um dia chuvoso deixa as outras intactas; INV-5 continua valendo — `rg` no log mostra que `event_recurrence` não foi tocada em nenhum dos dois casos.

### Fase 5.6 — Conflito

| Ação   | Detalhe                                                                                                                                           |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Criar  | `src-tauri/src/google/conflict.rs` (D37)                                                                                                          |
| Editar | `src-tauri/src/commands.rs` — `google_conflict_resolve`                                                                                           |
| Criar  | `src/features/calendar/sync/conflict.ts` — `safeToCombine(local, remote, base): Field[]` + `.test.ts` (o mesmo cálculo, em TypeScript, para a UI) |

**Aceite e teste:** `node --test src/features/calendar/sync/conflict.test.ts` verde com: campo alterado dos dois lados **não** é combinável; campo alterado de um lado só **é**; campo inexistente no `SYNCABLE_FIELDS` **nunca** é combinável; `rg "412" src-tauri/src/google/` presente e **sem** `retry` automático; `npm run typecheck` e `npm run lint` em 0; **[bloqueado em §4]** editando no Google e no cecistudy em seguida, o triângulo aparece, a tela de conflito lista as duas versões campo a campo, "Manter o do cecistudy" e "Manter o do Google" funcionam, e "Combinar" só oferece os campos seguros.

### Fase 5.7 — Exclusão, recriação e estado

| Ação   | Detalhe                                                                                                              |
| ------ | -------------------------------------------------------------------------------------------------------------------- |
| Editar | `src-tauri/src/commands.rs` — `delete_event` com vínculo devolve `requiresConfirmation` com a lista de efeitos (D38) |
| Editar | `src-tauri/src/commands.rs` — `google_recreate_remote`, `google_purge_local`, `google_disconnect`                    |
| Criar  | `src/features/calendar/sync/status.ts` — `StatusIcon` de D39                                                         |

**Aceite e teste:** `rg "sendUpdates=none" src-tauri/src/google/` → presente; `rg "DELETE FROM execution_record" src-tauri/src/commands.rs` → **vazio** (D12 + D38); `npm run typecheck` e `npm run lint` em 0; **[bloqueado em §4]** excluir um item vinculado abre confirmação nomeando os dois sistemas; excluir no Google e depois sincronizar deixa o item oculto com estado "Removido no Google", o histórico intacto, e "Recriar no Google" funcionando; a desconexão mantém todos os itens locais.

### Fase 5.8 — Indicador na interface

| Ação   | Detalhe                                                                           |
| ------ | --------------------------------------------------------------------------------- |
| Criar  | `ui/SyncBadge.tsx` (D39) e `ui/sections/SyncSection.tsx`                          |
| Editar | `ui/CalendarToolbar.tsx` (spec 02) — indicador global e botão "Sincronizar agora" |
| Editar | `ui/ConflictQueue.tsx` — lista de conflitos                                       |
| Editar | `ui/mobile/QuickActions.tsx` (spec 03) — "Sincronizar agora"                      |

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; `rg "text-chart" src/features/calendar/sync` → **vazio** (o estado de sync é ícone, não cor — D39 e D4); comportamento observável: item sincronizado mostra `Check`; editar offline mostra `ArrowUpDown` e a barra global mostra "2 pendentes"; um conflito mostra `AlertTriangle` e a barra global mostra "1 conflito"; §9 e §11.9 atendidos — **o Calendário interno funciona por completo com o Google desconectado**, que é a exigência de `calendario.md:146`.

---

## 6. Resumo de arquivos

**Criar**
`src/features/calendar/sync/{payload,identity,permission,conflict,status}.ts` + `.test.ts` · `src/features/calendar/ui/{SyncBadge,ConflictQueue}.tsx` · `src/features/calendar/ui/sections/SyncSection.tsx` · `src-tauri/src/oauth.rs` · `src-tauri/src/google/{mod,calendar,oauth,identity,payload,conflict}.rs` · `src-tauri/migrations/0004_google_connection.sql`

**Modificar**
`src-tauri/Cargo.toml` · `src-tauri/Cargo.lock` · `src-tauri/src/commands.rs` (+10 comandos) · `src-tauri/src/db.rs` · `src-tauri/src/lib.rs` · `src/lib/ipc.ts` · `src/features/calendar/ui/{CalendarToolbar,ContextPanel,TimedItem}.tsx` · `src/features/calendar/ui/mobile/QuickActions.tsx`

**Não tocar**
`src-tauri/tauri.conf.json` — **a CSP de produção não muda** (A6) · `src/styles.css` — nenhum token novo (D4, D39) · a grade e o arrasto (spec 02) · o timer (spec 03) · os adaptadores de origem (spec 04)

---

## 7. Critérios de aceite finais

1. `npm run typecheck` → 0 e `npm run lint` → `0 problems` em todas as fases.
2. **`rg "googleapis|oauth2.googleapis" src-tauri/tauri.conf.json` → vazio.** A CSP de produção é a mesma de `src-tauri/tauri.conf.json:31`.
3. `rg "refresh_token|access_token" src/lib/ipc.ts` → vazio: token nenhum atravessa a ponte.
4. `rg "reqwest" src/lib` → vazio: nenhuma chamada HTTP no front-end.
5. `rg "json!" src-tauri/src/google/` → nenhum corpo escrito à mão; `SYNCABLE_FIELDS` é a única fonte de chaves (D35).
6. `rg "sendUpdates=none" src-tauri/src/google/` → presente; `rg "DELETE FROM execution_record" src-tauri/src/commands.rs` → vazio.
7. `rg "tauri::command" src-tauri/src/commands.rs` → **34**.
8. `node --test src/features/calendar/sync/` e `node --test src/features/calendar/domain/` verdes, incluindo "campo alterado dos dois lados não é combinável" e "item `origin: google` produz payload vazio".
9. `rg "text-chart" src/features/calendar/sync` → vazio (D39).
10. **[bloqueado em §4]** Com a conta de teste: os cinco cenários do bloco de BLOQUEIO da Fase 5.3 e das Fases 5.5 a 5.8 são observáveis, **e** o Calendário funciona por completo com o Google desconectado.

---

## 8. Fora de escopo

- Escrever no calendário pessoal dela em vez do dedicado (ver D5 e o bloqueio de produto em §4).
- Importar de vários calendários da conta.
- `events.watch` / push — exige endpoint público (D33).
- `freeBusy` e disponibilidade.
- Convidados, `sendUpdates=all`, propose/reply, Rooms e `conferenceData`.
- Cor personal do calendário dedicado.
- Google Tasks, Drive, `.cectx` na Base de Conhecimento.
- Um segundo provedor (Outlook, ICS por arquivo). `*.ics` é leitura de arquivo, não integração — fora.
- Exclusão em massa com confirmação agrupada.
- Sincronização em background quando o app está fechado (impossível sem servidor; e §11 não pede).
- Tela de configurações de sincronização dedicada — a configuração vive na `CalendarToolbar` e em um `Dialog`, conforme `src/App.tsx:288` já ter um "Preferências" inerte.
- Escrita de um item de `origin: "google"` — proibida por INV-6 e D35 linha 3, sem exceção.
- Repopular a `TodayView` (`src/App.tsx:476-706`) com itens sincronizados — backlog.
