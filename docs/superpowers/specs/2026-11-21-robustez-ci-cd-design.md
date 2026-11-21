# Design: Robustez de CI/CD (`ci.yml` e `cd.yml`)

Data: 2026-11-21
Status: rascunho para revisão

- **Intenção e porquê:** validar a suíte de testes também no `macos-latest` (alvo oficial de release e ambiente de desenvolvimento) e acelerar/validar o `cd.yml` com cache de build e smoke test antes de empacotar os binários.
- **Objetivos:**
  - Transformar o job `test` do `.github/workflows/ci.yml` em matriz `os: [ubuntu-latest, macos-latest]` (`fail-fast: false`), mantendo `lint`, `deny` e `repo` em `ubuntu-latest`.
  - Adicionar `Swatinem/rust-cache@d1527c8f67ef9af357290475d4af4b0fab8f12e0 # v2.9.3` (com `key: cd-${{ matrix.target }}`) e passo de smoke test (`--version` e `--json harnesses` no binário compilado de release) no `.github/workflows/cd.yml`.
- **Não-objetivos:** alterar metas de compilação (`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `aarch64-apple-darwin`), `release-please-config.json` ou workflows `pr.yml`/`auditoria.yml`.
- **Abordagens consideradas:** (1) matriz apenas no job `test` do `ci.yml` + cache/smoke test direto no `cd.yml` (Escolhida, evita triplicar custo de `lint`/`deny`/`repo` mantendo proteção em macOS); (2) matriz em todos os jobs do `ci.yml` (descartada pelo tempo/custo redundante para checagens idênticas de formatação e licenças).
- **Restrições:** Actions fixadas por SHA com comentário de versão, `persist-credentials: false`, sem interpolação insegura em `run:` e aprovação pelo `zizmor`.
