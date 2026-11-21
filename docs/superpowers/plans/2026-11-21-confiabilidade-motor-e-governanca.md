# Plano de Implementação: Confiabilidade do motor declarativo e governança

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Atualizar a governança do repositório (`AGENTS.md`, `CLAUDE.md`, `Cargo.toml`) e eliminar os cinco pontos frágeis identificados no motor declarativo `crates/sexta-feira-core/src/plano.rs` em uma PR única.

**Architecture:** As mudanças de código ficam isoladas em `crates/sexta-feira-core/src/plano.rs`, mantendo inalteradas as interfaces públicas da crate (`planejar`, `aplicar`, `verificar` e tipos associados). Toda a lógica nova (escrita atômica por arquivo temporário + rename, rotação de `.bak`, recuperação e persistência parcial de `.sexta-feira-estado.toml`, normalização lexical de chaves, validação estrita de delimitadores e diff por LCS) usa apenas `std`.

**Tech Stack:** Rust edição 2024 (`1.100.0`), `std`, `serde`, `toml`, `thiserror`, `cargo test` / `cargo clippy` / `cargo fmt` / `cargo doc`.

**Spec:** [docs/superpowers/specs/2026-11-21-confiabilidade-motor-e-governanca-design.md](../specs/2026-11-21-confiabilidade-motor-e-governanca-design.md)

## Global Constraints

- Rust edição 2024, `rust-version = "1.100"`, `unsafe_code = "forbid"`, `missing_docs = "warn"`, `clippy::pedantic`, `unwrap_used` e `expect_used` proibidos fora de testes (`Cargo.toml`).
- Documentação, commits, comentários e identificadores de domínio em `pt-BR` (`AGENTS.md`).
- Nenhuma dependência externa adicionada ao `Cargo.toml`; tudo implementado com `std` (`docs/stack.md`).
- Todo trabalho ocorre na branch `fix/confiabilidade-motor-e-governanca`, nunca na `main` (`AGENTS.md`).
- TDD obrigatório em todas as alterações de código: escrever teste que falha (`RED`), confirmar falha, implementar código mínimo (`GREEN`), confirmar suíte verde e refatorar mantendo tudo verde (`test-driven-development`).

## Review Focus

- **Backup numerado em múltiplos `--forcar`:** aplicar `--forcar` duas vezes sobre divergências sucessivas deve preservar `SOUL.md.bak` e criar `SOUL.md.bak.1`, nunca sobrescrever o backup anterior.
- **Recuperação de `.sexta-feira-estado.toml` ausente quando arquivo bate:** se `SOUL.md` já tem o conteúdo exato esperado e `.sexta-feira-estado.toml` foi apagado, `aplicar` deve recriar a entrada de estado para que uma mudança futura em `SOUL.md` seja tratada como `Atualizar`, não `Conflito`.
- **Persistência do estado em falha parcial:** se o arquivo 1 (`ArquivoIntegral`) é gravado e o arquivo 2 falha em `aplicar` (ex.: destino é um diretório), `.sexta-feira-estado.toml` ainda assim deve registrar o arquivo 1 já gravado.
- **Marcadores duplicados ou texto colado sem `\n` em `mesclar_bloco`:** duas ocorrências de `inicio` ou `fim` devem falhar com `ErroDePlano::DelimitadoresInvalidos`; prefixo sem `\n` final antes de `inicio` deve ganhar `\n` separador sem corromper o marcador.
- **Linhas duplicadas e reordenação em `gerar_resumo_diff`:** remover uma de duas linhas idênticas ou inverter a ordem de duas linhas deve aparecer explicitamente com `-` e `+` no `resumo_diff`.

---

## Mapa de Arquivos

- `AGENTS.md` — adiciona regra explícita exigindo o fluxo das skills do Superpowers.
- `CLAUDE.md` — substituído por symlink relativo para `AGENTS.md`.
- `Cargo.toml` — atualiza `rust-version = "1.97"` para `rust-version = "1.100"`.
- `crates/sexta-feira-core/src/plano.rs` — escrita atômica, rotação de backup, recuperação/persistência resiliente do estado, normalização de `chave_estado`, validação estrita em `mesclar_bloco`, diff por LCS em `gerar_resumo_diff` e testes unitários correspondentes.

---

### Task 1: Atualizar governança (`AGENTS.md`, `CLAUDE.md`) e MSRV (`Cargo.toml`)

