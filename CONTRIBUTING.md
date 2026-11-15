# Contribuindo

## Requisitos

- Rust na versão fixada em `rust-toolchain.toml` (o `rustup` instala sozinho).
- [`just`](https://github.com/casey/just), [`cargo-nextest`](https://nexte.st), [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) e [`cargo-insta`](https://insta.rs).

## Comandos

| Comando | O que faz |
|---|---|
| `just fmt` | Formata o código |
| `just lint` | Verifica formatação e lints |
| `just test` | Roda os testes |
| `just deny` | Verifica licenças e avisos de segurança das dependências |
| `just ci` | Roda tudo o que o CI roda |
| `just snapshots` | Revisa snapshots alterados |

## Convenções

- **Idioma:** documentação, mensagens de commit e mensagens ao usuário em pt-BR. Identificadores de domínio no código também em pt-BR.
- **Commits:** [Conventional Commits](https://www.conventionalcommits.org/pt-br/), por exemplo `feat(core): adiciona motor de políticas`.
- **Branches:** todo trabalho entra na `main` por pull request, com o CI verde.
- **Dependências:** as versões ficam em `[workspace.dependencies]` no `Cargo.toml` da raiz; as crates usam `dep.workspace = true`. Bibliotecas novas seguem a lista em [docs/stack.md](docs/stack.md).
- **Lints:** avisos do `clippy` são tratados como erro. `unsafe` é proibido.
