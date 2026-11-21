# Plano de Implementação: `README.md` mais bonito e intuitivo

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reescrever o `README.md` para refletir o estado atual do projeto (v0.1.0 + personalização em camadas, `--destino`, `--json`, `harnesses`, CI em macOS) com apresentação visual limpa, diagrama Mermaid e guia prático passo a passo.

**Architecture:** Atualização focada exclusivamente no `README.md`, mantendo todos os links relativos, flags, alvos de release e comportamentos fiéis ao código atual em `main`.

**Tech Stack:** Markdown (GitHub Flavored Markdown + Mermaid).

**Spec:** [docs/superpowers/specs/2026-11-21-readme-intuitivo-design.md](../specs/2026-11-21-readme-intuitivo-design.md)

## Global Constraints

- Documentação em `pt-BR` (`AGENTS.md`).
- Apenas comandos, flags, arquivos e alvos de release reais do repositório (`AGENTS.md`, `docs/visao.md`).
- Trabalho na branch `docs/melhora-readme`, nunca direto na `main` (`AGENTS.md`).

## Review Focus

- **Links relativos válidos:** todos os caminhos apontados no `README.md` (`docs/visao.md`, `docs/stack.md`, `CONTRIBUTING.md`, `AGENTS.md`, `LICENSE`, `recursos/mordomo/persona.toml`, `recursos/mordomo/usuario.toml`) devem existir no repositório.
- **Alinhamento com a CLI real:** comandos (`harnesses`, `plano`, `aplicar`, `verificar`), flags (`--destino`, `--perfil`, `--persona`, `--usuario`, `--forcar`, `--json`) e `SEXTA_FEIRA_LOG` devem corresponder ao que `crates/sexta-feira-cli/src/main.rs` aceita.
- **Alvos de Release exatos:** os três alvos citados (`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `aarch64-apple-darwin`) devem bater com `.github/workflows/cd.yml`.

---

## Mapa de Arquivos

- `README.md` — reescrita completa mantendo precisão técnica e layout intuitivo.

---

### Task 1: Reescrever `README.md` e validar links e comandos

**Files:**
- Modify: `README.md:1-66`

**Interfaces:**
- Consumes: CLI real em `crates/sexta-feira-cli/src/main.rs`, alvos de `.github/workflows/cd.yml`, recursos em `recursos/mordomo/`.
- Produces: `README.md` completo e atualizado.

- [ ] **Step 1: Reescrever `README.md` seguindo o design aprovado**

Incluir:
- Cabeçalho centralizado com título, subtítulo e badges (`CI`, `Release`, `Rust 1.100+`, `Licença MIT`).
- Seção **Por que existe** + diagrama `mermaid` (`flowchart LR`) ilustrando camadas (`recursos/mordomo/*.toml` + `~/.config/sexta-feira/*.toml` + `--persona`/`--usuario`) -> `sexta-feira` (`core` + `adapters`) -> `Hermes` (`disponível`) e futuros harnesses (`em breve`).
- Seção **Início rápido**:
  - Opção 1: baixar binário da Release (`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `aarch64-apple-darwin` + `SHA256SUMS.txt`).
  - Opção 2: rodar/compilar com `cargo`.
  - Fluxo em 4 passos (`harnesses`, `plano`, `aplicar`, `verificar`), destacando que o Hermes usa o perfil isolado `$HERMES_HOME/profiles/sexta-feira/` e preserva memórias existentes em `memories/USER.md`.
- Seção **Personalização sem recompilar**: exemplo de `~/.config/sexta-feira/usuario.toml` e `--persona` / `--usuario`.
- Seção **Referência da CLI**: tabela com comandos, flags (`--destino`, `--perfil`, `--persona`, `--usuario`, `--forcar`, `--json`) e variável `SEXTA_FEIRA_LOG`.
- Seção **Situação do projeto**, **Estrutura do repositório**, **Desenvolvimento** (`just ci` / `cargo`) e **Documentação & Licença**.

- [ ] **Step 2: Validar todos os links relativos, sintaxe e comandos reais da CLI**

Run:
```bash
cargo run -q -p sexta-feira-cli -- --help && \
cargo run -q -p sexta-feira-cli -- harnesses && \
test -f docs/visao.md && test -f docs/stack.md && test -f CONTRIBUTING.md && test -f AGENTS.md && test -f LICENSE && test -f recursos/mordomo/persona.toml && test -f recursos/mordomo/usuario.toml && \
cargo test --workspace --locked
```
Expected: PASS com todos os arquivos existentes e `50` testes passando.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: torna o README.md mais intuitivo e visual"
```
