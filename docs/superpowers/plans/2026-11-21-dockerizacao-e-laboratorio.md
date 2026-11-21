# Plano de Implementação: Dockerização completa (CI, CLI e laboratório Hermes) via `justfile`

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorganizar toda a infraestrutura Docker e automação do projeto sob `docker/`, `.env.example` na raiz e um `justfile` agrupado na raiz (`desenvolvimento`, `docker`, `cli`, `laboratorio`), cobrindo CI completo em container, build/runtime `musl` da CLI e o laboratório do Hermes Agent, e atualizar a PR #25.

**Architecture:** Um único `docker/compose.yaml` orquestra quatro perfis (`ci`, `cli`, `local`, `dia-zero`) sobre imagens especializadas (`docker/ci/Dockerfile` e `docker/cli/Dockerfile`) e scripts do Hermes (`docker/hermes/bootstrap.sh` e `lab.sh`). O `justfile` na raiz carrega o `.env` da raiz automaticamente (`set dotenv-load`) e expõe comandos curtos de uma palavra para o dia a dia, enquanto `parar`/`resetar` isolam o laboratório Hermes e `destruir` remove todos os recursos Docker do projeto.

**Tech Stack:** `just`, Docker Engine/Compose v2 (`v5.7.1`), `cargo-binstall` (`cargo-nextest`, `cargo-deny`, `cargo-machete`, `zizmor`, `just`), Rust `1.100.0` (`musl`), Alpine `3.21`, Bash, `nousresearch/hermes-agent:latest`.

**Spec:** [docs/superpowers/specs/2026-11-21-dockerizacao-e-laboratorio-design.md](../specs/2026-11-21-dockerizacao-e-laboratorio-design.md)

## Global Constraints

- Tudo (documentação, comentários, mensagens ao usuário e commits) em `pt-BR` (`AGENTS.md`).
- Trabalho realizado na branch `feat/ambiente-local-docker-hermes` da PR #25, nunca direto na `main`.
- Segredos nunca entram no repositório: apenas `.env.example` na raiz é versionado; `.env` permanece ignorado.
- Sem `Makefile`; `justfile` na raiz é o executor oficial.
- Sem alterações no código Rust de `crates/` ou nos workflows `.github/workflows/*.yml`.
- Nunca montar `./target` do macOS dentro de containers Linux; usar volumes nomeados `target_ci` e `target_linux`.

## Review Focus

- **Contexto de build e `.dockerignore`:** `docker/ci/Dockerfile` e `docker/cli/Dockerfile` não devem enviar `/target`, `.git`, `.superpowers` ou `.env` no contexto de build nem incluir segredos nas imagens geradas.
- **Chave OpenRouter com espaços entre aspas (`"   "`) e permissões `0600`:** `ler_chave_openrouter()` em `lab.sh` deve aparar espaços *após* remover aspas para rejeitar chaves em branco, e `bootstrap.sh` deve criar arquivos temporários de `.env` já em modo `0600` via `os.open` (sem janela `0644`) preservando quebra de linha final.
- **Escopo isolado de `parar` / `resetar` vs. `destruir`:** `just parar` e `just resetar` não podem derrubar um container `ci` nem apagar `target_ci`/`cli_dados`/`cargo_registry`; `just destruir` deve remover todos os containers, redes e volumes do projeto Compose (tratando EOF `Ctrl-D` no prompt de confirmação sem falhar antes da hora) e `resetar` não deve engolir erros reais de `docker volume rm`.
- **Validação estrita de argumentos em `cmd_chat` e readiness com `s6-svscan`:** `lab.sh chat` deve rejeitar argumentos extras além do 2º e `aguardar_container()` deve aguardar `s6-svscan` + arquivos `/opt/data/{config.yaml,.env}` antes de rodar `bootstrap.sh`.
- **Consistência de documentação e `justfile`:** todos os comandos listados em `docs/ambiente-local-docker.md`, `README.md`, `AGENTS.md` e `CONTRIBUTING.md` devem existir no `justfile` e passar em `just --fmt --check` dentro do container `ci`.

---

## Mapa de Arquivos

