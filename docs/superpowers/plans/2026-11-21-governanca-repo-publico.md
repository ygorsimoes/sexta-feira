# Plano de Implementação: Governança e Workflows para Repositório Público

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (or superpowers:subagent-driven-development) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ajustar o workflow de PRs para suportar contribuições de forks sem falhar nas labels, adicionar `SECURITY.md` e `CODE_OF_CONDUCT.md` em `pt-BR`, e alinhar `docs/visao.md` à licença aberta MIT antes de publicar a release `v0.1.0`.

**Architecture:** Mudanças exclusivamente documentais e de condição de workflow GitHub Actions em `.github/workflows/pr.yml` (mantendo `pull_request` sem elevar privilégios). Nenhuma alteração em código Rust.

**Tech Stack:** GitHub Actions YAML, Markdown (`pt-BR`), Ruby YAML parser e `cargo test --workspace --locked` para verificação local, seguido de `CI`, `Título da PR` e `zizmor` no GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-11-21-governanca-repo-publico-design.md`

---

### Task 1: Evitar falha do job `labels` em PRs vindas de forks

**Files:**
- Modify: `.github/workflows/pr.yml:32-36`

- [ ] **Step 1: Atualizar condição `if` do job `labels` em `.github/workflows/pr.yml`**

Substituir:
```yaml
    # As PRs do Dependabot rodam com token somente de leitura e já chegam com label.
    if: github.event.pull_request.user.login != 'dependabot[bot]'
```
Por:
```yaml
    # Dependabot e PRs de forks rodam com token somente de leitura; evita falha de permissão no labeler.
    if: github.event.pull_request.user.login != 'dependabot[bot]' && !github.event.pull_request.head.repo.fork
```

- [ ] **Step 2: Validar sintaxe YAML do workflow**

Run: `ruby -ryaml -e "YAML.safe_load(File.read('.github/workflows/pr.yml'))"`
Expected: saída vazia e exit code `0`.

---

### Task 2: Criar `SECURITY.md` em `pt-BR`

**Files:**
- Create: `SECURITY.md`

- [ ] **Step 1: Escrever `SECURITY.md` na raiz do repositório**

Conteúdo com versões suportadas (`0.x` / `main`), canais privados de reporte (GitHub Private Vulnerability Reporting em primeiro lugar e `contato.ygorsimoes@gmail.com` como alternativa), escopo e prazo esperado de resposta (até 72h, divulgação coordenada).

---

### Task 3: Criar `CODE_OF_CONDUCT.md` em `pt-BR`

**Files:**
- Create: `CODE_OF_CONDUCT.md`

- [ ] **Step 1: Escrever `CODE_OF_CONDUCT.md` na raiz do repositório**

Conteúdo baseado no Contributor Covenant v2.1 em `pt-BR`, indicando `contato.ygorsimoes@gmail.com` para relatos de violações e critérios de aplicação.

---

### Task 4: Alinhar `Público` em `docs/visao.md`

**Files:**
- Modify: `docs/visao.md:20`

- [ ] **Step 1: Atualizar linha `Público` na tabela da seção 2**

Substituir:
```markdown
| Público | Uso pessoal (um único usuário) |
```
Por:
```markdown
| Público | Opinativo para fluxo de uso pessoal, distribuído em código aberto (MIT) para adoção e contribuições |
```

---

### Task 5: Verificação, commit, PR e acompanhamento de CI

**Files:**
- Stage: `docs/superpowers/specs/2026-11-21-governanca-repo-publico-design.md`, `docs/superpowers/plans/2026-11-21-governanca-repo-publico.md`, `.github/workflows/pr.yml`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `docs/visao.md`

- [ ] **Step 1: Rodar validações locais**

Run:
```bash
ruby -ryaml -e "YAML.safe_load(File.read('.github/workflows/pr.yml'))" && cargo fmt --all -- --check && cargo test --workspace --locked
```
Expected: exit code `0` em todos os comandos.

- [ ] **Step 2: Fazer commit na branch `chore/governanca-repositorio-publico`, push e abrir PR (`Closes #21`)**

- [ ] **Step 3: Acompanhar os checks de CI da PR (`gh pr checks --watch`)**
