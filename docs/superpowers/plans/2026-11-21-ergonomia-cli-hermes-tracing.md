# Plano de Implementação: Ergonomia da CLI, adaptador Hermes e instrumentação com `tracing`

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrar `FontesDaDefinicao`, `--destino` (com alias `--hermes-home`), `--persona`, `--usuario`, status de `harnesses`, saída `--json`, contagem de remoções no `plano`, enxugamento de `SOUL.md`/`USER.md`, expansão de `~/` no `ConfiguracaoHermes` e instrumentação `tracing` em todo o workspace.

**Architecture:** As responsabilidades permanecem divididas entre `sexta-feira-adapters` (`catalogo.rs` para disponibilidade e `hermes.rs` para expansão de til e logs), `recursos/mordomo/usuario.toml` (remoção de redundância literal), `sexta-feira-core` (instrumentação `tracing::debug!`) e `sexta-feira-cli` (flags compartilhadas, carregamento em camadas e serialização `--json` em `stdout` mantendo logs em `stderr`).

**Tech Stack:** Rust edição 2024 (`1.100.0`), `clap`, `serde`, `serde_json`, `tracing`, `tracing-subscriber`, `assert_cmd`, `insta`.

**Spec:** [docs/superpowers/specs/2026-11-21-ergonomia-cli-hermes-tracing-design.md](../specs/2026-11-21-ergonomia-cli-hermes-tracing-design.md)

## Global Constraints

- Rust edição 2024, `rust-version = "1.100"`, `unsafe_code = "forbid"`, `missing_docs = "warn"`, `clippy::pedantic`, `unwrap_used` e `expect_used` proibidos fora de testes (`Cargo.toml`).
- Nada específico de harness entra em `sexta-feira-core` (`AGENTS.md`).
- Documentação, commits, comentários e identificadores de domínio em `pt-BR` (`AGENTS.md`).
- Dependências novas restritas às aprovadas em [docs/stack.md](../../stack.md) (`tracing` + `serde_json` na CLI), declaradas em `[workspace.dependencies]`.
- Todo trabalho ocorre na branch `feat/ergonomia-cli-hermes-tracing`, nunca na `main` (`AGENTS.md`).
- TDD obrigatório: teste falhando (`RED`) → confirmação da falha → implementação mínima (`GREEN`) → suíte verde.

## Review Focus

- **`--json` em `verificar` quando há pendências:** `sexta-feira --json verificar hermes --destino <dir>` quando arquivos faltam ou divergem deve imprimir JSON válido em `stdout` (`"sincronizado": false`) e **ainda sair com código não-zero**, permitindo que scripts leiam o diagnóstico estruturado e detectem a falha pelo exit code.
- **Expansão de `~/` com `HOME` ausente ou vazio:** passar `--destino ~/.hermes` (ou `HERMES_HOME=~/.hermes`) quando `HOME` não está definido ou está em branco deve falhar com `ErroDeHermes::DiretorioBaseIndisponivel`, nunca criar uma pasta `./~` no diretório atual.
- **Logs `tracing` sem contaminar `stdout` nem `--json`:** sem `SEXTA_FEIRA_LOG`, `stderr` e `stdout` ficam limpos de linhas de log; com `SEXTA_FEIRA_LOG=debug`, os logs aparecem apenas em `stderr` mantendo `stdout` parseável como JSON.
- **Retrocompatibilidade de `--hermes-home`:** scripts existentes que invoquem `sexta-feira plano hermes --hermes-home <dir>` devem continuar funcionando identicamente a `--destino <dir>`, sem exibir `--hermes-home` no `--help`.
- **Personalização via `--persona` e `--usuario`:** passar `--usuario <arq>` (ex.: sobrescrevendo `tratamento = "comandante"`) deve refletir no `SOUL.md` e `USER.md` gerados por `plano`/`aplicar`/`verificar`, e arquivo ausente deve falhar com erro claro sem gravar nada em disco.

---

## Mapa de Arquivos

