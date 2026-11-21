# Design: Dockerização completa (CI, CLI e laboratório Hermes) via `justfile`

Data: 2026-11-21
Status: rascunho para revisão

## 1. Intenção e porquê

Reorganizar toda a camada de infraestrutura, Docker e automação do repositório **Sexta-feira** para que:
1. O [justfile](../../../justfile) na raiz seja o **ponto único e intuitivo** de operação (com grupos claros, carregamento automático de `.env` da raiz e comandos curtos como `just subir`, `just chat`, `just docker-ci`, `just resetar`, `just destruir`), sem necessidade de entrar em subpastas nem digitar caminhos longos de script.
2. O projeto seja **dockerizado em todas as frentes práticas**:
   - **CI e desenvolvimento (`docker/ci/Dockerfile`)**: reproduz localmente todos os checks do [ci.yml](../../../.github/workflows/ci.yml) (`fmt`, `clippy`, `cargo-machete`, `doc`, `nextest`, `doctest`, `cargo-deny`, `zizmor`) sem depender de ferramentas instaladas no host.
   - **Build e runtime da CLI (`docker/cli/Dockerfile`)**: build multi-stage gerando o binário estático Linux (`musl`, compatível com `x86_64` e `aarch64`) e uma imagem mínima da CLI `sexta-feira`.
   - **Laboratório do Hermes Agent (`docker/hermes/`)**: ambiente local pronto para testar a Sexta-feira via CLI ou instalação guiada pela `SKILL.md` (`local` e `dia-zero`) com OpenRouter pré-configurado sem wizard.
3. Toda a refatoração seja consolidada na PR #25 (`feat/ambiente-local-docker-hermes`), atualizando título e descrição para refletir o escopo completo.

## 2. Objetivos, não-objetivos e anti-objetivos

### Objetivos
- Centralizar configuração de ambiente no `.env` da raiz (versionando `.env.example` na raiz e removendo `docker/hermes/.env.example`).
- Adicionar `.dockerignore` na raiz para manter o contexto de build Docker enxuto (excluindo `/target`, `.git`, `.superpowers`, `.env*` exceto `.env.example`).
- Criar `docker/ci/Dockerfile` com Rust `1.100.0` ([rust-toolchain.toml](../../../rust-toolchain.toml)), `rustfmt`, `clippy`, `musl-tools` e `cargo-nextest`, `cargo-deny`, `cargo-machete`, `zizmor`, `just` via `ghcr.io/cargo-bins/cargo-binstall`.
- Criar `docker/cli/Dockerfile` multi-stage que compila `sexta-feira-cli` estaticamente (`musl` conforme `TARGETARCH`) e entrega uma imagem `alpine:3.21` mínima com usuário não-root (`10000`).
- Unificar a orquestração em `docker/compose.yaml` (perfis `ci`, `cli`, `local`, `dia-zero`), compartilhando caches Cargo/target em volumes nomeados.
- Reorganizar `docker/hermes/` (`bootstrap.sh` e `lab.sh`), incorporando as melhorias de robustez já identificadas (readiness com `s6-svscan`, escrita `0600` atômica via `os.open`, tratamento de EOF/argumentos/espaços e nota pré-`aplicar` na documentação).
- Estruturar o `justfile` com `[group('desenvolvimento')]`, `[group('docker')]`, `[group('cli')]` e `[group('laboratorio')]`.
- Atualizar a documentação afetada ([ambiente-local-docker.md](../../ambiente-local-docker.md), [README.md](../../../README.md), [AGENTS.md](../../../AGENTS.md) e [CONTRIBUTING.md](../../../CONTRIBUTING.md) onde citam estrutura/comandos) e atualizar título/corpo da PR #25.

### Não-objetivos
- Alterar código Rust de produção em `crates/` (`sexta-feira-core`, `sexta-feira-adapters`, `sexta-feira-cli`) ou os workflows `.github/workflows/*.yml`.
- Substituir `justfile` por `Makefile` (`justfile` permanece como único executor de tarefas).
- Criar laboratórios para harnesses sem adapter ainda (`OpenClaw`, `Claude Code`, `OpenCode`), embora a estrutura `docker/<harness>/` já nasça pronta para recebê-los.

### Anti-objetivos
- Compartilhar `./target` do macOS dentro de containers Linux (`target_ci` e `target_linux` ficam em volumes nomeados separados).
- Instalar toolchain Rust completo dentro da imagem runtime do Hermes ou dentro da imagem runtime final da CLI.
- Commitar segredos (`OPENROUTER_API_KEY`, `.env` real) ou sobrescrever `/init` (`s6-overlay`) do Hermes.

## 3. Restrições