**Files:**
- Modify: `AGENTS.md:35-46`
- Modify: `CLAUDE.md` (substituir arquivo regular por symlink para `AGENTS.md`)
- Modify: `Cargo.toml:8`

**Interfaces:**
- Não altera interfaces Rust. Garante que `CLAUDE.md -> AGENTS.md` e `Cargo.toml` tenha `rust-version = "1.100"`.

- [ ] **Step 1: Atualizar `AGENTS.md` com a obrigatoriedade das skills do Superpowers**

Na seção `## Regras` de `AGENTS.md`, adicionar a regra explícita de uso do fluxo Superpowers (`using-superpowers`, `brainstorming`, `writing-plans`, `test-driven-development`, `verification-before-completion`).

- [ ] **Step 2: Substituir `CLAUDE.md` por symlink para `AGENTS.md`**

Executar:
```bash
rm CLAUDE.md && ln -s AGENTS.md CLAUDE.md
```
Verificar com `ls -la CLAUDE.md` e `diff -u AGENTS.md CLAUDE.md` (sem saída).

- [ ] **Step 3: Alinhar `rust-version` em `Cargo.toml`**

Alterar `rust-version = "1.97"` para `rust-version = "1.100"` em `Cargo.toml`.

- [ ] **Step 4: Validar build e metadados do workspace**

Run: `cargo check --workspace --locked`
Expected: PASS sem avisos ou erros.

- [ ] **Step 5: Commit**

```bash
git add AGENTS.md CLAUDE.md Cargo.toml
git commit -m "chore(docs): exige Superpowers no AGENTS.md, unifica CLAUDE.md e alinha rust-version"
```

---

### Task 2: Escrita atômica, backup numerado, recuperação e persistência parcial do estado

**Files:**
- Modify: `crates/sexta-feira-core/src/plano.rs:197-252` (`aplicar`)
- Modify: `crates/sexta-feira-core/src/plano.rs:443-457` (`gravar_arquivo`)
- Modify: `crates/sexta-feira-core/src/plano.rs:483-485` (`caminho_backup`)
- Test: `crates/sexta-feira-core/src/plano.rs` (`mod tests`)

**Interfaces:**
- Mantém as assinaturas públicas `aplicar(plano: &PlanoDeSetup, forcar: bool) -> Result<ResultadoDaAplicacao, ErroDePlano>`, `planejar` e `verificar`.

- [ ] **Step 1: Escrever os testes que falham (`RED`)**

Adicionar três testes unitários em `crates/sexta-feira-core/src/plano.rs`:
1. `aplicar_com_forcar_nao_sobrescreve_backup_anterior`: aplica `v1`, edita `SOUL.md` (`"manual 1"`), aplica `--forcar` (cria `SOUL.md.bak`), edita novamente (`"manual 2"`), aplica `--forcar` novamente e verifica que `SOUL.md.bak` preserva `"manual 1"` e `SOUL.md.bak.1` contém `"manual 2"`.
2. `aplicar_recupera_estado_quando_arquivo_integral_ja_bate_mas_estado_foi_perdido`: aplica `v1`, remove `.sexta-feira-estado.toml`, roda `planejar` + `aplicar` para `v1` (que é `SemAlteracoes`), verifica que `.sexta-feira-estado.toml` foi recriado, e em seguida planeja `v2` confirmando que resulta em `OperacaoPlanejada::Atualizar` (não `Conflito`).
3. `aplicar_persiste_estado_parcial_quando_falha_no_meio`: cria plano com 2 arquivos (`a.md` `ArquivoIntegral` para criar e `dir_bloqueado` onde já existe um diretório impedindo escrita em `dir_bloqueado`), chama `aplicar(&plano, false)` esperando `Err(ErroDePlano::Io { .. })`, e verifica que `a.md` foi criado e `.sexta-feira-estado.toml` existe e registra o checksum de `a.md`.

- [ ] **Step 2: Rodar os novos testes para confirmar falha (`RED`)**

Run: `cargo test -p sexta-feira-core`
Expected: FAIL nos 3 novos testes (backup sobrescrito, estado não recriado em `SemAlteracoes`, estado não salvo em erro parcial).

- [ ] **Step 3: Implementar `gravar_arquivo` atômico, `caminho_backup` incremental e persistência resiliente em `aplicar`**