- `Cargo.toml` — adiciona `tracing = "0.1.44"` em `[workspace.dependencies]`.
- `crates/sexta-feira-core/Cargo.toml`, `crates/sexta-feira-adapters/Cargo.toml`, `crates/sexta-feira-cli/Cargo.toml` — adicionam `tracing.workspace = true`; a CLI adiciona `serde.workspace = true` e `serde_json.workspace = true`.
- `docs/stack.md` — marca `tracing` como `em uso`.
- `crates/sexta-feira-adapters/src/catalogo.rs` — `Harness::disponivel` e `Harness::rotulo_status`.
- `crates/sexta-feira-adapters/src/hermes.rs` — expansão de `~`/`~/` em `ConfiguracaoHermes::resolver_com_env` e `tracing::debug!`.
- `recursos/mordomo/usuario.toml` — remove repetição literal de tratamento e idioma em `preferencias`.
- `crates/sexta-feira-core/src/definicao.rs` e `crates/sexta-feira-core/src/plano.rs` — `tracing::debug!` em carregamento, planejamento e aplicação.
- `crates/sexta-feira-cli/src/main.rs` — `--json`, `--destino` (alias `--hermes-home`), `--persona`, `--usuario`, status de `harnesses`, contagem de `remover`, dica em `verificar` e `EnvFilter` padrão `"warn"`.
- `crates/sexta-feira-cli/tests/cli.rs` — atualiza snapshot de `harnesses` e adiciona testes E2E para `--destino`, `--hermes-home`, `--persona`, `--usuario`, `--json` e `SEXTA_FEIRA_LOG`.

---

### Task 1: Status de disponibilidade no catálogo (`catalogo.rs`)

**Files:**
- Modify: `crates/sexta-feira-adapters/src/catalogo.rs:24-87`

**Interfaces:**
- Produces:
  - `pub const fn disponivel(self) -> bool` em `Harness`
  - `pub const fn rotulo_status(self) -> &'static str` em `Harness` (`"disponível"` | `"em breve"`)

- [ ] **Step 1: Escrever teste falhando em `crates/sexta-feira-adapters/src/catalogo.rs`**

```rust
#[test]
fn hermes_esta_disponivel_e_demais_harnesses_em_breve() {
    assert!(Harness::Hermes.disponivel());
    assert_eq!(Harness::Hermes.rotulo_status(), "disponível");

    for outro in [
        Harness::Openclaw,
        Harness::ClaudeCode,
        Harness::ClaudeDesktop,
        Harness::Opencode,
    ] {
        assert!(!outro.disponivel());
        assert_eq!(outro.rotulo_status(), "em breve");
    }
}
```

- [ ] **Step 2: Rodar o teste para confirmar falha (`RED`)**

Run: `cargo test -p sexta-feira-adapters catalogo::tests`
Expected: FAIL (`disponivel` e `rotulo_status` não existem).

- [ ] **Step 3: Implementar `disponivel` e `rotulo_status` em `Harness`**

Em `crates/sexta-feira-adapters/src/catalogo.rs`, adicionar os dois métodos `pub const fn` com docstrings em pt-BR.

- [ ] **Step 4: Rodar testes e clippy (`GREEN`)**

Run: `cargo test -p sexta-feira-adapters && cargo clippy -p sexta-feira-adapters --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-adapters/src/catalogo.rs
git commit -m "feat(adapters): expõe status de disponibilidade no catálogo de harnesses"
```

---

### Task 2: Expansão de `~/` no Hermes e enxugamento de `usuario.toml`

**Files:**
- Modify: `crates/sexta-feira-adapters/src/hermes.rs:79-115,279-392`
- Modify: `recursos/mordomo/usuario.toml:1-7`

**Interfaces:**
- Produces: expansão segura de `~` e `~/` em `ConfiguracaoHermes::resolver_com_env` usando `env_home` (retornando `ErroDeHermes::DiretorioBaseIndisponivel` se `env_home` ausente/vazio).

- [ ] **Step 1: Escrever testes falhando em `crates/sexta-feira-adapters/src/hermes.rs`**

```rust
#[test]
fn expande_til_com_home_e_rejeita_til_sem_home() {
    let via_flag = ConfiguracaoHermes::resolver_com_env(
        Some(Path::new("~/.hermes-custom")),
        None,
        Some("/home/ygor"),
        None,
    )
    .expect("deve expandir ~/");
    assert_eq!(via_flag.hermes_home, PathBuf::from("/home/ygor/.hermes-custom"));

    let via_env_raiz = ConfiguracaoHermes::resolver_com_env(
        None,
        Some("~"),
        Some("/home/ygor"),
        None,
    )
    .expect("deve expandir ~");
    assert_eq!(via_env_raiz.hermes_home, PathBuf::from("/home/ygor"));

    let sem_home = ConfiguracaoHermes::resolver_com_env(
        Some(Path::new("~/.hermes")),
        None,
        Some("   "),
        None,
    )
    .expect_err("til sem HOME deve falhar");
    assert!(matches!(sem_home, ErroDeHermes::DiretorioBaseIndisponivel));
}

#[test]
fn user_md_nao_duplica_literalmente_tratamento_e_idioma_nas_preferencias() {
    let def = DefinicaoDoMordomo::padrao().expect("definição padrão");
    let cfg = ConfiguracaoHermes::resolver_com_env(Some(Path::new("/tmp/hermes")), None, None, None)
        .expect("cfg");
    let arquivos = AdaptadorHermes::new(cfg).planejar_arquivos(&def).expect("arquivos");
    let user = &arquivos[1].conteudo;
    assert_eq!(user.matches("senhor").count(), 1);
    assert_eq!(user.matches("pt-BR").count(), 1);
}
```