- Create: `.dockerignore` — exclui `/target`, `/dist`, `.git`, `.superpowers`, `.env*` (mantendo `.env.example`).
- Create: `.env.example` — move template de configuração para a raiz.
- Delete: `docker/hermes/.env.example` — substituído pelo `.env.example` na raiz.
- Create: `docker/ci/Dockerfile` — imagem CI/Dev (`rust:1.100.0-slim-bookworm` + `rustfmt`, `clippy`, `musl-tools`, `cargo-nextest`, `cargo-deny`, `cargo-machete`, `zizmor`, `just`).
- Create: `docker/cli/Dockerfile` — multi-stage `musl` da CLI `sexta-feira` + runtime `alpine:3.21` não-root.
- Create: `docker/compose.yaml` — Compose unificado (`ci`, `cli`, `hermes-builder`, `hermes`, `hermes-dia-zero`).
- Delete: `docker/hermes/compose.yaml` — substituído por `docker/compose.yaml`.
- Move/Modify: `docker/hermes/scripts/bootstrap-hermes.sh` → `docker/hermes/bootstrap.sh` — escrita `0600` atômica via `os.open`, sync de chave e marcador `.sexta-feira-lab-bootstrap`.
- Move/Modify: `docker/hermes/scripts/lab.sh` → `docker/hermes/lab.sh` — usa `.env` na raiz, `-p "$PROJETO"`, trim pós-aspas, EOF em `destruir`, validação em `cmd_chat`, `resetar` sem engolir erros e `s6-svscan` readiness.
- Modify: `.gitignore` — adicionar `/dist`.
- Modify: `justfile` — reorganizar com `set dotenv-load`, `[group(...)]` e comandos curtos.
- Modify: `docs/ambiente-local-docker.md`, `README.md`, `AGENTS.md`, `CONTRIBUTING.md` — atualizar árvore, comandos `just` e aviso pré-`aplicar` no Hermes.

---

### Task 1: Raiz (`.dockerignore`, `.env.example`, `.gitignore`) e Dockerfiles (`ci` + `cli`)

**Files:**
- Create: `.dockerignore`
- Create: `.env.example`
- Delete: `docker/hermes/.env.example`
- Modify: `.gitignore:1-7`
- Create: `docker/ci/Dockerfile`
- Create: `docker/cli/Dockerfile`

**Interfaces:**
- Consumes:
  - `rust-toolchain.toml` (`1.100.0`), `Cargo.toml`, `Cargo.lock`, `deny.toml`, `rustfmt.toml`.
- Produces:
  - Imagem `ci` com `cargo`, `rustfmt`, `clippy`, `cargo-nextest`, `cargo-deny`, `cargo-machete`, `zizmor`, `just` e target `$(uname -m)-unknown-linux-musl`.
  - Imagem `cli` multi-stage expondo `/usr/local/bin/sexta-feira` estático `musl` sob usuário `10000:10000` (`HOME=/dados`, `HERMES_HOME=/dados/.hermes`, `ENTRYPOINT ["sexta-feira"]`) e stage exportável `export-bin` (ou `COPY` via `--output`) para `just docker-build`.

- [ ] **Step 1: Atualizar `.gitignore`, criar `.dockerignore` e mover `.env.example` para a raiz**

- Adicionar `/dist` ao `.gitignore`.
- Criar `.dockerignore` ignorando `/target`, `/dist`, `.git`, `.superpowers`, `.env`, `.env.*`, `!.env.example`, `.DS_Store`, `spikes`.
- Mover `docker/hermes/.env.example` para `.env.example` na raiz mantendo as variáveis (`OPENROUTER_API_KEY`, `HERMES_MODEL`, `HERMES_IMAGE`, `HERMES_DASHBOARD`, `HERMES_GATEWAY_PORTA`, `HERMES_DASHBOARD_PORTA`).

- [ ] **Step 2: Criar `docker/ci/Dockerfile` e testar build + ferramentas instaladas**

Implementar `docker/ci/Dockerfile`:
- Estágio `FROM ghcr.io/cargo-bins/cargo-binstall:latest AS binstall`.
- `FROM rust:1.100.0-slim-bookworm` copiando `cargo-binstall` e certs, instalando `musl-tools`, `git`, `ca-certificates`, `pkg-config`, `libssl-dev`, `rustup component add rustfmt clippy`, `rustup target add "$(uname -m)-unknown-linux-musl"`, e `cargo-binstall -y cargo-nextest cargo-deny cargo-machete zizmor just`.
- `WORKDIR /workspace`.

