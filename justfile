# Tarefas locais da Sexta-feira. Rode `just` para listar.

default:
    @just --list

# Formata o código.
fmt:
    cargo fmt --all

# Verifica formatação e lints, como o CI.
lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --locked -- -D warnings

# Roda os testes.
test:
    cargo nextest run --workspace --locked

# Verifica licenças e avisos de segurança das dependências.
deny:
    cargo deny check

# Tudo o que o CI roda.
ci: lint test deny

# Revisa snapshots alterados.
snapshots:
    cargo insta review