- Tudo em `pt-BR` conforme [AGENTS.md](../../../AGENTS.md).
- Segredos nunca entram no repositório: `.env` na raiz já é ignorado por `.gitignore`.
- Rust fixado em `1.100.0` ([rust-toolchain.toml](../../../rust-toolchain.toml)).
- `just` no host continua sendo opcional no código-fonte se alguém invocar os scripts internos, mas o `justfile` é a interface principal documentada e o `just` também vem instalado dentro do container `ci`.

## 4. Abordagens consideradas

1. **Arquitetura unificada em `docker/` + `.env` e `justfile` agrupado na raiz *(Escolhida)*:** separa responsabilidades por pasta (`docker/ci/`, `docker/cli/`, `docker/hermes/`), usa um único `docker/compose.yaml` com volumes de cache compartilhados e expõe comandos curtos em grupos no `justfile`.
2. **Um `Dockerfile` gigante na raiz + múltiplos arquivos Compose:** polui a raiz do projeto Rust e fragmenta caches e limpeza global.
3. **Toda lógica shell inline no `justfile`:** reduz arquivos auxiliares, mas torna o `justfile` ilegível nas rotinas de readiness do `s6-overlay` e validação de `.env`.

## 5. Desenho da solução

### 5.1 Estrutura de arquivos

```text
sexta-feira/
├── .dockerignore                  # Exclui target/, .git/, .superpowers/, .env* (exceto .env.example)
├── .env.example                   # Template único na raiz (OPENROUTER_API_KEY, HERMES_MODEL, etc.)
├── justfile                       # Ponto único com grupos: desenvolvimento, docker, cli, laboratorio
├── docker/
│   ├── compose.yaml               # Perfis: ci, cli, local, dia-zero
│   ├── ci/
│   │   └── Dockerfile             # Imagem completa de desenvolvimento/CI (Rust 1.100.0 + ferramentas CI)
│   ├── cli/
│   │   └── Dockerfile             # Multi-stage da CLI `sexta-feira` (musl estático + runtime alpine)
│   └── hermes/
│       ├── bootstrap.sh           # Provisionamento OpenRouter sem wizard (default + sexta-feira)
│       └── lab.sh                 # Orquestrador do laboratório chamado pelo `justfile`
└── docs/
    └── ambiente-local-docker.md   # Guia unificado do Docker (CI, CLI e laboratório Hermes)
```

### 5.2 `docker/ci/Dockerfile` e `docker/cli/Dockerfile`

- **`docker/ci/Dockerfile`:**
  - Base `rust:1.100.0-slim-bookworm`.
  - Instala `rustup component add rustfmt clippy`, pacotes de sistema (`musl-tools`, `git`, `ca-certificates`, `pkg-config`, `libssl-dev`) e via `ghcr.io/cargo-bins/cargo-binstall:latest` instala `cargo-nextest`, `cargo-deny`, `cargo-machete`, `zizmor` e `just`.
  - Prepara os dois alvos musl (`x86_64-unknown-linux-musl` e `aarch64-unknown-linux-musl`) ou o alvo nativo da arquitetura em build (`rustup target add "$(uname -m)-unknown-linux-musl"`). *Decisão técnica:* instalar o target musl da arquitetura atual do container (`uname -m`) mantém a imagem rápida para o host local (`aarch64` no Apple Silicon e `x86_64` no Linux x64).
- **`docker/cli/Dockerfile`:**
  - Stage `builder` (`rust:1.100.0-slim-bookworm`): instala `musl-tools`, adiciona o target `$(uname -m)-unknown-linux-musl` (ou mapeia `TARGETARCH` `arm64`→`aarch64-unknown-linux-musl`, `amd64`→`x86_64-unknown-linux-musl`), compila `cargo build --release --locked --package sexta-feira-cli --target ...` usando mounts de cache BuildKit e copia o binário para `/usr/local/bin/sexta-feira`.
  - Stage `runtime` (`alpine:3.21`): instala `ca-certificates`, cria usuário não-root `sextafeira` (`UID 10000`), copia `/usr/local/bin/sexta-feira` e define `ENTRYPOINT ["sexta-feira"]`.

### 5.3 `docker/compose.yaml` unificado

