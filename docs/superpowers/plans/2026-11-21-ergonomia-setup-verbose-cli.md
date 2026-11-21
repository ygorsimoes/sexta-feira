# Plano de Implementação: Ergonomia de setup manual, diagnóstico e modo `--verbose` da CLI

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tornar a CLI `sexta-feira` intuitiva, observável e robusta com os novos subcomandos `iniciar` e `diagnostico`, flag global acumulável `-v` / `--verbose` em `stderr`, saída enriquecida com próximos passos nos subcomandos atuais e dicas acionáveis de erro, além de atualizar `SKILL.md` e `README.md`.

**Architecture:** `sexta-feira-core` ganha apenas inspeção agnóstica das camadas de configuração (`CamadaDeDefinicao`); `sexta-feira-adapters` fornece metadados diagnósticos dos harnesses (`inspecionar_ambiente_harnesses`); e `sexta-feira-cli` organiza o modo verbose (`verbose.rs`), diagnóstico (`diagnostico.rs`), onboarding (`iniciar.rs`) e melhoria de ergonomia/erros (`main.rs`).

**Tech Stack:** Rust 2024 (`rust-version = "1.100"`), `clap`, `serde`, `serde_json`, `toml`, `figment`, `thiserror`, `anyhow`, `tracing`, `tracing-subscriber`, `assert_cmd`, `insta`.

**Spec:** [docs/superpowers/specs/2026-11-21-ergonomia-setup-verbose-cli-design.md](../specs/2026-11-21-ergonomia-setup-verbose-cli-design.md)

## Global Constraints

- Rust edição 2024, `rust-version = "1.100"`, `unsafe_code = "forbid"`, `missing_docs = "warn"`, `clippy::pedantic`, `unwrap_used` e `expect_used` proibidos fora de testes (`Cargo.toml`, `AGENTS.md`).
- Zero conhecimento específico de harness em `crates/sexta-feira-core` (`AGENTS.md`).
- Nenhuma dependência fora de [docs/stack.md](../../stack.md).
- Documentação, identificadores de domínio, flags e mensagens ao usuário em `pt-BR`.
- `--json` deve emitir exclusivamente JSON válido em `stdout`; toda saída de `-v`/`--verbose` e `tracing` vai para `stderr`.
- Preservar todos os contratos e testes existentes em `crates/sexta-feira-cli/tests/cli.rs` (incluindo snapshot de `harnesses`).

## Review Focus

- **`--json` + `-vvv` sem contaminação de `stdout`:** mesmo com `-vvv` ou `SEXTA_FEIRA_LOG=trace`, `stdout` continua parseável como JSON puro em `harnesses`, `plano`, `aplicar`, `verificar`, `diagnostico` e `iniciar`.
- **`iniciar` sem TTY:** quando `stdin` não é terminal e `--nao-interativo` está ausente, falha imediatamente com mensagem clara e nunca trava lendo `stdin`.
- **Idempotência, patch e backup em `iniciar`:** arquivo inexistente cria (`"criado"`); arquivo existente com conteúdo igual mantém sem `.bak` (`"mantido"`); arquivo existente com alteração falha sem `--sobrescrever` (ou preserva campos não passados e gera `.bak` numerado com `--sobrescrever`, reportando `"atualizado"`); arquivo corrompido falha sem `--sobrescrever` e regenera com `.bak` com `--sobrescrever`.
- **Rejeição antes da gravação em `iniciar`:** valores vazios (ex.: `--tratamento "  "`) ou `--preferencia ""` falham na validação via `DefinicaoDoMordomo::carregar` sem gravar `usuario.toml` inválido nem sobrescrever arquivo bom.
- **`diagnostico` resiliente com config inválida:** mesmo quando `usuario.toml` em `${XDG_CONFIG_HOME:-~/.config}/sexta-feira/` tem erro de sintaxe ou chave desconhecida, `sexta-feira diagnostico` retorna exit `0`, registra o erro no relatório e continua inspecionando os harnesses.

---

## Mapa de Arquivos

