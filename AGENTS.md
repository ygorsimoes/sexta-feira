# Guia para agentes

Sexta-feira é um plugin agnóstico de harness que transforma um agente de IA em um mordomo virtual. Leia [docs/visao.md](docs/visao.md) antes de mudanças de arquitetura.

## Estrutura

- `crates/sexta-feira-core`: motor, sem conhecimento de harness.
- `crates/sexta-feira-adapters`: tradução do motor para cada harness.
- `crates/sexta-feira-cli`: binário `sexta-feira`.
- `docs/`: visão, arquitetura e stack.
- `spikes/`: experimentos descartáveis, fora do workspace.

## Antes de concluir uma mudança

Rode `just ci` (ou, sem `just`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo nextest run --workspace --locked`, `cargo deny check`).

## Regras

- Documentação, commits e mensagens ao usuário em pt-BR.
- Commits e títulos de pull request no padrão Conventional Commits (`feat(core): adiciona motor de políticas`).
- Branches no formato `tipo/descricao-em-pt-br`, sem acentos (`feat/motor-de-politicas`).
- Nada específico de harness entra em `sexta-feira-core`.
- Bibliotecas seguem [docs/stack.md](docs/stack.md). Uma biblioteca fora da lista precisa ser confirmada com o dono do projeto antes de entrar.
- Segredos nunca entram no repositório.