Run:
```bash
docker build -f docker/ci/Dockerfile -t sexta-feira-ci:test .
docker run --rm sexta-feira-ci:test sh -c 'rustc --version && cargo nextest --version && cargo deny --version && cargo machete --version && zizmor --version && just --version'
```
Expected: build conclui com sucesso e todas as 6 ferramentas imprimem suas versões.

- [ ] **Step 3: Criar `docker/cli/Dockerfile` e testar build `musl` + runtime não-root**

Implementar `docker/cli/Dockerfile`:
- `ARG TARGETARCH` no stage `builder` (`FROM rust:1.100.0-slim-bookworm AS builder`): instala `musl-tools`, mapeia `arm64` → `aarch64-unknown-linux-musl` e `amd64` → `x86_64-unknown-linux-musl` (ou `$(uname -m)-unknown-linux-musl`), adiciona target e compila `cargo build --release --locked --package sexta-feira-cli --target "$TARGET"` com `--mount=type=cache` para `/usr/local/cargo/registry` e `/src/target`, gravando `/out/sexta-feira`.
- Stage `FROM scratch AS export` copiando `/out/sexta-feira` como `/sexta-feira` (para extração limpa via `docker build --target export --output dist` no `just docker-build`!).
- Stage final `FROM alpine:3.21 AS runtime`: instala `ca-certificates`, cria grupo/usuário `sextafeira` (`10000:10000`) e `/dados` com `chown 10000:10000`, copia `/out/sexta-feira` para `/usr/local/bin/sexta-feira`, define `USER 10000:10000`, `ENV HOME=/dados HERMES_HOME=/dados/.hermes`, `WORKDIR /workspace`, `ENTRYPOINT ["sexta-feira"]`.

Run:
```bash
docker build -f docker/cli/Dockerfile -t sexta-feira-cli:test .
docker run --rm sexta-feira-cli:test --json harnesses
docker run --rm --entrypoint /bin/sh sexta-feira-cli:test -c 'id; ldd /usr/local/bin/sexta-feira 2>&1 || true'
```
Expected: retorna JSON válido dos harnesses, `uid=10000(sextafeira)` e binário estático (`Not a valid dynamic program` / `statically linked`).

- [ ] **Step 4: Remover imagens de teste temporárias e commitar**

```bash
docker rmi sexta-feira-ci:test sexta-feira-cli:test >/dev/null 2>&1 || true
git add .gitignore .dockerignore .env.example docker/hermes/.env.example docker/ci/Dockerfile docker/cli/Dockerfile
git commit -m "build(docker): adiciona dockerfiles de CI e CLI e centraliza .env.example na raiz"
```

---

### Task 2: Compose unificado (`docker/compose.yaml`) e scripts do Hermes (`bootstrap.sh` + `lab.sh`)

**Files:**
- Create: `docker/compose.yaml`
- Delete: `docker/hermes/compose.yaml`
- Move/Modify: `docker/hermes/scripts/bootstrap-hermes.sh` → `docker/hermes/bootstrap.sh`
- Move/Modify: `docker/hermes/scripts/lab.sh` → `docker/hermes/lab.sh`

**Interfaces:**
- Consumes:
  - `docker/ci/Dockerfile`, `docker/cli/Dockerfile`, `.env.example` / `.env` na raiz.
- Produces:
  - `docker/compose.yaml` com `name: sexta-feira`, perfis `ci`, `cli`, `local`, `dia-zero`, e volumes `cargo_registry`, `cargo_git`, `target_ci`, `target_linux`, `cli_dados`, `sexta_feira_bin`, `hermes_dados`, `hermes_dados_dia_zero`.
  - `docker/hermes/bootstrap.sh` (`chmod +x`) com `os.open(..., 0o600)`, newline guard, sync de `OPENROUTER_API_KEY` e marcador `.sexta-feira-lab-bootstrap`.
  - `docker/hermes/lab.sh` (`chmod +x`) com subcomandos `subir`, `chat`, `chat-padrao`, `shell`, `status`, `logs`, `parar`, `resetar`, `destruir`, `ajuda`, lendo `.env` na raiz, usando `-p "$PROJETO"`, validando espaços pós-aspas na chave, checando `s6-svscan` no readiness, validando args extras em `cmd_chat`, tratando EOF em `cmd_destruir`, e isolando `parar`/`resetar` aos serviços/volumes do laboratório.