- Modify: `crates/sexta-feira-core/src/definicao.rs` — `OrigemDaCamada`, `StatusDaCamada`, `CamadaDeDefinicao`, `FontesDaDefinicao::inspecionar_camadas`, `DefinicaoDoMordomo::carregar_com_camadas`.
- Modify: `crates/sexta-feira-core/src/lib.rs` — re-exportação dos tipos públicos de camadas.
- Modify: `crates/sexta-feira-adapters/src/hermes.rs` — `OrigemResolucaoDestino`, `ConfiguracaoHermes::resolver_com_origem`.
- Modify: `crates/sexta-feira-adapters/src/catalogo.rs` — `DiagnosticoHarness`, `inspecionar_ambiente_harnesses`.
- Modify: `crates/sexta-feira-adapters/src/lib.rs` — re-exportação dos tipos diagnósticos.
- Create: `crates/sexta-feira-cli/src/verbose.rs` — renderização do bloco `[verbose]` (`-v` e `-vv`) para `stderr` e inicialização do filtro de `tracing`.
- Create: `crates/sexta-feira-cli/src/diagnostico.rs` — lógica e saída (texto e `--json`) do subcomando `diagnostico`.
- Create: `crates/sexta-feira-cli/src/iniciar.rs` — lógica interativa/não-interativa, patch, validação e backup `.bak` numerado do subcomando `iniciar`.
- Modify: `crates/sexta-feira-cli/src/main.rs` — flag global `-v`/`--verbose`, novos subcomandos `Iniciar` e `Diagnostico`, `--help` com `after_help`, saída enriquecida em `plano`/`aplicar`/`verificar` e dicas acionáveis de erro.
- Modify: `crates/sexta-feira-cli/tests/cli.rs` — testes E2E cobrindo todos os novos comportamentos e casos de borda.
- Modify: `recursos/skills/instalar-sexta-feira/SKILL.md` e `README.md` — documentação dos novos comandos e `-v`/`--verbose`.

---

### Task 1: Inspeção agnóstica de camadas em `sexta-feira-core`

**Files:**
- Modify: `crates/sexta-feira-core/src/definicao.rs`
- Modify: `crates/sexta-feira-core/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub enum OrigemDaCamada { Embutida, DiretorioConfig, ArquivoExplicito }`
  - `pub enum StatusDaCamada { Aplicada, Ausente }`
  - `pub struct CamadaDeDefinicao { pub secao: &'static str, pub origem: OrigemDaCamada, pub caminho: Option<PathBuf>, pub status: StatusDaCamada }`
  - `impl FontesDaDefinicao { pub fn inspecionar_camadas(&self) -> Vec<CamadaDeDefinicao> }`
  - `impl DefinicaoDoMordomo { pub fn carregar_com_camadas(fontes: &FontesDaDefinicao) -> Result<(Self, Vec<CamadaDeDefinicao>), ErroDeDefinicao> }`

- [ ] **Step 1: Escrever teste falhando em `crates/sexta-feira-core/src/definicao.rs`**

```rust
#[test]
fn inspecionar_camadas_reflete_ordem_e_status_sem_conhecer_harnesses() {
    let dir = DirTemporario::novo("camadas-info");
    fs::write(dir.path().join("usuario.toml"), "tratamento = \"capitão\"\n").expect("grava");
    let explicito = dir.path().join("persona-extra.toml");
    fs::write(&explicito, "nome = \"Sexta\"\n").expect("grava");

    let fontes = FontesDaDefinicao {
        diretorio_config: Some(dir.path().to_path_buf()),
        persona_explicita: Some(explicito.clone()),
        usuario_explicito: None,
    };
    let (def, camadas) =
        DefinicaoDoMordomo::carregar_com_camadas(&fontes).expect("carrega com camadas");
    assert_eq!(def.persona.nome, "Sexta");
    assert_eq!(def.usuario.tratamento, "capitão");
    assert_eq!(camadas.len(), 5);
    assert_eq!(camadas[0].secao, "persona");
    assert_eq!(camadas[0].origem, OrigemDaCamada::Embutida);
    assert_eq!(camadas[1].status, StatusDaCamada::Ausente);
    assert_eq!(camadas[2].status, StatusDaCamada::Aplicada);
    assert_eq!(camadas[2].caminho, Some(explicito));
    assert_eq!(camadas[3].secao, "usuario");
    assert_eq!(camadas[4].status, StatusDaCamada::Aplicada);
}
```

