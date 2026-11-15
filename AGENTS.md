# Guia para agentes

Sexta-feira é um plugin agnóstico de harness que transforma um agente de IA em um mordomo virtual. Leia [docs/visao.md](docs/visao.md) antes de mudanças de arquitetura e [CONTRIBUTING.md](CONTRIBUTING.md) para o fluxo de issues e pull requests.

## Estrutura

- `crates/sexta-feira-core`: motor, sem conhecimento de harness.
- `crates/sexta-feira-adapters`: tradução do motor para cada harness.
- `crates/sexta-feira-cli`: binário `sexta-feira`.
- `docs/`: visão, arquitetura e stack.
- `spikes/`: experimentos descartáveis, fora do workspace.
- `.github/`: workflows, Dependabot e modelos de issue e de pull request.

## Fluxo de trabalho

1. Crie uma branch a partir da `main`; nunca faça commit direto nela.
2. Faça a mudança e abra a pull request preenchendo o modelo.
3. A validação final é o CI da pull request. Acompanhe os checks e corrija o que falhar antes de dar o trabalho por concluído.

Antes de abrir a pull request, rode localmente o que estiver disponível (`just ci`, ou os comandos `cargo` da tabela abaixo) para não gastar uma rodada de CI com um erro simples. O `Cargo.lock` é sempre gerado pelo `cargo`, nunca editado à mão.

## O que o CI verifica

| Check | O que cobre |
|---|---|
| Formatação, lints e documentação | `cargo fmt`, `cargo clippy` com avisos como erro, `cargo doc`, dependências sem uso (`cargo machete`) |
| Testes | `cargo nextest run`; snapshots do `insta` não podem mudar no CI |
| Dependências | `cargo deny check` (licenças, avisos de segurança, origem) |
| Segurança dos workflows | `zizmor` |
| Título da PR | Conventional Commits |

O check `CI` resume os quatro primeiros e é o exigido para o merge, junto com `Título da PR`.

## Regras

- Documentação, commits e mensagens ao usuário em pt-BR. Identificadores de domínio no código também.
- Commits e títulos de pull request no padrão Conventional Commits (`feat(core): adiciona motor de políticas`). O merge é sempre squash, então o título da pull request vira o commit.
- Branches no formato `tipo/descricao-em-pt-br`, sem acentos (`feat/motor-de-politicas`).
- Uma pull request trata de um assunto só e aponta a issue que resolve (`Closes #123`).
- Nada específico de harness entra em `sexta-feira-core`.
- Bibliotecas seguem [docs/stack.md](docs/stack.md). Uma biblioteca fora da lista precisa ser confirmada com o dono do projeto antes de entrar, e só entra no `Cargo.toml` quando o código passa a usá-la.
- As versões das dependências ficam em `[workspace.dependencies]`; as crates usam `dep.workspace = true`.
- Actions nos workflows são fixadas por SHA, com a versão em comentário.
- `unsafe` é proibido. `unwrap` e `expect` só em testes.
- Segredos nunca entram no repositório.
