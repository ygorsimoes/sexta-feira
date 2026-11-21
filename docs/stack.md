# Stack

Bibliotecas e ferramentas confirmadas para o projeto. Uma biblioteca só entra no `Cargo.toml` quando o código passa a usá-la; as que ainda não entraram estão marcadas como "prevista".

## Bibliotecas

| Função | Biblioteca | Situação |
|---|---|---|
| CLI | `clap` | em uso |
| Serialização | `serde`, `serde_json`, `toml` | `serde` e `toml` em uso; `serde_json` prevista |
| Configuração | `figment` | prevista (se necessário para mesclar configs) |
| Erros | `thiserror` nas bibliotecas, `anyhow` nos binários | em uso |
| Logs | `tracing`, `tracing-subscriber` | `tracing-subscriber` em uso; `tracing` prevista |

## Ferramentas

| Função | Ferramenta |
|---|---|
| Organização | Cargo workspace (`core`, `adapters`, `cli`) |
| Rust | Edição 2024, versão fixada em `rust-toolchain.toml` |
| Testes | `cargo-nextest`, `insta` (snapshots), `assert_cmd` |
| Qualidade | `rustfmt`, `clippy` com avisos como erro, `cargo-deny`, `cargo-machete` (dependências sem uso) |
| CI | GitHub Actions, com `zizmor` para a segurança dos workflows |
| Atualização de dependências | Dependabot (Cargo, GitHub Actions e versão do Rust) |
| Tarefas locais | `just` |
| Commits | Conventional Commits em pt-BR |
