# Design: Robustez de CI/CD (`ci.yml` e `cd.yml`)

Data: 2026-11-21
Status: rascunho para revisão

- **Intenção e porquê:** validar a suíte de testes também no `macos-latest` (alvo oficial de release e ambiente de desenvolvimento) e validar o binário compilado no `cd.yml` com smoke test antes do empacotamento (mantendo builds de release sem cache para evitar `cache-poisoning` apontado pelo `zizmor`).
- **Objetivos:**
  - Transformar o job `test` do `.github/workflows/ci.yml` em matriz `os: [ubuntu-latest, macos-latest]` (`fail-fast: false`), mantendo `lint`, `deny` e `repo` em `ubuntu-latest`.
  - Adicionar passo de smoke test (`--version` e `--json harnesses` no binário compilado de release) no `.github/workflows/cd.yml`.
- **Não-objetivos:** alterar metas de compilação (`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `aarch64-apple-darwin`), `release-please-config.json` ou workflows `pr.yml`/`auditoria.yml`.
- **Abordagens consideradas:** (1) matriz apenas no job `test` do `ci.yml` + cache/smoke test direto no `cd.yml` (Escolhida, evita triplicar custo de `lint`/`deny`/`repo` mantendo proteção em macOS); (2) matriz em todos os jobs do `ci.yml` (descartada pelo tempo/custo redundante para checagens idênticas de formatação e licenças).
- **Restrições:** Actions fixadas por SHA com comentário de versão, `persist-credentials: false`, sem interpolação insegura em `run:` e aprovação pelo `zizmor`.
