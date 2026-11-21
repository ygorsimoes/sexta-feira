# Design: Governança e Workflows para Repositório Público

- **Issue:** #21
- **Branch:** `chore/governanca-repositorio-publico`
- **Data:** 2026-11-21

## 1. Contexto e objetivo

Com a abertura de `ygorsimoes/sexta-feira` como repositório público sob licença MIT, os fluxos de contribuição externa, segurança de workflows e documentação comunitária precisam ficar consistentes com o novo estado do projeto.

As configurações de segurança da plataforma no GitHub (`Dependabot security updates`, `Secret scanning`, `Push protection`, `Vulnerability alerts` e `Private vulnerability reporting`) já foram ativadas via API, e o ruleset da branch `main` (`27317213`) foi alinhado para aceitar exclusivamente `squash` merge. Este design cobre as mudanças versionadas no repositório e o sequenciamento do primeiro release (`v0.1.0`).

## 2. Mudanças no repositório

### 2.1. `.github/workflows/pr.yml`

No job `labels` (`Labels por área`), pull requests originadas de forks executam o evento `pull_request` com `GITHUB_TOKEN` somente de leitura, o que faria `actions/labeler` falhar por falta de permissão `pull-requests: write`.

- **Ajuste:** alterar a condição `if` do job `labels` para:
  `github.event.pull_request.user.login != 'dependabot[bot]' && !github.event.pull_request.head.repo.fork`
- **Segurança:** manter o trigger `pull_request` (sem migrar para `pull_request_target`), preservando a conformidade com o `zizmor`.

### 2.2. `SECURITY.md`

Adicionar política de segurança na raiz do repositório, em `pt-BR`:

- Canais de reporte privado em ordem de preferência:
  1. **GitHub Private Vulnerability Reporting** (`Security` -> `Advisories` -> `Report a vulnerability` em `ygorsimoes/sexta-feira`).
  2. E-mail do mantenedor (`contato.ygorsimoes@gmail.com`).
- Escopo e prazos esperados (confirmação inicial em até 72 horas; sem divulgação pública antes de correção publicada ou acordo mútuo).
- Versões suportadas (série atual `0.x` na branch `main`).

### 2.3. `CODE_OF_CONDUCT.md`

Adicionar Código de Conduta na raiz do repositório baseado no **Contributor Covenant v2.1** em `pt-BR`, com contato para reporte (`contato.ygorsimoes@gmail.com`), responsabilidades de moderação e escopo aplicável ao repositório, issues, pull requests e espaços associados do projeto.

### 2.4. `docs/visao.md`

Na seção `2. Decisões tomadas`, atualizar a linha `Público` de `Uso pessoal (um único usuário)` para explicitar que o projeto continua opinativo para fluxo de uso pessoal, mas é distribuído em código aberto (licença MIT) e aberto a contribuições e adoção por outros usuários.

## 3. Sequenciamento do release `v0.1.0`

1. Abrir a PR desta branch (`Closes #21`), validar que todos os checks de CI (`CI`, `Título da PR` e `zizmor`) passam e fazer o squash merge na `main`.
2. Em seguida, fazer o merge da PR #10 (`chore(main): release 0.1.0`, gerenciada pelo `release-please`), garantindo que a tag `v0.1.0` e os binários gerados pelo `cd.yml` já incluam `SECURITY.md`, `CODE_OF_CONDUCT.md` e o ajuste de `pr.yml`.

## 4. Verificação

- Validar sintaxe YAML de `.github/workflows/pr.yml` com `ruby -ryaml -e "YAML.safe_load(File.read('.github/workflows/pr.yml'))"`.
- Rodar `cargo test --workspace --locked` para garantir integridade local.
- Conferir os checks da PR no GitHub (`CI`, `Título da PR` e `zizmor`).