- [ ] **Step 1: Escrever testes TDD (RED) para os novos comportamentos de `bootstrap.sh` e `lab.sh`**

Testar antes de refatorar (validando que os scripts antigos falham nas condições novas):
1. Chave `"   "` (apenas espaços entre aspas) em `.env` deve ser rejeitada por `lab.sh`.
2. `lab.sh chat sexta-feira local extra-arg` deve falhar com erro de argumento excedente.
3. `lab.sh destruir </dev/null` (EOF imediato) deve cancelar com exit `0` em vez de falhar com exit `1`.
4. `.env` criado por `bootstrap.sh` deve nascer com `0600` direto (`os.open`).

- [ ] **Step 2: Criar `docker/compose.yaml` e remover `docker/hermes/compose.yaml`**

Definir todos os serviços (`ci`, `cli`, `hermes-builder`, `hermes`, `hermes-dia-zero`) e volumes nomeados conforme spec §5.3 (`env_file: path: ../.env, required: false` nos serviços Hermes), montando `./hermes:/opt/sexta-feira/scripts:ro`.

- [ ] **Step 3: Mover e atualizar `docker/hermes/bootstrap.sh` e `docker/hermes/lab.sh`**

Aplicar `git mv` dos scripts antigos para `docker/hermes/bootstrap.sh` e `docker/hermes/lab.sh`, ajustando caminhos (`REPO_ROOT`, `$REPO_ROOT/.env`, `$REPO_ROOT/docker/compose.yaml`, `/opt/sexta-feira/scripts/bootstrap.sh`) e todas as correções de robustez do **Review Focus**.

- [ ] **Step 4: Validar sintaxe, `compose config` em todos os perfis e passar os testes (GREEN)**

Run:
```bash
sh -n docker/hermes/bootstrap.sh
bash -n docker/hermes/lab.sh
for p in ci cli local dia-zero; do
  docker compose -f docker/compose.yaml --env-file .env.example --profile "$p" config >/dev/null
done
```
E rodar a verificação dos 4 cenários do Step 1 (chave `"   "`, arg extra em `chat`, EOF em `destruir`, permissões `0600` e sync em `bootstrap.sh`), garantindo todos GREEN.

- [ ] **Step 5: Commit**

```bash
git add docker/compose.yaml docker/hermes/compose.yaml docker/hermes/bootstrap.sh docker/hermes/lab.sh docker/hermes/scripts
git commit -m "refactor(docker): unifica compose e scripts sob docker/ com .env na raiz"
```

---

### Task 3: Reestruturar `justfile` na raiz e validar via `just` no container `ci`

**Files:**
- Modify: `justfile`

**Interfaces:**
- Consumes:
  - `docker/compose.yaml` (perfis `ci`, `cli`), `docker/cli/Dockerfile` (`--target export`), `docker/hermes/lab.sh`.
- Produces:
  - `justfile` com `set shell := ["bash", "-euo", "pipefail", "-c"]`, `set dotenv-load`, `set positional-arguments`, e grupos `[group('desenvolvimento')]`, `[group('docker')]`, `[group('cli')]`, `[group('laboratorio')]`.

- [ ] **Step 1: Escrever o novo `justfile` agrupado**

Incluir:
- `[group('desenvolvimento')]`: `fmt`, `lint`, `doc`, `test`, `deny`, `repo`, `ci`, `snapshots` (mantendo receitas existentes).
- `[group('docker')]`:
  - `docker-lint`, `docker-doc`, `docker-test`, `docker-deny`, `docker-repo`, `docker-ci`: executam via `docker compose -f docker/compose.yaml --profile ci run --rm ci just <receita>` (mantendo um único ponto de verdade das regras no próprio `justfile`!).
  - `docker-build`: constrói `docker/cli/Dockerfile` (`sexta-feira-cli:local`) e exporta o binário estático Linux para `./dist/sexta-feira`.
  - `docker-shell`: abre shell interativo no serviço `ci`.