- [ ] **Step 2: Rodar o teste para confirmar falha de compilação**

Run: `cargo test -p sexta-feira-core inspecionar_camadas_reflete_ordem_e_status_sem_conhecer_harnesses`
Expected: FAIL (`OrigemDaCamada` / `carregar_com_camadas` não definidos).

- [ ] **Step 3: Implementar `OrigemDaCamada`, `StatusDaCamada`, `CamadaDeDefinicao`, `inspecionar_camadas` e `carregar_com_camadas` em `definicao.rs` e re-exportar em `lib.rs`**

Para cada seção (`"persona"` e `"usuario"`), emitir na ordem: `Embutida` (`Aplicada`, `caminho: None`), `DiretorioConfig` (`caminho: Some(dir.join("<secao>.toml"))`, status conforme `.exists()` quando `diretorio_config` for `Some`) e `ArquivoExplicito` (apenas quando `Some(p)`, com status conforme `p.exists()`). `carregar` delega para `carregar_com_camadas`. Documentar todos os itens públicos com docstrings (`missing_docs = "warn"`).

- [ ] **Step 4: Rodar os testes de `sexta-feira-core`**

Run: `cargo test -p sexta-feira-core --locked`
Expected: PASS (todos os testes de `sexta-feira-core`, incluindo o novo).

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-core/src/definicao.rs crates/sexta-feira-core/src/lib.rs
git commit -m "feat(core): expõe inspeção agnóstica das camadas de definição"
```

---

### Task 2: Metadados diagnósticos de harnesses em `sexta-feira-adapters`

**Files:**
- Modify: `crates/sexta-feira-adapters/src/hermes.rs`
- Modify: `crates/sexta-feira-adapters/src/catalogo.rs`
- Modify: `crates/sexta-feira-adapters/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub enum OrigemResolucaoDestino { Explicito, VariavelAmbiente, PadraoHome, Indisponivel }`
  - `impl ConfiguracaoHermes { pub fn resolver_com_origem(hermes_home_explicito: Option<&Path>, env_hermes_home: Option<&str>, env_home: Option<&str>, perfil: Option<&str>) -> Result<(Self, OrigemResolucaoDestino), ErroDeHermes> }`
  - `pub struct DiagnosticoHarness { pub harness: Harness, pub disponivel: bool, pub status: &'static str, pub destino_base: Option<PathBuf>, pub origem_destino: OrigemResolucaoDestino, pub destino_existe: bool, pub diretorio_perfil: Option<PathBuf>, pub perfil_existe: bool, pub arquivo_estado: Option<PathBuf>, pub estado_existe: bool, pub caminhos_indicativos: Vec<PathBuf> }`
  - `pub fn inspecionar_ambiente_harnesses(filtro: Option<Harness>, destino_explicito: Option<&Path>, perfil: Option<&str>, env_hermes_home: Option<&str>, env_home: Option<&str>) -> Vec<DiagnosticoHarness>`

- [ ] **Step 1: Escrever teste falhando em `crates/sexta-feira-adapters/src/catalogo.rs`**

Testar `inspecionar_ambiente_harnesses` com um diretório temporário para `env_home` contendo `<tmp>/.hermes/profiles/sexta-feira/.sexta-feira-estado.toml` e `<tmp>/.claude`: verificar que `hermes` reporta `disponivel: true`, `origem_destino: OrigemResolucaoDestino::PadraoHome`, `destino_existe: true`, `perfil_existe: true`, `estado_existe: true`, enquanto `claude-code` reporta `disponivel: false` e `caminhos_indicativos` contendo `<tmp>/.claude`.

- [ ] **Step 2: Rodar o teste para confirmar falha de compilação**

Run: `cargo test -p sexta-feira-adapters inspecionar_ambiente_harnesses`
Expected: FAIL.

- [ ] **Step 3: Implementar `resolver_com_origem` em `hermes.rs`, `DiagnosticoHarness` e `inspecionar_ambiente_harnesses` em `catalogo.rs`, e re-exportar em `lib.rs`**

Caminhos indicativos para harnesses `em breve` quando `env_home` não for vazio:
- `openclaw`: `<HOME>/.openclaw`
- `claude-code`: `<HOME>/.claude`
- `claude-desktop`: `<HOME>/.config/Claude` (não-macOS) / `<HOME>/Library/Application Support/Claude` (macOS)
- `opencode`: `<HOME>/.config/opencode`

- [ ] **Step 4: Rodar os testes de `sexta-feira-adapters`**

Run: `cargo test -p sexta-feira-adapters --locked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-adapters/src/hermes.rs crates/sexta-feira-adapters/src/catalogo.rs crates/sexta-feira-adapters/src/lib.rs
git commit -m "feat(adapters): adiciona metadados diagnósticos do ambiente por harness"
```

---

### Task 3: Modo global `-v` / `--verbose`, `--help` e saída/erros acionáveis na CLI

**Files:**
- Create: `crates/sexta-feira-cli/src/verbose.rs`
- Modify: `crates/sexta-feira-cli/src/main.rs`
- Modify: `crates/sexta-feira-cli/tests/cli.rs`

**Interfaces:**
- Consumes: `CamadaDeDefinicao`, `DefinicaoDoMordomo::carregar_com_camadas`, `ConfiguracaoHermes::resolver_com_origem`, `PlanoDeSetup`, `ResultadoDaVerificacao`.
- Produces: módulo `verbose.rs` (`inicializar_tracing`, `emitir_ambiente`, `emitir_contexto_definicao`, `emitir_contexto_plano`, `emitir_contexto_verificacao`), flag global `-v`/`--verbose`, `--help` com `after_help`, saídas enriquecidas em `plano`/`aplicar`/`verificar` e dicas `"Dica: ..."` em erros.

- [ ] **Step 1: Escrever testes falhando em `crates/sexta-feira-cli/tests/cli.rs`**

Cobrir:
1. `verbose_emite_contexto_rico_em_stderr_sem_poluir_json_em_stdout`: rodar `--json -vv plano hermes --destino ...`, validar que `stdout` faz parse como JSON puro e `stderr` contém `[verbose]`, camadas de configuração, destino resolvido, estratégia `ArquivoIntegral`/`BlocoDelimitado`, `SOUL.md` (contagem de linhas), e variáveis de ambiente (`HOME`/`XDG_CONFIG_HOME`).
2. `sexta_feira_log_mantem_precedencia_sobre_verbose`: `SEXTA_FEIRA_LOG=warn -vv harnesses` não emite `DEBUG` do `tracing` enquanto `-vv` sem `SEXTA_FEIRA_LOG` emite `DEBUG`.
3. `plano_aplicar_e_erros_exibem_dicas_acionaveis`: `plano` exibe caminho do perfil e dica de `aplicar`; `aplicar` exibe caminho aplicado e próximos passos (`verificar` e config); erro em harness `em breve` menciona `sexta-feira harnesses` e instalação assistida (`SKILL.md`); erro de conflito sem `--forcar` orienta usar `plano` ou `--forcar`; erro de chave desconhecida orienta chaves válidas / `diagnostico`.

- [ ] **Step 2: Rodar novos testes para confirmar falha**

Run: `cargo test -p sexta-feira-cli --test cli verbose_emite_contexto_rico`
Expected: FAIL (`unexpected argument '-vv'` / ausência das dicas).

- [ ] **Step 3: Implementar `crates/sexta-feira-cli/src/verbose.rs` e atualizar `crates/sexta-feira-cli/src/main.rs`**

- Adicionar `#[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true)] verbose: u8` na `Cli`.
- Configurar `tracing_subscriber` respeitando `SEXTA_FEIRA_LOG` quando não vazia; caso contrário mapeando `verbose` (`0 => warn`, `1 => info`, `2 => debug`, `>= 3 => trace`).
- Em `verbose.rs`, emitir blocos `[verbose]` em `std::io::stderr()` para `-v` (camadas, definição, origem/caminho do destino, estado, estratégia/permissão/operação de cada arquivo) e `-vv` (variáveis de ambiente e linhas/bytes dos artefatos).
- Adicionar `after_help` com exemplos em `pt-BR` em `Cli` e nos subcomandos.
- Enriquecer `plano`, `aplicar`, `verificar` (sem alterar o snapshot de `harnesses`) e formatar erros com `"Dica: ..."` antes de encerrar no `main`.

- [ ] **Step 4: Rodar todos os testes de `sexta-feira-cli`**

Run: `cargo test -p sexta-feira-cli --locked`
Expected: PASS (todos os testes anteriores + os novos).

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-cli/src/verbose.rs crates/sexta-feira-cli/src/main.rs crates/sexta-feira-cli/tests/cli.rs
git commit -m "feat(cli): adiciona modo --verbose detalhado, ajuda e erros acionáveis"
```

---

### Task 4: Subcomando `sexta-feira diagnostico`

**Files:**
- Create: `crates/sexta-feira-cli/src/diagnostico.rs`
- Modify: `crates/sexta-feira-cli/src/main.rs`
- Modify: `crates/sexta-feira-cli/tests/cli.rs`

**Interfaces:**
- Consumes: `FontesDaDefinicao`, `DefinicaoDoMordomo::carregar_com_camadas`, `inspecionar_ambiente_harnesses`, `AdaptadorHermes`, `verificar`, `verbose`.
- Produces: subcomando `Diagnostico` (`[harness]`, `--destino`, `--perfil`, `--persona`, `--usuario`) emitindo texto estruturado ou `--json` com exit `0`.

- [ ] **Step 1: Escrever testes falhando em `crates/sexta-feira-cli/tests/cli.rs`**

Cobrir:
1. `diagnostico_em_json_e_texto_cobre_ambiente_config_invalida_e_harnesses`:
   - Setup não aplicado: exit `0`, JSON contém `versao`, `plataforma`, `configuracao`, `definicao.valida == true`, `harnesses[0].id == "hermes"`, `proximo_passo` sugerindo `plano`/`aplicar`.
   - `usuario.toml` com chave desconhecida em `XDG_CONFIG_HOME`: `diagnostico` ainda retorna exit `0`, `definicao.valida == false` com `erro` detalhado e texto exibindo o problema sem abortar.
   - Filtrado com `diagnostico hermes --destino ...` após `aplicar`: mostra `sincronizado: true` e `proximo_passo` indicando setup sincronizado.

- [ ] **Step 2: Rodar teste para confirmar falha**

Run: `cargo test -p sexta-feira-cli --test cli diagnostico_em_json`
Expected: FAIL.

- [ ] **Step 3: Implementar `crates/sexta-feira-cli/src/diagnostico.rs` e integrar em `main.rs`**

Emitir JSON e texto estruturado com versão, SO/arch, variáveis de ambiente, camadas e status de validação da definição, lista de harnesses inspecionados (com status de sincronização quando `hermes` resolver) e `proximo_passo`. Integrar com `-v` / `-vv`.

- [ ] **Step 4: Rodar os testes de `sexta-feira-cli`**

Run: `cargo test -p sexta-feira-cli --locked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-cli/src/diagnostico.rs crates/sexta-feira-cli/src/main.rs crates/sexta-feira-cli/tests/cli.rs
git commit -m "feat(cli): adiciona subcomando diagnostico com saída em texto e JSON"
```

---

### Task 5: Subcomando `sexta-feira iniciar`

**Files:**
- Create: `crates/sexta-feira-cli/src/iniciar.rs`
- Modify: `crates/sexta-feira-cli/src/main.rs`
- Modify: `crates/sexta-feira-cli/tests/cli.rs`

**Interfaces:**
- Consumes: `FontesDaDefinicao`, `DefinicaoDoMordomo::carregar`, `AdaptadorHermes`, `planejar`, `aplicar`, `verificar`, `verbose`.
- Produces: subcomando `Iniciar` com flags `--tratamento`, `--idioma`, `--preferencia`, `--config-dir`, `--harness`, `--destino`, `--perfil`, `--aplicar`, `--forcar`, `--sobrescrever`, `--nao-interativo`.

- [ ] **Step 1: Escrever testes falhando em `crates/sexta-feira-cli/tests/cli.rs`**

Cobrir todos os casos da spec 5.4:
1. `iniciar_sem_tty_falha_sem_nao_interativo`: rodar `iniciar` sem `--nao-interativo` em ambiente sem TTY falha imediatamente citando `--nao-interativo`.
2. `iniciar_nao_interativo_cria_mantem_mescla_com_backup_e_rejeita_invalido`:
   - Cria `usuario.toml` novo com `--tratamento "comandante"` (`"acao": "criado"` no `--json`).
   - Repetir `iniciar --nao-interativo` sem alterações mantém o arquivo sem criar `.bak` (`"acao": "mantido"`).
   - Passar `--tratamento "chefe"` sem `--sobrescrever` falha pedindo `--sobrescrever`.
   - Passar `--sobrescrever --preferencia "Direto ao ponto"` preserva `tratamento = "comandante"` anterior, gera `usuario.toml.bak` e reporta `"acao": "atualizado"`.
   - `usuario.toml` corrompido falha sem `--sobrescrever` e regenera com `.bak` com `--sobrescrever`.
   - `--tratamento "   "` falha na validação antes de gravar/sobrescrever.
3. `iniciar_nao_interativo_pode_aplicar_no_hermes_e_emitir_json`: encadeia `--harness hermes --destino ... --aplicar --json` e verifica que o perfil Hermes é criado com o tratamento configurado.

- [ ] **Step 2: Rodar teste para confirmar falha**

Run: `cargo test -p sexta-feira-cli --test cli iniciar_`
Expected: FAIL.

- [ ] **Step 3: Implementar `crates/sexta-feira-cli/src/iniciar.rs` e integrar em `main.rs`**

Implementar:
- Detecção de TTY (`std::io::stdin().is_terminal()`).
- Prompt interativo limpo quando em TTY.
- Leitura e patch sobre `usuario.toml` existente quando TOML válido; detecção de idempotência para retornar `"mantido"` sem `.bak`; geração de `.bak` numerado (`usuario.toml.bak`, `.bak.1`, ...) ao sobrescrever alterações.
- Validação em memória via `DefinicaoDoMordomo::carregar` antes de tocar no disco.
- Encadeamento opcional com `plano` / `aplicar` / `verificar` quando `--harness` for informado.

- [ ] **Step 4: Rodar os testes de `sexta-feira-cli`**

Run: `cargo test -p sexta-feira-cli --locked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-cli/src/iniciar.rs crates/sexta-feira-cli/src/main.rs crates/sexta-feira-cli/tests/cli.rs
git commit -m "feat(cli): adiciona subcomando iniciar para setup guiado e scriptável"
```

---

### Task 6: Atualizar `SKILL.md`, `README.md` e validar todo o workspace

**Files:**
- Modify: `recursos/skills/instalar-sexta-feira/SKILL.md`
- Modify: `README.md`

- [ ] **Step 1: Atualizar `recursos/skills/instalar-sexta-feira/SKILL.md`**

- Passo 1/3: mencionar `sexta-feira --json diagnostico` (e `-v`/`-vv` para contexto completo em `stderr`) assim que a CLI estiver disponível.
- Passo 4: sugerir `sexta-feira iniciar --nao-interativo ...` como forma segura e validada de criar/atualizar `usuario.toml`, mantendo os alertas de substituição de listas e chaves permitidas.
- Passo 5A: lembrar que `-v` / `-vv` exibem camadas, destino resolvido e detalhes de cada arquivo sem alterar `stdout` do `--json`.

- [ ] **Step 2: Atualizar `README.md`**

- Seção **Instalação manual pela CLI**: incluir `sexta-feira diagnostico` e `sexta-feira iniciar` no fluxo rápido.
- Seção **Referência da CLI**: adicionar `iniciar`, `diagnostico` e `-v` / `--verbose` (`-vv` / `-vvv`).

- [ ] **Step 3: Rodar checagem completa do workspace**

Run:
```bash
cargo fmt --all --check && \
cargo clippy --workspace --all-targets --locked -- -D warnings && \
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked && \
cargo test --workspace --locked && \
cargo test --doc --workspace --locked
```
Expected: PASS com 0 avisos e 0 falhas.

- [ ] **Step 4: Commit**

```bash
git add recursos/skills/instalar-sexta-feira/SKILL.md README.md
git commit -m "docs: atualiza skill e README com iniciar, diagnostico e --verbose"
```
