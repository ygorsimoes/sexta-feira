# Plano de Implementação: Instalação guiada pelo próprio agente (`SKILL.md`)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Criar a skill `recursos/skills/instalar-sexta-feira/SKILL.md` para que qualquer agente de IA consiga instalar a Sexta-feira em si mesmo usando a CLI em Rust quando houver adapter disponível e fazendo fallback assistido a partir de `recursos/mordomo/*.toml` quando não houver, além de documentar esse fluxo no `README.md` e em `docs/visao.md`.

**Architecture:** Uma única `SKILL.md` em formato Agent Skills funciona tanto como protocolo imediato via URL raw no dia zero quanto como skill instalável pelos harnesses. O fluxo combina diagnóstico de ambiente/harness, obtenção verificada da CLI `sexta-feira`, personalização segura em `~/.config/sexta-feira/*.toml` e bifurcação explícita entre o motor declarativo da CLI (`plano` → `aplicar` → `verificar`) e a tradução assistida dos recursos em `recursos/mordomo/*.toml`.

**Tech Stack:** Markdown (Agent Skills `SKILL.md` com frontmatter YAML, GitHub Flavored Markdown), CLI `sexta-feira` (`clap` / `--json`), TOML.

**Spec:** [docs/superpowers/specs/2026-11-21-instalacao-guiada-por-agente-design.md](../specs/2026-11-21-instalacao-guiada-por-agente-design.md)

## Global Constraints

- Toda documentação, instruções na skill, commits e mensagens em `pt-BR` (`AGENTS.md`).
- Trabalho realizado em branch criada a partir da `main` (ex.: `feat/skill-instalacao-por-agente`), nunca direto na `main` (`AGENTS.md`).
- Apenas comandos, subcomandos, flags, caminhos e alvos de Release reais do repositório (`crates/sexta-feira-cli/src/main.rs`, `.github/workflows/cd.yml`, `recursos/mordomo/*.toml`).
- Sem alterações em código Rust ou workflows de CI/CD.
- Nenhuma ação destrutiva, `--forcar` ou sobrescrita de arquivo no fallback sem confirmação explícita do usuário.

## Review Focus

- **Execução imediata vs. salvamento da skill:** no dia zero (quando o usuário cola a URL da skill pedindo instalação), o agente não deve apenas copiar `SKILL.md` para a pasta de skills e parar; o topo da skill precisa instruir execução imediata.
- **Rejeição de chaves desconhecidas (`#[serde(deny_unknown_fields)]`):** `Persona` e `PerfilDoUsuario` rejeitam campos desconhecidos; a skill deve fixar as chaves válidas exatas de `usuario.toml` (`tratamento`, `idioma`, `preferencias`) e `persona.toml` (`nome`, `arquetipo`, `idioma`, `resumo`, `tom`, `diretrizes`, `limites`).
- **Plataforma sem binário de Release ou falha na obtenção da CLI:** o CD publica apenas `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` e `aarch64-apple-darwin` com `SHA256SUMS.txt`; em outras plataformas (ou ambientes sem `cargo`/rede para release), a skill deve saber validar checksum, tentar `cargo build` quando disponível ou transicionar de forma explícita para o fallback assistido.
- **Segurança no fallback sem adapter:** o agente precisa avisar claramente que não há adapter oficial na CLI para o harness atual, confirmar antes de editar, preservar conteúdo existente com `<!-- sexta-feira:inicio -->` / `<!-- sexta-feira:fim -->` ou backup `.bak` e nunca inventar flags não verificadas de outros harnesses.
- **Consistência de URLs e links:** URLs raw apontando para `ygorsimoes/sexta-feira` na branch `main` e links relativos entre `README.md`, `docs/visao.md` e `recursos/skills/instalar-sexta-feira/SKILL.md` devem bater exatamente.

---

## Mapa de Arquivos

- Create: `recursos/skills/instalar-sexta-feira/SKILL.md` — protocolo completo de instalação guiada pelo próprio agente.
- Modify: `README.md` — destaque de instalação pelo próprio agente no **Início rápido** e atualização da árvore em **Estrutura do repositório**.
- Modify: `docs/visao.md` — registro de `recursos/skills/` na seção **5. Estrutura do repositório**.

---

### Task 1: Criar branch e implementar `recursos/skills/instalar-sexta-feira/SKILL.md`

**Files:**
- Create: `recursos/skills/instalar-sexta-feira/SKILL.md`

