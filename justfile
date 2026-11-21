# Tarefas da Sexta-feira. Rode `just` para listar todos os grupos.

set shell := ["bash", "-euo", "pipefail", "-c"]
set dotenv-load
set positional-arguments

COMPOSE_BASE := "docker compose -p ${COMPOSE_PROJECT_NAME:-sexta-feira} -f docker/compose.yaml"

default:
    @just --list

# Formata o código Rust.
[group('desenvolvimento')]
fmt:
    cargo fmt --all

# Verifica formatação, lints e dependências sem uso.
[group('desenvolvimento')]
lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo machete

# Gera a documentação, com avisos como erro.
[group('desenvolvimento')]
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

# Roda os testes unitários, de integração e doctests.
[group('desenvolvimento')]
test:
    cargo nextest run --workspace --locked
    cargo test --doc --workspace --locked

# Verifica licenças e avisos de segurança das dependências.
[group('desenvolvimento')]
deny:
    cargo deny check

# Verifica a segurança dos workflows do GitHub Actions.
[group('desenvolvimento')]
repo:
    zizmor --offline .github

# Roda localmente tudo o que o CI verifica.
[group('desenvolvimento')]
ci: lint doc test deny repo

# Revisa snapshots alterados do insta.
[group('desenvolvimento')]
snapshots:
    cargo insta review

# Executa lints dentro do container de CI padronizado.
[group('docker')]
docker-lint:
    {{ COMPOSE_BASE }} --profile ci run --rm ci just lint

# Gera e valida a documentação dentro do container de CI.
[group('docker')]
docker-doc:
    {{ COMPOSE_BASE }} --profile ci run --rm ci just doc

# Roda todos os testes dentro do container de CI.
[group('docker')]
docker-test:
    {{ COMPOSE_BASE }} --profile ci run --rm ci just test

# Roda cargo-deny dentro do container de CI.
[group('docker')]
docker-deny:
    {{ COMPOSE_BASE }} --profile ci run --rm ci just deny

# Roda zizmor dentro do container de CI.
[group('docker')]
docker-repo:
    {{ COMPOSE_BASE }} --profile ci run --rm ci just repo

# Executa a suíte completa de CI dentro do container padronizado.
[group('docker')]
docker-ci:
    {{ COMPOSE_BASE }} --profile ci run --rm ci just ci

# Compila a imagem da CLI e exporta binário estático Linux para ./dist/sexta-feira.
[group('docker')]
docker-build:
    docker build -f docker/cli/Dockerfile --target runtime -t sexta-feira-cli:local .
    docker build -f docker/cli/Dockerfile --target export --output type=local,dest=dist .
    chmod +x dist/sexta-feira

# Abre um shell interativo dentro do container de CI.
[group('docker')]
docker-shell:
    {{ COMPOSE_BASE }} --profile ci run --rm ci bash

# Executa a CLI sexta-feira containerizada (ex.: just cli --json harnesses).
[group('cli')]
cli *args:
    {{ COMPOSE_BASE }} --profile cli run --rm cli "$@"

# Sobe o laboratório Hermes (`local` ou `dia-zero`; aceita `--reconfigurar`).
[group('laboratorio')]
subir modo="local" *opcoes:
    ./docker/hermes/lab.sh subir "$@"

# Abre o chat do Hermes no perfil escolhido (padrão: sexta-feira).
[group('laboratorio')]
chat perfil="sexta-feira" modo="local":
    ./docker/hermes/lab.sh chat "$@"

# Abre o chat do Hermes no perfil padrão (para instalar via SKILL.md).
[group('laboratorio')]
chat-padrao modo="local":
    ./docker/hermes/lab.sh chat-padrao "$@"

# Abre um shell interativo dentro do container Hermes.
[group('laboratorio')]
shell modo="local":
    ./docker/hermes/lab.sh shell "$@"

# Mostra o estado dos containers, volumes e da Sexta-feira.
[group('laboratorio')]
status:
    ./docker/hermes/lab.sh status

# Acompanha os logs do container Hermes (`local` ou `dia-zero`).
[group('laboratorio')]
logs modo="local":
    ./docker/hermes/lab.sh logs "$@"

# Para os containers do laboratório preservando dados e caches.
[group('laboratorio')]
parar:
    ./docker/hermes/lab.sh parar

# Apaga apenas dados do Hermes (`local`, `dia-zero` ou `todos`), mantendo caches Rust.
[group('laboratorio')]
resetar alvo="todos":
    ./docker/hermes/lab.sh resetar "$@"

# Remove todos os containers, redes e volumes Docker do projeto (use `--sim` para pular confirmação).
[group('laboratorio')]
destruir *opcoes:
    ./docker/hermes/lab.sh destruir "$@"
