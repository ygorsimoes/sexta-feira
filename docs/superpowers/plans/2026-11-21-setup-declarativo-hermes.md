# Plano de Implementação: Setup declarativo da Sexta-feira para o Hermes (Recorte 1 — Persona)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implementar o motor declarativo de setup no `sexta-feira-core`, mover o catálogo de harnesses para `sexta-feira-adapters` com o adapter do Hermes (gerando `SOUL.md` e `memories/USER.md` no perfil isolado `sexta-feira`), expor os subcomandos `plano`, `aplicar` e `verificar` na CLI e atualizar toda a documentação do repositório.

**Architecture:** `sexta-feira-core` concentra o modelo agnóstico do mordomo (`DefinicaoDoMordomo`), os ativos padrão embutidos de `recursos/mordomo/`, o contrato `AdaptadorDeSetup` e o planejador/executor declarativo (`PlanoDeSetup`, estado `.sexta-feira-estado.toml`, estratégia integral vs. bloco delimitado). `sexta-feira-adapters` detém `Harness` e o `AdaptadorHermes`, que traduz a definição para `$HERMES_HOME/profiles/<perfil>/SOUL.md` e `memories/USER.md`. `sexta-feira-cli` roteia `harnesses`, `plano`, `aplicar` e `verificar` sem conter regras de negócio.

**Tech Stack:** Rust edição 2024 (`1.100.0`), `serde`, `toml`, `thiserror`, `anyhow`, `clap`, `tracing-subscriber`, `assert_cmd`, `insta`, `cargo-nextest`, `clippy`, `rustfmt`, `cargo-deny`, `cargo-machete`, `zizmor`.

**Spec:** [docs/superpowers/specs/2026-11-21-setup-declarativo-hermes-design.md](../specs/2026-11-21-setup-declarativo-hermes-design.md)

## Global Constraints

- Rust edição 2024, `unsafe_code = "forbid"`, `missing_docs = "warn"`, `clippy::pedantic`, `unwrap_used` e `expect_used` proibidos fora de testes (`Cargo.toml`).
- Erros com `thiserror` nas bibliotecas e `anyhow` na CLI; documentação, mensagens ao usuário e identificadores de domínio em `pt-BR` (`AGENTS.md`).
- Nada específico de harness entra em `sexta-feira-core` (`AGENTS.md`).
- Apenas bibliotecas listadas em `docs/stack.md` entram no `Cargo.toml`, centralizadas em `[workspace.dependencies]` e com `dep.workspace = true` (`AGENTS.md`).
- Todo trabalho acontece em uma branch criada a partir da `main` (`feat/setup-declarativo-hermes`), nunca commit direto na `main`, com validação final pelo CI da PR (`AGENTS.md`).

## Review Focus