**Interfaces:**
- Consumes:
  - CLI `sexta-feira`: subcomandos `harnesses`, `plano`, `aplicar`, `verificar`; flags globais/alvo `--json`, `--destino`, `--perfil`, `--persona`, `--usuario`, `--forcar`.
  - Schemas de `recursos/mordomo/persona.toml` (`nome`, `arquetipo`, `idioma`, `resumo`, `tom`, `diretrizes`, `limites`) e `recursos/mordomo/usuario.toml` (`tratamento`, `idioma`, `preferencias`).
  - Artefatos de Release em `.github/workflows/cd.yml`: `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `aarch64-apple-darwin` e `SHA256SUMS.txt`.
- Produces:
  - Skill `recursos/skills/instalar-sexta-feira/SKILL.md` auto-contida em `pt-BR`.

- [ ] **Step 1: Criar branch de trabalho a partir da `main`**

Run:
```bash
git checkout -b feat/skill-instalacao-por-agente
```
Expected: `Switched to a new branch 'feat/skill-instalacao-por-agente'`.

- [ ] **Step 2: Escrever `recursos/skills/instalar-sexta-feira/SKILL.md`**

Incluir, de forma clara e enxuta:
1. Frontmatter YAML (`name: instalar-sexta-feira` e `description` objetiva em `pt-BR`).
2. **Regra de entrada (execução imediata):** orientar que, se o usuário pediu para instalar a Sexta-feira agora (inclusive via URL), o agente deve executar o procedimento imediatamente em vez de apenas salvar a skill.
3. **Passo 1 — Diagnóstico do ambiente:** identificar SO/arquitetura (`uname -s`, `uname -m`), harness em execução no momento, caminhos de configuração do harness e ferramentas disponíveis (`curl`, `tar`, `sha256sum`/`shasum`, `git`, `cargo`).
4. **Passo 2 — Obter a CLI `sexta-feira`:**
   - Usar `sexta-feira` já presente no `PATH` ou no repositório local (`cargo run -q -p sexta-feira-cli --` / `./target/release/sexta-feira`).
   - Caso ausente: verificar última Release de `ygorsimoes/sexta-feira` para os alvos `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` ou `aarch64-apple-darwin`, baixar `.tar.gz` + `SHA256SUMS.txt`, validar o checksum antes de extrair e instalar em diretório do `PATH` (ex.: `~/.local/bin`, pedindo confirmação se precisar criar/alterar `PATH`). Nunca pular verificação de `SHA256SUMS.txt` sem aviso e consentimento.
   - Se não houver binário pré-compilado para a plataforma/arquitetura e houver `cargo`/`rustup`, compilar a partir do repositório (`cargo build --release --package sexta-feira-cli`).
   - Se não for possível obter nem compilar a CLI, registrar o motivo e seguir diretamente para o **Passo 5B (Fallback assistido)**.
5. **Passo 3 — Checar suporte na CLI (`harnesses --json`):** executar `sexta-feira --json harnesses` e verificar se o harness atual está listado com `"disponivel": true`.
6. **Passo 4 — Personalização em camadas (`~/.config/sexta-feira/`):**
   - Verificar se `$XDG_CONFIG_HOME/sexta-feira/` ou `~/.config/sexta-feira/` já contém `usuario.toml` / `persona.toml`.
   - Perguntar brevemente ao usuário (sem questionário longo) se quer ajustar forma de tratamento (padrão `"senhor"`) ou preferências iniciais e gravar apenas chaves válidas (`usuario.toml`: `tratamento`, `idioma`, `preferencias`; `persona.toml`: `nome`, `arquetipo`, `idioma`, `resumo`, `tom`, `diretrizes`, `limites`), explicando que chaves desconhecidas são rejeitadas.
7. **Passo 5A — Caminho oficial (harness com `"disponivel": true`, ex.: `hermes`):**
   - Confirmar `--destino` (se fora do padrão `$HERMES_HOME` / `~/.hermes`) e `--perfil` (padrão `sexta-feira`).
   - Executar `sexta-feira --json plano <harness> ...` e apresentar o resumo (`criar`, `atualizar`, `remover`, `conflitos`).
   - Se houver conflitos, mostrar motivo/diff e exigir confirmação explícita antes de passar `--forcar` (que gera `.bak`).
   - Executar `sexta-feira --json aplicar <harness> ...` e validar com `sexta-feira --json verificar <harness> ...` (`exit 0`).
   - Orientar como usar o perfil criado no harness (inspecionando `--help`/documentação local do harness em vez de assumir flags).
8. **Passo 5B — Fallback assistido (harness com `"disponivel": false`, fora do catálogo ou CLI indisponível):**
   - Informar explicitamente ao usuário que a CLI `sexta-feira` ainda não possui adapter oficial para `<harness>` (ou explicar por que a CLI não pôde ser usada) e pedir confirmação para tentar a instalação assistida pelo próprio agente.
   - Ler `recursos/mordomo/persona.toml` e `recursos/mordomo/usuario.toml` do repositório local ou das URLs raw (`https://raw.githubusercontent.com/ygorsimoes/sexta-feira/main/recursos/mordomo/persona.toml` e `usuario.toml`), combinando com eventuais overrides em `~/.config/sexta-feira/*.toml`.
   - Inspecionar as convenções reais do harness atual no ambiente (arquivos de persona/instruções, perfil de usuário, skills).
   - Propor explicitamente ao usuário o(s) arquivo(s) alvo e o conteúdo traduzido antes de gravar. Se o arquivo já existir e contiver conteúdo do usuário, preservar com bloco delimitado (`<!-- sexta-feira:inicio -->` ... `<!-- sexta-feira:fim -->`) ou criar backup `.bak` após confirmação; nunca sobrescrever às cegas.
   - Conferir ao final que os arquivos criados estão legíveis, válidos e alinhados às preferências escolhidas.