1. Em `gravar_arquivo(caminho: &Path, conteudo: &str) -> Result<(), ErroDePlano>`:
   - Criar diretório pai se necessário.
   - Gerar arquivo temporário irmão no mesmo diretório (`contador atômico` + `std::process::id()`), gravar `conteudo` no temporário; em erro, remover o temporário (melhor esforço) e retornar `ErroDePlano::Io`.
   - Executar `fs::rename(&temp, caminho)`, removendo o temporário se o rename falhar.
2. Em `caminho_backup(caminho: &Path) -> PathBuf`:
   - Testar `<caminho>.bak`; se `!candidato.exists()`, retornar. Caso contrário, testar `<caminho>.bak.1`, `<caminho>.bak.2`, ... até encontrar caminho inexistente.
3. Em `aplicar(plano: &PlanoDeSetup, forcar: bool) -> Result<ResultadoDaAplicacao, ErroDePlano>`:
   - No braço `OperacaoPlanejada::SemAlteracoes`: se `acao.estrategia == EstrategiaDeEscrita::ArquivoIntegral`, verificar se `estado.arquivos_integrais.get(&chave) != Some(&checksum_desejado)`; se divergir ou estiver ausente, inserir e marcar `alterou_estado = true`.
   - Executar o loop de gravação dentro de um bloco/helper ou capturar o `Result`; se houver erro durante o loop e `alterou_estado` for `true`, chamar `let _ = salvar_estado(&plano.arquivo_estado, &estado);` antes de propagar o erro original.

- [ ] **Step 4: Rodar os testes para verificar que passam (`GREEN`)**

Run: `cargo test -p sexta-feira-core`
Expected: PASS em todos os testes do `sexta-feira-core`.

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-core/src/plano.rs
git commit -m "fix(core): torna gravação atômica, preserva backups e recupera estado parcial"
```

---

### Task 3: Estabilizar `chave_estado` para caminhos relativos e absolutos

**Files:**
- Modify: `crates/sexta-feira-core/src/plano.rs:474-481` (`chave_estado`)
- Test: `crates/sexta-feira-core/src/plano.rs` (`mod tests`)

**Interfaces:**
- Mantém `fn chave_estado(caminho: &Path, raiz: Option<&Path>) -> String`.

- [ ] **Step 1: Escrever teste que falha (`RED`)**

Adicionar teste `chave_estado_e_estavel_entre_caminho_relativo_e_absoluto` em `plano.rs`:
- Usa `DirTemporario`, constrói `estado` absoluto (`dir.path().join(".sexta-feira-estado.toml")`) e `soul` com componente redundante (`dir.path().join("sub/../SOUL.md")`), aplica com o caminho normalizado e depois planeja/verifica misturando caminho equivalente (ex.: relativo ao `current_dir` ou com `./` / `..`), confirmando que a chave gerada é `"SOUL.md"` e não dispara `Conflito`.

- [ ] **Step 2: Rodar o teste para confirmar falha (`RED`)**

Run: `cargo test -p sexta-feira-core chave_estado_e_estavel`
Expected: FAIL porque `strip_prefix` não colapsa `.` / `..` nem absolutiza caminhos relativos antes da comparação.

- [ ] **Step 3: Implementar normalização lexical + resolução de caminho relativo em `chave_estado`**

Implementar helper `normalizar_caminho(caminho: &Path) -> PathBuf` que:
- Se `caminho.is_relative()` e `std::env::current_dir()` for `Ok(cwd)`, prefixa `cwd`.
- Percorre `components()` resolvendo `Component::CurDir` (ignora) e `Component::ParentDir` (faz `pop()` quando o topo é `Normal`).
Em `chave_estado`, normalizar `caminho` e `base` antes de `strip_prefix`.

- [ ] **Step 4: Rodar os testes para verificar que passam (`GREEN`)**

Run: `cargo test -p sexta-feira-core`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-core/src/plano.rs
git commit -m "fix(core): normaliza caminhos ao calcular chave no arquivo de estado"
```

---

### Task 4: Validação estrita de delimitadores duplicados e separação de linha em `mesclar_bloco`

**Files:**
- Modify: `crates/sexta-feira-core/src/plano.rs:379-420` (`mesclar_bloco`)
- Test: `crates/sexta-feira-core/src/plano.rs` (`mod tests`)

