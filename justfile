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
    cargo machete

# Gera a documentação, com avisos como erro.
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

# Roda os testes.
test:
    cargo nextest run --workspace --locked
    cargo test --doc --workspace --locked

# Verifica licenças e avisos de segurança das dependências.
deny:
    cargo deny check

# Verifica a segurança dos workflows.
repo:
    zizmor --offline .github

# Tudo o que o CI roda.
ci: lint doc test deny repo

# Revisa snapshots alterados.
snapshots:
    cargo insta review