- `name: sexta-feira` (ou respeitando `COMPOSE_PROJECT_NAME` se definido no ambiente).
- `env_file` opcional na raiz (`../.env`, `required: false`) para `hermes` / `hermes-dia-zero`.
- **Serviços:**
  - `ci` (`profiles: ["ci"]`, `build` em `docker/ci/Dockerfile`): monta `..:/workspace`, `cargo_registry`, `cargo_git`, `target_ci:/workspace/target-docker` (`CARGO_TARGET_DIR=/workspace/target-docker`, `RUSTUP_TOOLCHAIN=1.100.0`).
  - `cli` (`profiles: ["cli"]`, `build` em `docker/cli/Dockerfile`): monta `cli_dados:/dados` (com `HOME=/dados` e `HERMES_HOME=/dados/.hermes`) e `..:/workspace:ro` (`working_dir: /workspace`), permitindo testar comandos com estado (`iniciar`, `aplicar hermes`, `verificar hermes`) em sandbox isolado sem risco de permissão no host e sem alterar os volumes do laboratório Hermes.
  - `hermes-builder` (`profiles: ["local"]`, `image: rust:1.100.0-slim-bookworm`, `RUSTUP_TOOLCHAIN=1.100.0`): compila `--release --locked --package sexta-feira-cli` com `cargo_registry`, `cargo_git` e `target_linux`, entregando `/out/sexta-feira` de forma atômica no volume `sexta_feira_bin`.
  - `hermes` (`profiles: ["local"]`, `nousresearch/hermes-agent:latest`): `depends_on: hermes-builder`, monta `hermes_dados:/opt/data`, `sexta_feira_bin:/opt/sexta-feira/bin:ro`, `..:/workspace/sexta-feira:ro`, `./hermes:/opt/sexta-feira/scripts:ro`, `command: ["sleep", "infinity"]`, `HERMES_UID=10000`, `HERMES_GID=10000`, `HERMES_DASHBOARD=0`.
  - `hermes-dia-zero` (`profiles: ["dia-zero"]`): mesmo base sem `hermes-builder`, sem `sexta_feira_bin`, sem repo montado, usando volume isolado `hermes_dados_dia_zero:/opt/data`.

### 5.4 `justfile` na raiz e ergonomia

```text
[desenvolvimento]
  fmt, lint, doc, test, deny, repo, ci, snapshots

[docker]
  docker-ci, docker-lint, docker-doc, docker-test, docker-deny, docker-repo, docker-build, docker-shell

[cli]
  cli *args          # roda a CLI containerizada (`docker compose --profile cli run --rm cli ...`)

[laboratorio]
  subir [modo="local"] [*opcoes]
  chat [perfil="sexta-feira"] [modo="local"]
  chat-padrao [modo="local"]
  shell [modo="local"]
  status
  logs [modo="local"]
  parar
  resetar [alvo="todos"]
  destruir [*opcoes]
```

- `justfile` usa `set dotenv-load`, `set positional-arguments` e `set shell := ["bash", "-euo", "pipefail", "-c"]`.
- `just docker-build` constrói a imagem `docker/cli/Dockerfile` e opcionalmente exporta o binário estático Linux para `./dist/sexta-feira` (com `dist/` ignorado no `.gitignore` / `.dockerignore`).
- `docker/hermes/lab.sh` lê `.env` / `.env.example` da **raiz do repositório** (`$REPO_ROOT/.env`), valida chave não vazia (removendo espaços após remover aspas), usa `PROJETO="${COMPOSE_PROJECT_NAME:-sexta-feira}"` (e passa `-p "$PROJETO"` no `docker compose`), aguarda `s6-svscan` em `aguardar_container()`, trata EOF em `cmd_destruir()` e valida argumentos extras em `cmd_chat()`.
- **Escopo explícito de limpeza no Compose unificado:**
  - `just parar`: para apenas os serviços do laboratório (`hermes`, `hermes-dia-zero`), sem derrubar eventual container `ci`.
  - `just resetar [local|dia-zero|todos]`: remove apenas os containers alvo do laboratório (`rm -sf`) e seus volumes de dados (`hermes_dados*`), preservando `ci`, `cli_dados` e todos os caches Cargo/target (`cargo_registry`, `cargo_git`, `target_linux`, `target_ci`).
  - `just destruir [--sim]`: remove **todos** os recursos Docker do projeto (`docker compose ... down -v --remove-orphans`, incluindo `hermes*`, `ci`, `cli` e todos os volumes de dados e caches Rust, mediante confirmação interativa ou `--sim`).
- `docker/hermes/bootstrap.sh` cria `.env` temporário já com `0600` via `os.open`, garante quebra de linha final e sincroniza `OPENROUTER_API_KEY` preservando `config.yaml` manual (até `--reconfigurar`).

## 6. O que fica a critério do construtor

- Detalhes internos de formatação das receitas no `justfile` (mantendo `.editorconfig` `indent_size = 4`).
- Ajuste fino de flags no `cargo-binstall` dentro de `docker/ci/Dockerfile` se alguma crate precisar `--strategies` ou fallback de compilação na plataforma `arm64`/`amd64`.
- Verificação real em Docker construindo `docker/ci/Dockerfile`, `docker/cli/Dockerfile`, rodando `just ci` dentro do container `ci`, validando `cli --json harnesses` e validando o fluxo completo do laboratório Hermes (`local` e `dia-zero`), antes de atualizar título e descrição da PR #25.