**Interfaces:**
- Mantém `fn mesclar_bloco(existente: Option<&str>, bloco_interno: &str, inicio: &'static str, fim: &'static str, caminho: &Path) -> Result<String, ErroDePlano>`.

- [ ] **Step 1: Escrever testes que falham (`RED`)**

Adicionar dois testes em `plano.rs`:
1. `bloco_delimitado_rejeita_marcadores_duplicados`: grava arquivo contendo duas ocorrências de `inicio` (ou duas de `fim`) com `inicio` antes de `fim`, chama `planejar`, espera `Err(ErroDePlano::DelimitadoresInvalidos { .. })`.
2. `bloco_delimitado_separa_linha_quando_prefixo_nao_termina_com_quebra`: grava arquivo com `"Prefixo sem quebra<!-- sexta-feira:inicio -->\nantigo\n<!-- sexta-feira:fim -->\n"`, aplica novo bloco e verifica que `inicio` aparece em nova linha após `"Prefixo sem quebra\n"`.

- [ ] **Step 2: Rodar os testes para confirmar falha (`RED`)**

Run: `cargo test -p sexta-feira-core bloco_delimitado_`
Expected: FAIL nos 2 novos testes.

- [ ] **Step 3: Implementar contagem exata de marcadores e separação de linha em `mesclar_bloco`**

- Contar `texto.matches(inicio).count()` e `texto.matches(fim).count()`.
- Se `(0, 0)`: manter comportamento atual.
- Se `(1, 1)` com `let Some(i) = texto.find(inicio)` e `let Some(f) = texto.find(fim)` e `i + inicio.len() <= f`:
  - `let antes = &texto[..i];`
  - Se `!antes.is_empty() && !antes.ends_with('\n')`, inserir `\n` entre `antes` e `bloco_completo`.
- Qualquer outro caso (duplicados, parcial, invertido ou sobreposto): retornar `Err(ErroDePlano::DelimitadoresInvalidos { ... })`.

- [ ] **Step 4: Rodar os testes para verificar que passam (`GREEN`)**

Run: `cargo test -p sexta-feira-core`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-core/src/plano.rs
git commit -m "fix(core): rejeita delimitadores duplicados e garante quebra antes do bloco"
```

---

### Task 5: Diff preciso por sequência e multiplicidade via LCS em `gerar_resumo_diff`

**Files:**
- Modify: `crates/sexta-feira-core/src/plano.rs:503-523` (`gerar_resumo_diff`)
- Test: `crates/sexta-feira-core/src/plano.rs` (`mod tests`)

**Interfaces:**
- Mantém `fn gerar_resumo_diff(antes: &str, depois: &str) -> String`.

- [ ] **Step 1: Escrever testes que falham (`RED`)**

Adicionar dois testes em `plano.rs`:
1. `resumo_diff_detecta_remocao_de_linha_duplicada`: `antes = "a\nb\na\n"`, `depois = "a\nb\n"` → `gerar_resumo_diff` deve conter `"- a"`.
2. `resumo_diff_detecta_reordenacao_de_linhas`: `antes = "linha 1\nlinha 2\n"`, `depois = "linha 2\nlinha 1\n"` → `gerar_resumo_diff` deve conter tanto `"- "` quanto `"+ "` (não o fallback `"~ alteração de formatação"`).

- [ ] **Step 2: Rodar os testes para confirmar falha (`RED`)**

Run: `cargo test -p sexta-feira-core resumo_diff_`
Expected: FAIL nos 2 novos testes.

- [ ] **Step 3: Implementar LCS por linha em Rust puro**

Dividir `antes` e `depois` em `Vec<&str>` via `.lines()`, construir a tabela DP de LCS (`vec![vec![0usize; m + 1]; n + 1]`) de trás para frente (`(0..n).rev()`, `(0..m).rev()`) para percorrer na ordem natural do arquivo emitindo `- linha` e `+ linha` para elementos fora do LCS. Manter em helper privado curto e dentro das regras de `clippy::pedantic` (ex.: sem indexação arriscada e sem funções muito longas).

- [ ] **Step 4: Rodar os testes e toda a suíte + lints + docs**

Run:
```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked && cargo test --workspace --locked
```
Expected: PASS completo em todas as crates, sem avisos nem snapshots quebrados.

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-core/src/plano.rs
git commit -m "fix(core): calcula resumo de diff por LCS preservando ordem e duplicatas"
```
