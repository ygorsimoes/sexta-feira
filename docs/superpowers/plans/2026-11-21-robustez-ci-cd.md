# Plano de Implementação: Robustez de CI/CD (`ci.yml` e `cd.yml`)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Executar os testes do CI em matriz Linux/macOS (`ubuntu-latest` e `macos-latest`) e adicionar cache Rust + smoke test do binário compilado no workflow de CD.

**Architecture:** Alterações isoladas em `.github/workflows/ci.yml` e `.github/workflows/cd.yml`, preservando `permissions` mínimas, fixação de actions por SHA com comentário de versão e passagem de entradas por `env:` para conformidade com `zizmor`.

**Tech Stack:** GitHub Actions, `Swatinem/rust-cache`, `cargo`, `zizmor` (validado no CI).

**Spec:** [docs/superpowers/specs/2026-11-21-robustez-ci-cd-design.md](../specs/2026-11-21-robustez-ci-cd-design.md)

## Global Constraints

- Actions nos workflows são fixadas por SHA, com a versão em comentário (`AGENTS.md`).
- Toda alteração deve passar pelo `zizmor --offline .github` e pelos checks da PR (`AGENTS.md`).
- Trabalho na branch `ci/robustez-ci-cd`, nunca direto na `main` (`AGENTS.md`).

## Review Focus

- **Job agregador `ok` no `ci.yml` com matriz no `test`:** `needs: [lint, test, deny, repo]` deve continuar bloqueando merge se apenas `macos-latest` falhar (`fail-fast: false` garante diagnóstico nos dois SOs sem mascarar falha no agregador).
- **Smoke test do `cd.yml` com `TARGET` e sem interpolação direta:** passar `TARGET: ${{ matrix.target }}` via `env:` e executar `"target/${TARGET}/release/sexta-feira"` com `--version` e `--json harnesses` sem acionar alertas do `zizmor` (`template-injection`).
- **Cache no `cd.yml` isolado por alvo (`key: cd-${{ matrix.target }}`):** evitar colisão de artefatos entre `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` e `aarch64-apple-darwin`.

---

## Mapa de Arquivos

- `.github/workflows/ci.yml` — matriz `os: [ubuntu-latest, macos-latest]` no job `test`.
- `.github/workflows/cd.yml` — `Swatinem/rust-cache` e passo de smoke test pós-compilação.

---

### Task 1: Matriz Linux/macOS no `ci.yml` e cache + smoke test no `cd.yml`

**Files:**
- Modify: `.github/workflows/ci.yml:45-64`
- Modify: `.github/workflows/cd.yml:52-66`

**Interfaces:**
- Consumes: `Swatinem/rust-cache@d1527c8f67ef9af357290475d4af4b0fab8f12e0 # v2.9.3`
- Produces: workflows `.github/workflows/ci.yml` e `.github/workflows/cd.yml` válidos e compatíveis com `zizmor`.

- [ ] **Step 1: Atualizar job `test` em `.github/workflows/ci.yml`**

Adicionar `strategy` com `fail-fast: false` e `matrix.os: [ubuntu-latest, macos-latest]`, nomeando o job `Testes (${{ matrix.os }})` e usando `runs-on: ${{ matrix.os }}`.

- [ ] **Step 2: Atualizar job `build` em `.github/workflows/cd.yml`**

Após `Instalar toolchain e target Rust`, adicionar `Swatinem/rust-cache@d1527c8f67ef9af357290475d4af4b0fab8f12e0 # v2.9.3` com `key: cd-${{ matrix.target }}`; após `Compilar binário de release`, adicionar `Smoke test do binário` executando `"$bin" --version` e `"$bin" --json harnesses` com `TARGET` em `env:`.

- [ ] **Step 3: Validar sintaxe YAML dos workflows e suíte local**

Run:
```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml')); yaml.safe_load(open('.github/workflows/cd.yml'))" && \
cargo test --workspace --locked
```
Expected: PASS (exit 0, workflows válidos em YAML e suíte local verde).

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml .github/workflows/cd.yml
git commit -m "ci: testa macOS no CI e adiciona cache e smoke test no CD"
```