9. **Passo 6 — Pós-instalação opcional:** caso o harness suporte skills, oferecer salvar esta `SKILL.md` localmente para facilitar futuras atualizações ou ajustes.

- [ ] **Step 3: Validar consistência da `SKILL.md` contra o repositório**

Run:
```bash
test -f recursos/skills/instalar-sexta-feira/SKILL.md && \
rg -n "x86_64-unknown-linux-musl|aarch64-unknown-linux-musl|aarch64-apple-darwin|SHA256SUMS\.txt|--json|harnesses|plano|aplicar|verificar" recursos/skills/instalar-sexta-feira/SKILL.md
```
Expected: arquivo existente e todas as referências batendo com a CLI e o `cd.yml`.

- [ ] **Step 4: Commit**

```bash
git add recursos/skills/instalar-sexta-feira/SKILL.md
git commit -m "feat(recursos): adiciona skill de instalação guiada por agente"
```

---

### Task 2: Atualizar `README.md` e `docs/visao.md`

**Files:**
- Modify: `README.md`
- Modify: `docs/visao.md`

**Interfaces:**
- Consumes: caminho criado em Task 1 (`recursos/skills/instalar-sexta-feira/SKILL.md`) e URL raw correspondente na branch `main`.
- Produces: `README.md` e `docs/visao.md` atualizados e coerentes.

- [ ] **Step 1: Atualizar `README.md`**

- Na seção **🚀 Início rápido**, adicionar no topo a subseção **"Instalar com seu próprio agente (recomendado)"** com um bloco copiável de prompt pronto apontando para `https://raw.githubusercontent.com/ygorsimoes/sexta-feira/main/recursos/skills/instalar-sexta-feira/SKILL.md`, explicando em uma linha que o agente usa a CLI quando há suporte e faz tradução assistida de `recursos/mordomo/*.toml` nos demais harnesses.
- Manter a instalação manual via binário/código-fonte e os 4 comandos da CLI logo abaixo como alternativa explícita.
- Atualizar a árvore em **🗂️ Estrutura do repositório** para listar `recursos/skills/` junto de `recursos/mordomo/`.

- [ ] **Step 2: Atualizar `docs/visao.md`**

- Na seção **5. Estrutura do repositório**, refletir `recursos/skills/instalar-sexta-feira/SKILL.md` (ou `recursos/skills/`) como primeiro recurso de skill já existente, mantendo o restante da arquitetura intacto.

- [ ] **Step 3: Validar documentação, links e checagens do projeto**

Run:
```bash
test -f recursos/skills/instalar-sexta-feira/SKILL.md && \
cargo run -q -p sexta-feira-cli -- --help && \
cargo run -q -p sexta-feira-cli -- --json harnesses && \
just ci
```
Expected: `SKILL.md` presente, comandos da CLI funcionando e `just ci` (ou comandos `cargo` equivalentes disponíveis no ambiente) passando sem avisos ou falhas.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/visao.md
git commit -m "docs: documenta instalação guiada pelo próprio agente"
```