- **`SOUL.md` pré-existente antes da primeira execução do `sexta-feira`** (sem `.sexta-feira-estado.toml`): `planejar` marca `Conflito`; `aplicar` sem `--forcar` aborta sem gravar nada; `aplicar --forcar` cria `SOUL.md.bak` com o conteúdo original e grava o novo.
- **`memories/USER.md` pré-existente com memórias aprendidas pelo Hermes**: com delimitadores, atualiza apenas o trecho interno preservando texto antes/depois; sem delimitadores, insere o bloco no topo preservando integralmente o conteúdo existente abaixo.
- **`--perfil` inválido ou inseguro** (vazio, contendo `/`, `\`, `..`, ou espaços em branco): `ConfiguracaoHermes` rejeita explicitamente com erro tipado antes de resolver caminhos.
- **`HERMES_HOME` vazio ou ausente sem `HOME`**: string vazia em `HERMES_HOME` é ignorada e cai para `HOME/.hermes`; se ambos faltarem sem `--hermes-home`, retorna erro tipado claro em `pt-BR`.
- **Atomicidade na pré-checagem e idempotência**: se o arquivo 1 estiver pronto para `Criar` e o arquivo 2 estiver em `Conflito`, `aplicar` sem `--forcar` aborta antes de criar o arquivo 1; após `aplicar`, `verificar` passa (exit 0) e novo `plano` mostra apenas `SemAlteracoes`.

---

## Mapa de Arquivos

- `Cargo.toml` — adiciona `toml` em `[workspace.dependencies]`.
- `recursos/mordomo/persona.toml` — fonte única da identidade Sexta-feira (J.A.R.V.I.S., `pt-BR`, diretrizes e limites).
- `recursos/mordomo/usuario.toml` — fonte única do perfil do usuário (tratamento `"senhor"`, idioma e estilo).
- `crates/sexta-feira-core/Cargo.toml` — adiciona `toml.workspace = true`.
- `crates/sexta-feira-core/src/lib.rs` — expõe `adaptador`, `definicao`, `plano` (remove `harness`).
- `crates/sexta-feira-core/src/definicao.rs` — `DefinicaoDoMordomo`, `Persona`, `PerfilDoUsuario`, `ErroDeDefinicao`, `definicao_padrao()`.
- `crates/sexta-feira-core/src/plano.rs` — `EstrategiaDeEscrita`, `ArquivoDesejado`, `OperacaoPlanejada`, `AcaoPlanejada`, `PlanoDeSetup`, `ResultadoDaAplicacao`, `ResultadoDaVerificacao`, `ErroDePlano`, `planejar`, `aplicar`, `verificar` e controle de `.sexta-feira-estado.toml`.
- `crates/sexta-feira-core/src/adaptador.rs` — trait `AdaptadorDeSetup` e `ErroDeAdaptador`.
- `crates/sexta-feira-adapters/Cargo.toml` — adiciona `thiserror.workspace = true`.
- `crates/sexta-feira-adapters/src/lib.rs` — expõe `catalogo` e `hermes`.
- `crates/sexta-feira-adapters/src/catalogo.rs` — `Harness` e `HarnessDesconhecido` (movidos do `core`).
- `crates/sexta-feira-adapters/src/hermes.rs` — `ConfiguracaoHermes`, `AdaptadorHermes`, `ErroDeHermes`, renderização de `SOUL.md` (< 80 linhas) e bloco de `memories/USER.md`.
- `crates/sexta-feira-cli/Cargo.toml` — remove `[package.metadata.cargo-machete]`.
- `crates/sexta-feira-cli/src/main.rs` — subcomandos `harnesses`, `plano`, `aplicar`, `verificar`.
- `crates/sexta-feira-cli/tests/cli.rs` — testes E2E com `assert_cmd` e `insta`.
- `AGENTS.md`, `CLAUDE.md`, `README.md`, `docs/stack.md`, `docs/visao.md` — atualização documental final.

---

### Task 1: Preparar branch e mover o catálogo de harnesses para `sexta-feira-adapters`

**Files:**
- Delete: `crates/sexta-feira-core/src/harness.rs`
- Modify: `crates/sexta-feira-core/src/lib.rs`
- Modify: `crates/sexta-feira-adapters/Cargo.toml`
- Create: `crates/sexta-feira-adapters/src/catalogo.rs`
- Modify: `crates/sexta-feira-adapters/src/lib.rs`
- Modify: `crates/sexta-feira-cli/Cargo.toml`
- Modify: `crates/sexta-feira-cli/src/main.rs`

**Interfaces:**
- Produces in `sexta_feira_adapters`: `pub use catalogo::{Harness, HarnessDesconhecido};` (mantendo `Harness::TODOS`, `Harness::id`, `Display`, `FromStr`, `Serialize`, `Deserialize`).

- [ ] **Step 1: Criar a branch a partir da `main`**

Run: `git checkout -b feat/setup-declarativo-hermes`
Expected: `Switched to a new branch 'feat/setup-declarativo-hermes'`

- [ ] **Step 2: Mover `Harness` e `HarnessDesconhecido` para `crates/sexta-feira-adapters/src/catalogo.rs` e ajustar dependências**

Em `crates/sexta-feira-adapters/Cargo.toml`, adicionar `serde.workspace = true` e `thiserror.workspace = true`.
Em `crates/sexta-feira-adapters/src/catalogo.rs`, manter `Harness` e `HarnessDesconhecido` com seus dois testes unitários (`id_e_from_str_sao_inversos` e `identificador_invalido_e_rejeitado`).
Em `crates/sexta-feira-adapters/src/lib.rs`, expor `pub mod catalogo; pub use catalogo::{Harness, HarnessDesconhecido};`.
Remover `crates/sexta-feira-core/src/harness.rs` e limpar `crates/sexta-feira-core/src/lib.rs`.
Em `crates/sexta-feira-cli/Cargo.toml`, remover `[package.metadata.cargo-machete]` (mantendo `sexta-feira-adapters` e `sexta-feira-core`; `clap`, `anyhow`, `tracing-subscriber`).
Em `crates/sexta-feira-cli/src/main.rs`, importar `sexta_feira_adapters::Harness`.

- [ ] **Step 3: Verificar compilação, lints e testes**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked`
Expected: PASS com os 2 testes unitários em `sexta-feira-adapters` e o teste `lista_os_harnesses` em `sexta-feira-cli`.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor(adapters): move catálogo de harnesses para sexta-feira-adapters"
```

---

### Task 2: Criar a fonte única da persona e o módulo `definicao` no `sexta-feira-core`

**Files:**
- Modify: `Cargo.toml`
- Create: `recursos/mordomo/persona.toml`
- Create: `recursos/mordomo/usuario.toml`
- Modify: `crates/sexta-feira-core/Cargo.toml`
- Create: `crates/sexta-feira-core/src/definicao.rs`
- Modify: `crates/sexta-feira-core/src/lib.rs`

**Interfaces:**
- Produces in `sexta_feira_core::definicao`:
  - `pub struct Persona { pub nome: String, pub arquetipo: String, pub idioma: String, pub resumo: String, pub tom: Vec<String>, pub diretrizes: Vec<String>, pub limites: Vec<String> }`
  - `pub struct PerfilDoUsuario { pub tratamento: String, pub idioma: String, pub preferencias: Vec<String> }`
  - `pub struct DefinicaoDoMordomo { pub persona: Persona, pub usuario: PerfilDoUsuario }`
  - `pub enum ErroDeDefinicao` (`#[derive(Debug, thiserror::Error)]`)
  - `impl DefinicaoDoMordomo { pub fn padrao() -> Result<Self, ErroDeDefinicao>; pub fn de_toml(persona_toml: &str, usuario_toml: &str) -> Result<Self, ErroDeDefinicao>; }`

- [ ] **Step 1: Adicionar `toml` em `Cargo.toml` e `crates/sexta-feira-core/Cargo.toml`, e escrever `recursos/mordomo/persona.toml` e `usuario.toml`**

Em `recursos/mordomo/persona.toml`, definir `nome = "Sexta-feira"`, `arquetipo = "J.A.R.V.I.S."`, `idioma = "pt-BR"`, `resumo`, `tom` (calma, formal, objetiva, humor sutil britânico quando oportuno, sem bajulação), `diretrizes` (proatividade com rigor técnico, transparência diante de incertezas, confirmação prévia antes de ações externas ou irreversíveis) e `limites`.
Em `recursos/mordomo/usuario.toml`, definir `tratamento = "senhor"`, `idioma = "pt-BR"` e `preferencias` de comunicação direta e estruturada.

- [ ] **Step 2: Escrever os testes falhando em `crates/sexta-feira-core/src/definicao.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definicao_padrao_carrega_com_valores_esperados() {
        let def = DefinicaoDoMordomo::padrao().expect("definição padrão deve ser válida");
        assert_eq!(def.persona.nome, "Sexta-feira");
        assert_eq!(def.persona.idioma, "pt-BR");
        assert_eq!(def.usuario.tratamento, "senhor");
        assert!(!def.persona.diretrizes.is_empty());
        assert!(!def.persona.limites.is_empty());
    }

    #[test]
    fn rejeita_tratamento_ou_nome_vazio() {
        let persona = r#"
            nome = "   "
            arquetipo = "J.A.R.V.I.S."
            idioma = "pt-BR"
            resumo = "Mordomo pessoal."
            tom = ["formal"]
            diretrizes = ["ser clara"]
            limites = ["confirmar ações externas"]
        "#;
        let usuario = r#"
            tratamento = "senhor"
            idioma = "pt-BR"
            preferencias = ["objetividade"]
        "#;
        assert!(matches!(
            DefinicaoDoMordomo::de_toml(persona, usuario),
            Err(ErroDeDefinicao::CampoObrigatorioVazio { campo: "persona.nome" })
        ));
    }
}
```

- [ ] **Step 3: Rodar o teste para confirmar falha**

Run: `cargo test -p sexta-feira-core definicao`
Expected: FAIL (módulo `definicao` ainda não existe).

- [ ] **Step 4: Implementar `crates/sexta-feira-core/src/definicao.rs` e expor em `src/lib.rs`**

Implementar `DefinicaoDoMordomo::de_toml` usando `toml::from_str`, validar campos obrigatórios não vazios (`persona.nome`, `persona.arquetipo`, `persona.idioma`, `persona.resumo`, `usuario.tratamento`, `usuario.idioma`, além de listas não vazias `tom`, `diretrizes`, `limites`), e `DefinicaoDoMordomo::padrao()` usando `include_str!("../../../recursos/mordomo/persona.toml")` e `include_str!("../../../recursos/mordomo/usuario.toml")`.

- [ ] **Step 5: Rodar testes e lints para confirmar sucesso**

Run: `cargo test -p sexta-feira-core --locked && cargo clippy -p sexta-feira-core --all-targets --locked -- -D warnings`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock recursos/ crates/sexta-feira-core/
git commit -m "feat(core): adiciona definição agnóstica e fonte única da persona"
```

---

### Task 3: Implementar o motor declarativo de plano, diff, estado e aplicação em `sexta-feira-core`

**Files:**
- Create: `crates/sexta-feira-core/src/plano.rs`
- Create: `crates/sexta-feira-core/src/adaptador.rs`
- Modify: `crates/sexta-feira-core/src/lib.rs`

**Interfaces:**
- Produces in `sexta_feira_core::plano`:
  - `pub enum EstrategiaDeEscrita { ArquivoIntegral, BlocoDelimitado { inicio: &'static str, fim: &'static str } }`
  - `pub struct ArquivoDesejado { pub caminho: PathBuf, pub conteudo: String, pub estrategia: EstrategiaDeEscrita }`
  - `pub enum OperacaoPlanejada { Criar, Atualizar, SemAlteracoes, Conflito { motivo: String } }`
  - `pub struct AcaoPlanejada { pub caminho: PathBuf, pub operacao: OperacaoPlanejada, pub conteudo_final: String, pub estrategia: EstrategiaDeEscrita, pub resumo_diff: Option<String> }`
  - `pub struct PlanoDeSetup { pub arquivo_estado: PathBuf, pub acoes: Vec<AcaoPlanejada> }`
  - `pub struct ResultadoDaVerificacao { pub sincronizado: bool, pub itens: Vec<ItemDeVerificacao> }`
  - `pub struct ItemDeVerificacao { pub caminho: PathBuf, pub sincronizado: bool, pub detalhe: String }`
  - `pub enum ErroDePlano`
  - `pub fn planejar(arquivos: Vec<ArquivoDesejado>, arquivo_estado: PathBuf) -> Result<PlanoDeSetup, ErroDePlano>`
  - `pub fn aplicar(plano: &PlanoDeSetup, forcar: bool) -> Result<ResultadoDaAplicacao, ErroDePlano>`
  - `pub fn verificar(arquivos: &[ArquivoDesejado], arquivo_estado: &Path) -> Result<ResultadoDaVerificacao, ErroDePlano>`
- Produces in `sexta_feira_core::adaptador`:
  - `pub enum ErroDeAdaptador`
  - `pub trait AdaptadorDeSetup { fn id(&self) -> &'static str; fn arquivo_estado(&self) -> PathBuf; fn planejar_arquivos(&self, definicao: &DefinicaoDoMordomo) -> Result<Vec<ArquivoDesejado>, ErroDeAdaptador>; }`

- [ ] **Step 1: Escrever os testes falhando em `crates/sexta-feira-core/src/plano.rs` cobrindo todos os cenários do Review Focus**

Incluir testes com diretório temporário RAII em `std::env::temp_dir()`:
1. `planejar_e_aplicar_criam_arquivos_e_segunda_execucao_e_idempotente` — cria `ArquivoIntegral` e `BlocoDelimitado`, aplica, verifica (`sincronizado == true`), replaneja (`SemAlteracoes` em ambos).
2. `detecta_conflito_quando_arquivo_integral_preexistente_nao_tem_estado_e_aborta_sem_forcar` — coloca arquivo 1 inexistente (`Criar`) e arquivo 2 `ArquivoIntegral` pré-existente (`Conflito`); `aplicar(&plano, false)` retorna erro de conflito e **não cria o arquivo 1** (atomicidade da pré-checagem).
3. `aplicar_com_forcar_gera_backup_e_atualiza_estado` — após edição manual de `ArquivoIntegral` (mudando checksum em disco vs. `.sexta-feira-estado.toml`), `aplicar(&plano, true)` salva `<arquivo>.bak` com o conteúdo editado, grava o novo conteúdo e atualiza o estado para ficar sincronizado.
4. `bloco_delimitado_preserva_conteudo_existente_com_ou_sem_marcadores` — testa (a) arquivo existente sem delimitadores (insere bloco no topo e mantém conteúdo antigo abaixo), (b) arquivo existente com delimitadores e conteúdo antes/depois (substitui só o trecho interno), e (c) caso malformado com apenas um marcador (retorna `ErroDePlano::DelimitadoresInvalidos`).

- [ ] **Step 2: Rodar os testes para confirmar falha**

Run: `cargo test -p sexta-feira-core plano`
Expected: FAIL.

- [ ] **Step 3: Implementar `crates/sexta-feira-core/src/plano.rs`, `src/adaptador.rs` e atualizar `src/lib.rs`**

Detalhes chave de implementação:
- Checksum determinístico interno sem crate externa (FNV-1a 64-bit formatado em hexadecimal + tamanho em bytes, ex.: `fnv1a64:<hex>:<len>`), persistido em `.sexta-feira-estado.toml` via `serde` + `toml`.
- Para `ArquivoIntegral`: se existe no disco, compara conteúdo exato com `conteudo_final` — se igual e sem registro no estado, ou se divergente sem estado prévio, marcar `Conflito`? Atenção: se o conteúdo em disco já for **idêntico** ao desejado (ex.: sem estado por perda do arquivo de estado), ou se existir estado batendo: se `disco == desejado`, marcar `SemAlteracoes` (ou se não tiver estado e diferir, `Conflito`; se tiver estado e `checksum(disco) != estado.checksum && disco != desejado`, `Conflito`). Isso torna reexecução limpa e segura!
- Para `BlocoDelimitado { inicio, fim }`: helper puro `mesclar_bloco(existente: Option<&str>, bloco_interno: &str, inicio: &str, fim: &str) -> Result<String, ErroDePlano>`. Normalizar quebra de linha `\n`, garantindo espaçamento limpo entre o bloco e o restante do conteúdo.
- `aplicar(plano, false)` valida `plano.acoes` inteira antes de tocar no sistema de arquivos; se qualquer ação for `OperacaoPlanejada::Conflito`, retorna `Err(ErroDePlano::ConflitoPendente { ... })`.

- [ ] **Step 4: Rodar testes e lints do `core`**

Run: `cargo test -p sexta-feira-core --locked && cargo clippy -p sexta-feira-core --all-targets --locked -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-core/
git commit -m "feat(core): implementa motor declarativo de plano, estado e adaptador"
```

---

### Task 4: Implementar o adapter do Hermes em `sexta-feira-adapters`

**Files:**
- Create: `crates/sexta-feira-adapters/src/hermes.rs`
- Modify: `crates/sexta-feira-adapters/src/lib.rs`

**Interfaces:**
- Produces in `sexta_feira_adapters::hermes`:
  - `pub struct ConfiguracaoHermes { pub hermes_home: PathBuf, pub perfil: String }`
  - `impl ConfiguracaoHermes { pub fn resolver(hermes_home_explicito: Option<&Path>, perfil: Option<&str>) -> Result<Self, ErroDeHermes>; pub fn resolver_com_env(hermes_home_explicito: Option<&Path>, env_hermes_home: Option<&str>, env_home: Option<&str>, perfil: Option<&str>) -> Result<Self, ErroDeHermes>; pub fn diretorio_do_perfil(&self) -> PathBuf; }`
  - `pub struct AdaptadorHermes { ... }` implementing `sexta_feira_core::AdaptadorDeSetup`
  - `pub enum ErroDeHermes` (`#[derive(Debug, thiserror::Error)]`)
  - `pub const PERFIL_PADRAO: &str = "sexta-feira";`
  - `pub const LIMITE_LINHAS_SOUL: usize = 80;`

- [ ] **Step 1: Escrever testes falhando em `crates/sexta-feira-adapters/src/hermes.rs`**

Cobrir:
1. `resolve_hermes_home_por_prioridade` — flag explícita > `HERMES_HOME` não vazia > `HOME/.hermes`; `HERMES_HOME=""` cai para `HOME/.hermes`; ausência de todos retorna `ErroDeHermes::DiretorioBaseIndisponivel`.
2. `rejeita_perfil_invalido` — `""`, `"   "`, `"../fora"`, `"a/b"`, `"a\\b"` retornam `ErroDeHermes::PerfilInvalido`.
3. `gera_soul_enxuto_com_personalidade_jarvis_e_user_com_senhor` — verifica que `SOUL.md` tem `<= 80` linhas, contém `"Sexta-feira"`, `"J.A.R.V.I.S."` e `"pt-BR"`, e que `memories/USER.md` usa `BlocoDelimitado` contendo `"senhor"`.
4. `ciclo_completo_com_motor_declarativo_do_core` — `planejar` → `aplicar` → `verificar` (`sincronizado == true`) → preserva memória prévia em `memories/USER.md`.

- [ ] **Step 2: Rodar testes para confirmar falha**

Run: `cargo test -p sexta-feira-adapters hermes`
Expected: FAIL.

- [ ] **Step 3: Implementar `crates/sexta-feira-adapters/src/hermes.rs` e expor em `src/lib.rs`**

Implementar resolução segura de diretórios/perfil (`ConfiguracaoHermes::resolver_com_env` pura e testável, chamada por `resolver`), renderização concisa de `SOUL.md` (validando `<= LIMITE_LINHAS_SOUL` linhas para não inflar o prompt do Hermes) e do conteúdo de `memories/USER.md`, e implementação de `AdaptadorDeSetup` apontando `arquivo_estado` para `<perfil_dir>/.sexta-feira-estado.toml`.

- [ ] **Step 4: Rodar testes e lints**

Run: `cargo test -p sexta-feira-adapters --locked && cargo clippy -p sexta-feira-adapters --all-targets --locked -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-adapters/
git commit -m "feat(adapters): implementa adaptador declarativo para perfil do Hermes"
```

---

### Task 5: Expor `plano`, `aplicar` e `verificar` na CLI `sexta-feira` e testar ponta a ponta

**Files:**
- Modify: `crates/sexta-feira-cli/src/main.rs`
- Modify: `crates/sexta-feira-cli/tests/cli.rs`

**Interfaces:**
- CLI commands:
  - `sexta-feira harnesses`
  - `sexta-feira plano <harness> [--hermes-home <dir>] [--perfil <nome>]`
  - `sexta-feira aplicar <harness> [--hermes-home <dir>] [--perfil <nome>] [--forcar]`
  - `sexta-feira verificar <harness> [--hermes-home <dir>] [--perfil <nome>]`

- [ ] **Step 1: Escrever testes E2E falhando em `crates/sexta-feira-cli/tests/cli.rs`**

Usando `assert_cmd` + `insta` e diretório temporário RAII:
1. `lista_os_harnesses` (mantido).
2. `plano_hermes_mostra_criacao_dos_arquivos_sem_escrever_no_disco` — snapshot de `plano hermes --hermes-home <tmp>` (normalizando ou usando caminho relativo/filtro no snapshot) e garante que `<tmp>/profiles/sexta-feira` não foi criado.
3. `aplicar_e_verificar_hermes_sao_idempotentes` — `aplicar hermes` → `verificar hermes` (sucesso) → `plano hermes` mostra `SemAlteracoes` (`OK`) → verifica conteúdo de `SOUL.md` e `memories/USER.md`.
4. `aplicar_hermes_preserva_memorias_existentes_e_respeita_conflito_em_soul` — cria `SOUL.md` manual e `memories/USER.md` prévio; `aplicar` sem `--forcar` falha; `aplicar --forcar` cria `SOUL.md.bak` com conteúdo antigo, atualiza `SOUL.md` e preserva as memórias em `memories/USER.md`.
5. `verificar_hermes_falha_quando_nao_aplicado_ou_divergente` — exit code não-zero.
6. `harness_nao_implementado_retorna_erro_claro` — `plano openclaw` falha com mensagem explícita em `pt-BR`.

- [ ] **Step 2: Rodar os testes E2E para confirmar falha**

Run: `cargo test -p sexta-feira-cli --locked`
Expected: FAIL.

- [ ] **Step 3: Implementar os subcomandos em `crates/sexta-feira-cli/src/main.rs`**

Usar `clap` derive (`Subcommand`, `Args`), carregar `DefinicaoDoMordomo::padrao()`, rotear `Harness::Hermes` para `AdaptadorHermes`, rejeitar outros harnesses com erro descritivo em `pt-BR`, e formatar saída limpa e determinística para `plano`, `aplicar` e `verificar`.

- [ ] **Step 4: Rodar os testes E2E e atualizar/validar snapshots**

Run: `INSTA_UPDATE=always cargo test -p sexta-feira-cli --locked` (revisar cuidadosamente os snapshots inline gerados) e depois `INSTA_UPDATE=no cargo test --workspace --locked`.
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-cli/
git commit -m "feat(cli): adiciona subcomandos plano, aplicar e verificar para o Hermes"
```

---

### Task 6: Atualizar documentações, validar todo o CI local e abrir a PR

**Files:**
- Modify: `AGENTS.md`
- Modify: `CLAUDE.md`
- Modify: `README.md`
- Modify: `docs/stack.md`
- Modify: `docs/visao.md`

- [ ] **Step 1: Atualizar `AGENTS.md` e `CLAUDE.md`**

Atualizar a seção `## Estrutura` em `AGENTS.md` (incluindo `recursos/`, refletindo o papel atualizado de `sexta-feira-core` como modelo agnóstico e motor declarativo, e `sexta-feira-adapters` como catálogo e tradutores) e manter `CLAUDE.md` sincronizado com `AGENTS.md` (ou referenciando-o de forma consistente com a convenção do repositório).

- [ ] **Step 2: Atualizar `README.md`, `docs/stack.md` e `docs/visao.md`**

- `README.md`: atualizar a tabela `## Situação` (Recorte 1: modelo do setup e adapter do Hermes implementados para persona e perfil), a árvore `## Estrutura` (incluindo `recursos/`) e os exemplos de uso em `## Desenvolvimento` (`plano`, `aplicar`, `verificar`).
- `docs/stack.md`: mover `toml` de "prevista" para "em uso" (`serde`, `toml` em uso; `serde_json` prevista).
- `docs/visao.md`: atualizar seção 5 (`recursos/mordomo/`) e seção 6 (registrando o status do Recorte 1 implementado para o Hermes: persona `SOUL.md` + `memories/USER.md`).

- [ ] **Step 3: Executar todas as verificações do CI localmente**

Run:
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
INSTA_UPDATE=no cargo nextest run --workspace --locked
cargo machete
cargo deny check
zizmor --offline .github
```
(Ou `just ci`, garantindo que todas passem sem avisos.)
Expected: todos os comandos retornam código 0.

- [ ] **Step 4: Commit, push e abertura da PR**

```bash
git add AGENTS.md CLAUDE.md README.md docs/
git commit -m "docs: atualiza documentação para o setup declarativo do Hermes"
git push -u origin feat/setup-declarativo-hermes
gh pr create --base main --title "feat: implementa setup declarativo do Hermes com persona Sexta-feira" --body-file ...
```
Acompanhar `gh pr checks --watch` até todos os checks da PR passarem.
