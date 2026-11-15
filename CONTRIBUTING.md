# Contribuindo

## Fluxo

1. **Issue.** Todo trabalho começa em uma issue, aberta por um dos modelos: bug, funcionalidade ou tarefa.
2. **Branch.** Crie a branch a partir da `main`, no formato `tipo/descricao-em-pt-br`.
3. **Pull request.** Abra a pull request preenchendo o modelo e apontando a issue (`Closes #123`).
4. **CI.** A validação é o CI da pull request. O merge só acontece com os checks verdes.
5. **Merge.** Sempre squash; o título da pull request vira a mensagem do commit na `main`.

## Issues

- Uma issue descreve um problema ou um resultado, não uma lista de assuntos. Se crescer, divida.
- O título segue o mesmo padrão dos commits (`feat: ...`, `fix: ...`, `chore: ...`), porque costuma virar o título da pull request.
- Funcionalidades e tarefas precisam de critérios de aceite: o que tem de ser verdade para a issue ser fechada.
- Bugs precisam de passos para reproduzir, do resultado observado e do esperado.
- Nunca cole segredos, tokens ou dados pessoais em issues ou logs.

Labels: `bug`, `funcionalidade` e `tarefa` indicam o tipo e vêm do modelo; `core`, `adapters`, `cli`, `docs` e `ci` indicam a área; `dependencias` marca as pull requests do Dependabot.

## Pull requests

- Uma pull request trata de um assunto só. Mudanças sem relação vão em pull requests separadas.
- O título segue o [Conventional Commits](https://www.conventionalcommits.org/pt-br/) em pt-BR, por exemplo `feat(core): adiciona motor de políticas`. Um check valida o formato.
- Tipos aceitos: `build`, `chore`, `ci`, `docs`, `feat`, `fix`, `perf`, `refactor`, `revert`, `style`, `test`. O escopo é opcional e, quando usado, é a área (`core`, `adapters`, `cli`, `docs`, `ci`, `deps`).
- A descrição diz o que muda, por quê e como foi verificado, inclusive o que não foi possível verificar.
- Comportamento novo ou corrigido vem com teste.
- A documentação acompanha a mudança na mesma pull request.
- As labels de área são aplicadas automaticamente pelos arquivos alterados.

## CI

| Check | O que cobre |
|---|---|
| Formatação, lints e documentação | `cargo fmt`, `cargo clippy` com avisos como erro, `cargo doc`, dependências sem uso |
| Testes | `cargo nextest run` |
| Dependências | `cargo deny check` (licenças, avisos de segurança, origem) |
| Segurança dos workflows | `zizmor` |
| Título da PR | Conventional Commits |

O check `CI` resume os quatro primeiros. Além disso, o workflow `Auditoria` roda toda segunda-feira e procura avisos de segurança novos nas dependências.

## Dependências

- As versões ficam em `[workspace.dependencies]` no `Cargo.toml` da raiz; as crates usam `dep.workspace = true`.
- Bibliotecas novas seguem a lista em [docs/stack.md](docs/stack.md) e só entram quando o código passa a usá-las.
- O Dependabot abre, toda segunda-feira, uma pull request agrupada para o Cargo, uma para as GitHub Actions e uma para a versão do Rust. Ao aceitar a do Rust, ajuste também `rust-version` no `Cargo.toml` da raiz.
- Actions são fixadas por SHA, com a versão em comentário.

## Ambiente local (opcional)

O CI é a validação oficial; rodar as verificações localmente só antecipa o resultado.

- Rust na versão fixada em `rust-toolchain.toml` (o `rustup` instala sozinho).
- [`just`](https://github.com/casey/just), [`cargo-nextest`](https://nexte.st), [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny), [`cargo-insta`](https://insta.rs), [`cargo-machete`](https://github.com/bnjbvr/cargo-machete) e [`zizmor`](https://github.com/zizmorcore/zizmor).

| Comando | O que faz |
|---|---|
| `just fmt` | Formata o código |
| `just lint` | Verifica formatação, lints e dependências sem uso |
| `just doc` | Gera a documentação, com avisos como erro |
| `just test` | Roda os testes |
| `just deny` | Verifica licenças e avisos de segurança das dependências |
| `just repo` | Verifica a segurança dos workflows |
| `just ci` | Roda tudo o que o CI roda |
| `just snapshots` | Revisa snapshots alterados |

## Convenções de código

- **Idioma:** documentação, mensagens de commit e mensagens ao usuário em pt-BR. Identificadores de domínio no código também em pt-BR.
- **Lints:** avisos do `clippy` são tratados como erro. `unsafe` é proibido.
- **Erros:** `thiserror` nas bibliotecas, `anyhow` nos binários.
