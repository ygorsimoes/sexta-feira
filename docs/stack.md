# Stack

Bibliotecas e ferramentas confirmadas para o projeto. Uma biblioteca só entra no `Cargo.toml` quando o código passa a usá-la; as que ainda não entraram estão marcadas como "prevista".

## Bibliotecas

| Função | Biblioteca | Situação |
|---|---|---|
| Runtime assíncrono | `tokio` | prevista |
| Servidor HTTP | `axum` | prevista |
| MCP | `rmcp` | prevista |
| Banco do motor (auditoria, políticas, agenda) | SQLite com `sqlx` | prevista |
| Telegram (avisos e botões) | `teloxide` | prevista |
| Cliente HTTP (Composio) | `reqwest` | prevista |
| CLI | `clap` | em uso |
| Serialização | `serde`, `serde_json`, `toml` | `serde` em uso |
| Configuração | `figment` | prevista |
| Erros | `thiserror` nas bibliotecas, `anyhow` nos binários | em uso |
| Logs | `tracing` | `tracing-subscriber` em uso; `tracing` prevista |
| Agendador | `tokio-cron-scheduler` | prevista |
| Datas | `jiff` | prevista |

## Ferramentas

| Função | Ferramenta |
|---|---|
| Organização | Cargo workspace (`core`, `adapters`, `cli`) |
| Rust | Edição 2024, versão fixada em `rust-toolchain.toml` |
| Testes | `cargo-nextest`, `insta` (snapshots), `assert_cmd` |
| Qualidade | `rustfmt`, `clippy` com avisos como erro, `cargo-deny`, `cargo-machete` (dependências sem uso), `typos` (ortografia) |
| CI | GitHub Actions, com `zizmor` para a segurança dos workflows |
| Atualização de dependências | Dependabot (Cargo, GitHub Actions e versão do Rust) |
| Tarefas locais | `just` |
| Commits | Conventional Commits em pt-BR |

## Serviços externos

| Função | Serviço |
|---|---|
| Integrações (Gmail, Google Calendar etc.) | Composio |
| Memória | ai-memory (opção preferida, pendente de testes; ver [visao.md](visao.md)) |