- [ ] **Step 2: Rodar os testes para confirmar falha (`RED`)**

Run: `cargo test -p sexta-feira-adapters hermes::tests`
Expected: FAIL (`expande_til_com_home_e_rejeita_til_sem_home` e `user_md_nao_duplica...` falham).

- [ ] **Step 3: Implementar `expandir_til` em `hermes.rs` e enxugar `recursos/mordomo/usuario.toml`**

- Em `hermes.rs`: helper `expandir_til(caminho: &Path, env_home: Option<&str>) -> Result<PathBuf, ErroDeHermes>` tratando `~` exato e `~/` (ou `~\`? apenas `~` e `~/`), usando `env_home.map(str::trim).filter(|s| !s.is_empty())` e retornando `DiretorioBaseIndisponivel` caso ausente.
- Em `recursos/mordomo/usuario.toml`: substituir as `preferencias` para remover a repetição literal de `"senhor"` e `"pt-BR"` já presentes em `tratamento` e `idioma`:
```toml
tratamento = "senhor"
idioma = "pt-BR"
preferencias = [
  "Estruturar respostas de forma clara, direta e concisa",
  "Priorizar decisões práticas e bem fundamentadas, destacando riscos reais",
]
```

- [ ] **Step 4: Rodar testes e clippy no workspace (`GREEN`)**

Run: `cargo test --workspace --locked && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-adapters/src/hermes.rs recursos/mordomo/usuario.toml
git commit -m "fix(adapters): expande til no Hermes e remove redundância em usuario.toml"
```

---

### Task 3: Instrumentação com `tracing` no workspace

**Files:**
- Modify: `Cargo.toml:13-26`
- Modify: `crates/sexta-feira-core/Cargo.toml:11-17`
- Modify: `crates/sexta-feira-adapters/Cargo.toml:11-16`
- Modify: `crates/sexta-feira-cli/Cargo.toml:15-23`
- Modify: `docs/stack.md:10-15`
- Modify: `crates/sexta-feira-core/src/definicao.rs`
- Modify: `crates/sexta-feira-core/src/plano.rs`
- Modify: `crates/sexta-feira-adapters/src/hermes.rs`
- Modify: `crates/sexta-feira-cli/src/main.rs:52-68`

**Interfaces:**
- Consumes: `tracing = "0.1.44"`.
- Produces: eventos `tracing::debug!` em `core`, `adapters` e `cli`; filtro padrão `SEXTA_FEIRA_LOG` alterado de `"info"` para `"warn"` em `main.rs`.

- [ ] **Step 1: Adicionar `tracing` aos `Cargo.toml` e atualizar `docs/stack.md`**

Em `Cargo.toml` (`[workspace.dependencies]`): `tracing = "0.1.44"`.
Em `crates/sexta-feira-{core,adapters,cli}/Cargo.toml`: `tracing.workspace = true`.
Em `docs/stack.md`: `| Logs | \`tracing\`, \`tracing-subscriber\` | em uso |`.

- [ ] **Step 2: Escrever teste falhando em `crates/sexta-feira-cli/tests/cli.rs`**

```rust
#[test]
fn logs_tracing_respeitam_sexta_feira_log_e_nao_sujam_stderr_por_padrao() {
    let padrao = Command::cargo_bin("sexta-feira")
        .expect("binario")
        .env_remove("SEXTA_FEIRA_LOG")
        .arg("harnesses")
        .assert()
        .success();
    let stderr_padrao = String::from_utf8(padrao.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr_padrao.trim().is_empty());

    let debug = Command::cargo_bin("sexta-feira")
        .expect("binario")
        .env("SEXTA_FEIRA_LOG", "debug")
        .arg("harnesses")
        .assert()
        .success();
    let stderr_debug = String::from_utf8(debug.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr_debug.contains("DEBUG"));
}
```

- [ ] **Step 3: Rodar o teste para confirmar falha (`RED`)**

Run: `cargo test -p sexta-feira-cli --test cli logs_tracing`
Expected: FAIL (`stderr_debug` não contém `DEBUG`).

- [ ] **Step 4: Ajustar filtro padrão para `"warn"` e adicionar `tracing::debug!`**

- Em `main.rs`: `EnvFilter::try_from_env("SEXTA_FEIRA_LOG").unwrap_or_else(|_| EnvFilter::new("warn"))` e `tracing::debug!` no dispatch dos subcomandos.
- Em `definicao.rs` (`DefinicaoDoMordomo::carregar`), `plano.rs` (`planejar`, `aplicar`) e `hermes.rs` (`resolver_com_env`, `planejar_arquivos`): emitir eventos `tracing::debug!` com caminhos/quantidades relevantes (sem segredos ou conteúdo completo do usuário).

- [ ] **Step 5: Rodar testes e clippy no workspace (`GREEN`)**

Run: `cargo test --workspace --locked && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock docs/stack.md crates/sexta-feira-core/Cargo.toml crates/sexta-feira-adapters/Cargo.toml crates/sexta-feira-cli/Cargo.toml crates/sexta-feira-core/src/definicao.rs crates/sexta-feira-core/src/plano.rs crates/sexta-feira-adapters/src/hermes.rs crates/sexta-feira-cli/src/main.rs crates/sexta-feira-cli/tests/cli.rs
git commit -m "feat: instrumenta core, adapters e cli com tracing"
```

---

### Task 4: Personalização externa, `--destino`, `--json`, `harnesses` e `remover` na CLI

**Files:**
- Modify: `crates/sexta-feira-cli/Cargo.toml:15-24`
- Modify: `crates/sexta-feira-cli/src/main.rs:1-187`
- Modify: `crates/sexta-feira-cli/tests/cli.rs:1-185`

**Interfaces:**
- Consumes: `FontesDaDefinicao`, `DefinicaoDoMordomo::carregar`, `Harness::disponivel`, `Harness::rotulo_status`, `serde`, `serde_json`.
- Produces:
  - `Cli { json: bool, comando: Comando }`
  - `OpcoesAlvo { harness, destino (alias = "hermes-home"), perfil, persona, usuario }`

- [ ] **Step 1: Adicionar `serde` e `serde_json` em `crates/sexta-feira-cli/Cargo.toml`**

```toml
serde.workspace = true
serde_json.workspace = true
```

- [ ] **Step 2: Atualizar/escrever testes falhando em `crates/sexta-feira-cli/tests/cli.rs`**

- Atualizar snapshot `lista_os_harnesses` para o novo formato alinhado com `disponível` / `em breve`.
- Adicionar testes para:
  - `harnesses_com_json_retorna_lista_estruturada`
  - `personalizacao_via_persona_e_usuario_reflete_em_plano_e_falha_se_ausente`
  - `destino_e_alias_hermes_home_funcionam_igualmente`
  - `plano_aplicar_e_verificar_em_json_cobrem_sucesso_pendencia_e_remover` (incluindo saída JSON válida em `stdout` mesmo quando `verificar --json` retorna erro por pendência!).

- [ ] **Step 3: Rodar os testes para confirmar falha (`RED`)**

Run: `cargo test -p sexta-feira-cli --test cli`
Expected: FAIL (`--json`, `--destino`, `--persona`, `--usuario` não existem e snapshot de `harnesses` diverge).

- [ ] **Step 4: Implementar as alterações em `crates/sexta-feira-cli/src/main.rs`**

- `Cli` com `#[arg(long, global = true)] json: bool`.
- `OpcoesAlvo` com `#[arg(long, alias = "hermes-home")] destino: Option<PathBuf>`, `#[arg(long)] persona: Option<PathBuf>`, `#[arg(long)] usuario: Option<PathBuf>`.
- Helper `carregar_contexto(alvo: &OpcoesAlvo)` montando `FontesDaDefinicao::padrao_do_ambiente()` com overrides explícitos.
- DTOs privados `#[derive(Serialize)]` para `harnesses`, `plano`, `aplicar` e `verificar` usando `snake_case` e `#[serde(skip_serializing_if = "Option::is_none")]`.
- Em `executar_plano`: contar `remover` e exibir `, {remover} remover` quando `remover > 0` no resumo texto (e sempre no `resumo` do JSON).
- Em `executar_verificar`: no modo texto, se `!resultado.sincronizado`, escrever dica `Dica: execute \`sexta-feira aplicar {}\` para sincronizar.` em `stderr` antes do `bail!`; no modo `--json`, escrever o JSON em `stdout` antes do `bail!`.

- [ ] **Step 5: Rodar toda a suíte de validação local (`GREEN`)**

Run:
```bash
cargo fmt --all --check && \
cargo clippy --workspace --all-targets --locked -- -D warnings && \
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked && \
cargo test --workspace --locked && \
cargo test --doc --workspace --locked
```
Expected: PASS com zero warnings e todos os testes verdes.

- [ ] **Step 6: Commit**

```bash
git add crates/sexta-feira-cli/Cargo.toml crates/sexta-feira-cli/src/main.rs crates/sexta-feira-cli/tests/cli.rs
git commit -m "feat(cli): adiciona --json, --destino, --persona/--usuario e status em harnesses"
```
