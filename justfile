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

# Sobe o laboratório Docker do Hermes (modo: local ou dia-zero).
hermes-subir modo="local" *opcoes:
    ./docker/hermes/scripts/lab.sh subir {{modo}} {{opcoes}}

# Abre o chat do Hermes no perfil escolhido (padrão: sexta-feira).
hermes-chat perfil="sexta-feira" modo="local":
    ./docker/hermes/scripts/lab.sh chat {{perfil}} {{modo}}

# Abre o chat do Hermes no perfil padrão (para instalar via SKILL.md).
hermes-chat-padrao modo="local":
    ./docker/hermes/scripts/lab.sh chat-padrao {{modo}}

# Abre um shell dentro do container do Hermes.
hermes-shell modo="local":
    ./docker/hermes/scripts/lab.sh shell {{modo}}

# Mostra o estado dos containers, volumes e da Sexta-feira.
hermes-status:
    ./docker/hermes/scripts/lab.sh status

# Acompanha os logs do container do Hermes.
hermes-logs modo="local":
    ./docker/hermes/scripts/lab.sh logs {{modo}}

# Para os containers do laboratório preservando dados e cache.
hermes-parar:
    ./docker/hermes/scripts/lab.sh parar

# Limpa apenas os dados do Hermes (preservando cache Rust).
hermes-resetar alvo="todos":
    ./docker/hermes/scripts/lab.sh resetar {{alvo}}

# Remove tudo do laboratório Docker (containers, redes e todos os volumes).
hermes-destruir *opcoes:
    ./docker/hermes/scripts/lab.sh destruir {{opcoes}}