- `[group('cli')]`:
  - `cli *args`: executa `docker compose -f docker/compose.yaml --profile cli run --rm cli "$@"`.
- `[group('laboratorio')]`:
  - `subir modo="local" *opcoes`
  - `chat perfil="sexta-feira" modo="local"`
  - `chat-padrao modo="local"`
  - `shell modo="local"`
  - `status`
  - `logs modo="local"`
  - `parar`
  - `resetar alvo="todos"`
  - `destruir *opcoes`

- [ ] **Step 2: Validar formatação e execução do `justfile` dentro do container `ci`**

Run:
```bash
docker compose -f docker/compose.yaml --env-file .env.example --profile ci run --rm --build ci sh -c '
  just --fmt --check
  just --list
  just ci
'
```
Expected: `just --fmt --check` passa, `just --list` exibe os 4 grupos limpos e `just ci` completa com todos os checks verdes (fmt, clippy, machete, doc, nextest, doctest, deny, zizmor).

- [ ] **Step 3: Commit**

```bash
git add justfile
git commit -m "feat(just): organiza justfile por grupos e integra CI, CLI e laboratorio Docker"
```

---

### Task 4: Documentação, validação ponta a ponta e atualização da PR #25

**Files:**
- Modify: `docs/ambiente-local-docker.md`
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `CONTRIBUTING.md`

**Interfaces:**
- Consumes:
  - Comandos finais do `justfile`, árvore de `docker/`, `.env.example` na raiz.
- Produces:
  - Documentação sincronizada em `pt-BR` e PR #25 atualizada com checks verdes.

- [ ] **Step 1: Atualizar `docs/ambiente-local-docker.md`, `README.md`, `AGENTS.md` e `CONTRIBUTING.md`**

- `docs/ambiente-local-docker.md`: reescrever cobrindo `.env` na raiz, `just docker-ci`, `just docker-build`, `just cli`, fluxo completo do laboratório (`just subir`, `just chat-padrao`, `just chat`, `just resetar`, `just destruir`), nota sobre `brew install just` caso o host ainda não tenha `just` e aviso sobre rodar `aplicar` antes de comandos `hermes -p sexta-feira`.
- `README.md`: atualizar seção `Desenvolvimento` mencionando `just docker-ci` como alternativa sem instalar ferramentas no host e estrutura de pastas com `docker/`.
- `AGENTS.md` e `CONTRIBUTING.md`: adicionar `docker/` na árvore de estrutura e mencionar `just docker-ci`.

- [ ] **Step 2: Executar verificação ponta a ponta completa (CI Docker + CLI Docker + Hermes Lab)**

Run (usando `.env` temporário na raiz com chave sintética e limpando tudo ao final):
1. `docker compose -f docker/compose.yaml --env-file .env.example --profile ci run --rm ci just ci`
2. `docker compose -f docker/compose.yaml --env-file .env.example --profile cli run --rm cli --json harnesses` e fluxo stateful `iniciar --nao-interativo` + `aplicar hermes` + `verificar hermes` dentro de `cli_dados`.
3. Build da CLI estática em `./dist/sexta-feira` (confirmar existência e remover `./dist`).
4. `./docker/hermes/lab.sh subir local` → verificar `API_SERVER_KEY` + `OPENROUTER_API_KEY` + `sexta-feira aplicar`/`verificar` → `subir dia-zero` → `resetar` (confirmar que `target_ci`, `target_linux` e `cargo_registry` permanecem) → `destruir --sim` → remover `.env` temporário.

- [ ] **Step 3: Commitar documentação, atualizar título/descrição da PR #25, fazer push e acompanhar CI**

```bash
git add docs/ambiente-local-docker.md README.md AGENTS.md CONTRIBUTING.md
git commit -m "docs: atualiza documentacao para dockerizacao completa via justfile"
git push
gh pr edit 25 --title "feat(docker): dockeriza CI, CLI e laboratorio local via justfile" --body "..."
gh pr checks 25 --watch --interval 10
```
Expected: todos os checks da PR #25 (`CI`, `Título da PR`, `Labels por área`) passando em verde.
