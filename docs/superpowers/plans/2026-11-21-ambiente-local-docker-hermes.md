# Plano de Implementação: Ambiente local em Docker para testar a Sexta-feira com o Hermes Agent

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Entregar um laboratório Docker local simples de operar (`./docker/hermes/scripts/lab.sh` e atalhos `just hermes-*`) que sobe o **Hermes Agent** pré-configurado com **OpenRouter**, compila a CLI `sexta-feira` para Linux com cache persistente e permite testar tanto a aplicação direta via CLI quanto a instalação guiada pela `SKILL.md` nos modos `local` e `dia-zero`.

**Architecture:** Um `compose.yaml` em `docker/hermes/` orquestra dois perfis: `local` (serviço `builder` em Rust `1.100.0` com volumes nomeados de cache compilando `--release` para Linux e injetando o binário no container `hermes` + repositório montado em `:ro`) e `dia-zero` (container `hermes-dia-zero` limpo, sem CLI nem repositório montado). Um script `bootstrap-hermes.sh` configura o provedor OpenRouter no perfil padrão e no perfil isolado `sexta-feira` sem tocar em `SOUL.md` ou `memories/USER.md`, e `lab.sh` encapsula toda a operação com um único comando.

**Tech Stack:** Docker Compose v2, imagem oficial multi-arch `nousresearch/hermes-agent`, Rust `1.100.0-slim-bookworm`, Bash, `just` (espelho opcional).

**Spec:** [docs/superpowers/specs/2026-11-21-ambiente-local-docker-hermes-design.md](../specs/2026-11-21-ambiente-local-docker-hermes-design.md)

## Global Constraints

- Toda documentação, scripts, mensagens ao usuário, receitas do `justfile` e commits em `pt-BR` (`AGENTS.md`).
- Trabalho realizado em branch criada a partir da `main` (`feat/ambiente-local-docker-hermes`), nunca direto na `main` (`AGENTS.md`).
- Segredos nunca entram no repositório: `docker/hermes/.env` deve permanecer ignorado pelo `.gitignore`; apenas `docker/hermes/.env.example` é versionado.
- Toolchain Rust do serviço `builder` alinhada a `1.100.0` ([rust-toolchain.toml](../../../rust-toolchain.toml)).
- Nunca montar `./target` do macOS no container Linux, nunca instalar toolchain Rust dentro da imagem runtime do Hermes, nunca usar `network_mode: host` e nunca sobrescrever `/init` (`s6-overlay`).
- Sem alterações em código Rust de produção (`crates/`) ou workflows em `.github/`.

## Review Focus

- **`.env` ausente ou `OPENROUTER_API_KEY` vazia na primeira subida:** ao rodar `lab.sh subir` sem `.env`, o script deve criar `docker/hermes/.env` a partir de `.env.example`, detectar que a chave está vazia e abortar antes de iniciar containers com instrução clara em `pt-BR` sem expor segredos.
- **Inicialização de `/init` (`s6-overlay`), `HERMES_HOME=/opt/data` e permissões `UID`/`GID`:** `lab.sh` deve exportar `HERMES_UID` e `HERMES_GID` automaticamente do host, aguardar o `/opt/data` ficar pronto após `up -d` e garantir `HERMES_HOME=/opt/data` ao invocar `bootstrap-hermes.sh`, `hermes` ou `sexta-feira`.
- **Isolamento do perfil `sexta-feira` sem conflito no motor declarativo:** `bootstrap-hermes.sh` grava apenas `config.yaml` (modelo OpenRouter) em `/opt/data/` e `/opt/data/profiles/sexta-feira/`, sem criar `SOUL.md` nem `memories/USER.md`, garantindo que `sexta-feira plano hermes` planeje `CRIAR` sem conflitos e `verificar hermes` passe com `exit 0`.
- **Separação rigorosa entre modos `local` e `dia-zero`:** no modo `local`, `sexta-feira` precisa estar no `PATH` em sessões `docker compose exec` e o repo em `/workspace/sexta-feira:ro`; no modo `dia-zero`, `which sexta-feira` não deve existir e `curl`/`tar`/`sha256sum` precisam estar funcionais para a `SKILL.md`.
- **`resetar` vs `destruir` com segurança de cache:** `resetar` apaga apenas os containers e volumes `hermes_dados*` mantendo `cargo_registry`, `cargo_git` e `target_linux`; `destruir` remove containers, redes e todos os volumes do Compose (exigindo confirmação ou `--sim`).

---

## Mapa de Arquivos

- Create: `docker/hermes/.env.example` — template comentado com `OPENROUTER_API_KEY`, `HERMES_MODEL`, `HERMES_IMAGE` e portas opcionais.
- Create: `docker/hermes/compose.yaml` — definição dos serviços `builder`, `hermes` (`local`) e `hermes-dia-zero` (`dia-zero`), além dos volumes nomeados.
- Create: `docker/hermes/scripts/bootstrap-hermes.sh` — provisionamento idempotente de `config.yaml`/OpenRouter no perfil padrão e no perfil `sexta-feira`.
- Create: `docker/hermes/scripts/lab.sh` — CLI ergonômica (`subir`, `chat`, `chat-padrao`, `shell`, `status`, `logs`, `parar`, `resetar`, `destruir`, `ajuda`) sem depender de `just`.
- Modify: `justfile` — adicionar receitas `hermes-*` delegando para `docker/hermes/scripts/lab.sh`.
- Create: `docs/ambiente-local-docker.md` — guia objetivo de uso do ambiente local Docker.
- Modify: `README.md` — adicionar link discreto para o guia de ambiente local Docker.

---

### Task 1: Branch, inspeção da imagem oficial e infraestrutura Compose (`compose.yaml` + `.env.example`)

**Files:**
- Create: `docker/hermes/.env.example`
- Create: `docker/hermes/compose.yaml`

**Interfaces:**
- Consumes:
  - `rust-toolchain.toml` (`1.100.0`) para a tag da imagem `builder`.
  - `.gitignore` (regras `.env`, `.env.*`, `!.env.example`).
- Produces:
  - Variáveis de ambiente esperadas no `.env`: `OPENROUTER_API_KEY`, `HERMES_MODEL` (padrão sugerido `anthropic/claude-sonnet-4`), `HERMES_IMAGE` (padrão `nousresearch/hermes-agent:latest`), `HERMES_DASHBOARD` (padrão `1`), `HERMES_GATEWAY_PORTA` (padrão `8642`), `HERMES_DASHBOARD_PORTA` (padrão `9119`).
  - Serviços Compose com `name: sexta-feira-hermes`: `builder` (profile `local`), `hermes` (profile `local`), `hermes-dia-zero` (profile `dia-zero`).
  - Volumes nomeados: `hermes_dados`, `hermes_dados_dia_zero`, `cargo_registry`, `cargo_git`, `target_linux`, `sexta_feira_bin`.

- [ ] **Step 1: Criar a branch a partir da `main` e inspecionar a imagem oficial do Hermes**

```bash
git checkout -b feat/ambiente-local-docker-hermes
docker pull nousresearch/hermes-agent:latest
docker run --rm --entrypoint /bin/sh nousresearch/hermes-agent:latest -c 'id; env | grep -E "HERMES|PATH"; which hermes curl tar sha256sum git python3 || true; ls -la /opt/hermes/docker/main-wrapper.sh || true'
```
Expected: branch criada; confirmação exata de `PATH`, `HERMES_HOME`, usuário padrão e ferramentas disponíveis dentro de `nousresearch/hermes-agent:latest` (se `curl`, `tar` ou `sha256sum` estiverem ausentes, registrar para adicionar apenas o necessário ou tratar na imagem base).

- [ ] **Step 2: Validar validação do `.gitignore` para `docker/hermes/.env`**

Run: `git check-ignore -v docker/hermes/.env docker/hermes/.env.example`
Expected: `docker/hermes/.env` ignorado pela regra `.env` e `docker/hermes/.env.example` **não** ignorado (exit 0/1 conforme esperado).

- [ ] **Step 3: Criar `docker/hermes/.env.example`**

Incluir apenas variáveis necessárias com comentários em `pt-BR`:
- `OPENROUTER_API_KEY=` (obrigatória, sem valor padrão)
- `HERMES_MODEL=anthropic/claude-sonnet-4`
- `HERMES_IMAGE=nousresearch/hermes-agent:latest`
- `HERMES_DASHBOARD=1`
- `HERMES_GATEWAY_PORTA=8642`
- `HERMES_DASHBOARD_PORTA=9119`

- [ ] **Step 4: Criar `docker/hermes/compose.yaml`**

Definir:
- `name: sexta-feira-hermes`
- Âncora `x-hermes-base` com `image: ${HERMES_IMAGE:-nousresearch/hermes-agent:latest}`, `restart: unless-stopped`, `command: ["gateway", "run"]`, `ports` mapeando `127.0.0.1:${HERMES_GATEWAY_PORTA:-8642}:8642` e `127.0.0.1:${HERMES_DASHBOARD_PORTA:-9119}:9119`, `environment` com `HERMES_HOME=/opt/data`, `HERMES_UID=${HERMES_UID:-1000}`, `HERMES_GID=${HERMES_GID:-1000}`, `HERMES_DASHBOARD=${HERMES_DASHBOARD:-1}`, `OPENROUTER_API_KEY`, `HERMES_MODEL`, `TERM=xterm-256color`, e mount `./scripts:/opt/sexta-feira/scripts:ro`.
- Serviço `builder` (`profiles: ["local"]`, `image: rust:1.100.0-slim-bookworm`, `restart: "no"`): monta `../..:/src:ro`, `cargo_registry:/usr/local/cargo/registry`, `cargo_git:/usr/local/cargo/git`, `target_linux:/target-linux`, `sexta_feira_bin:/out`; roda `cargo build --release --locked --manifest-path /src/Cargo.toml --package sexta-feira-cli`, copia `/target-linux/release/sexta-feira` para `/out/sexta-feira` e aplica `chmod 0755 /out/sexta-feira`.
- Serviço `hermes` (`profiles: ["local"]`): estende `x-hermes-base`, `depends_on: builder: condition: service_completed_successfully`, `working_dir: /workspace/sexta-feira`, `PATH` incluindo `/opt/sexta-feira/bin` (usando o `PATH` real inspecionado no Step 1), mounts: `hermes_dados:/opt/data`, `sexta_feira_bin:/opt/sexta-feira/bin:ro`, `../..:/workspace/sexta-feira:ro`.
- Serviço `hermes-dia-zero` (`profiles: ["dia-zero"]`): estende `x-hermes-base`, sem `builder`, sem `sexta_feira_bin` e sem repo montado; monta apenas `hermes_dados_dia_zero:/opt/data` e `./scripts:/opt/sexta-feira/scripts:ro`.
- Todos os volumes nomeados declarados explicitamente na seção `volumes:`.

- [ ] **Step 5: Validar a sintaxe do `compose.yaml` nos dois perfis**

Run:
```bash
docker compose -f docker/hermes/compose.yaml --env-file docker/hermes/.env.example --profile local config >/dev/null && \
docker compose -f docker/hermes/compose.yaml --env-file docker/hermes/.env.example --profile dia-zero config >/dev/null
```
Expected: exit 0 sem avisos ou erros de interpolação.

- [ ] **Step 6: Commit**

```bash
git add docker/hermes/.env.example docker/hermes/compose.yaml
git commit -m "build(docker): adiciona compose e modelo env para laboratorio hermes"
```

---

### Task 2: Script de bootstrap não interativo (`docker/hermes/scripts/bootstrap-hermes.sh`)

**Files:**
- Create: `docker/hermes/scripts/bootstrap-hermes.sh`

**Interfaces:**
- Consumes:
  - Variáveis de ambiente dentro do container: `HERMES_HOME` (padrão `/opt/data`), `OPENROUTER_API_KEY`, `HERMES_MODEL` (padrão `anthropic/claude-sonnet-4`).
  - CLI `hermes` dentro da imagem `nousresearch/hermes-agent` (ou fallback gerando `config.yaml` / `.env` com permissão `0600`).
- Produces:
  - Script executável (`chmod +x`) aceitando flag opcional `--reconfigurar`.
  - Garante `/opt/data/config.yaml` (perfil padrão) e `/opt/data/profiles/sexta-feira/config.yaml` (perfil isolado) configurados com `provider: openrouter`, `default: <HERMES_MODEL>` e `base_url: https://openrouter.ai/api/v1`, e `OPENROUTER_API_KEY` disponível nos arquivos `.env` correspondentes, **sem** criar `SOUL.md` nem `memories/USER.md`.

- [ ] **Step 1: Validar comportamento real de `hermes config set` na imagem oficial**

Run (usando diretório temporário dentro de um container descartável):
```bash
docker run --rm --entrypoint /bin/sh -e HERMES_HOME=/tmp/htest -e OPENROUTER_API_KEY=sk-test nousresearch/hermes-agent:latest -c \
  'mkdir -p /tmp/htest && hermes config set model.provider openrouter && hermes config set model.default anthropic/claude-sonnet-4 && ls -la /tmp/htest && cat /tmp/htest/config.yaml'
```
Expected: confirmar se `hermes config set` funciona em diretório vazio e qual estrutura exata grava em `config.yaml` / `.env` (incluindo com `-p sexta-feira`). Se `hermes` estiver em `/opt/hermes/.venv/bin` ou exigir `python`, registrar o caminho exato encontrado.

- [ ] **Step 2: Implementar `docker/hermes/scripts/bootstrap-hermes.sh`**

Implementar em `bash`/`sh` compatível com o container:
- `set -eu`
- Checar se `OPENROUTER_API_KEY` está não vazia (sair com erro claro em `pt-BR` sem imprimir o valor da chave).
- Aceitar `--reconfigurar` para forçar regravação de `model.*` mesmo quando `config.yaml` já existir.
- Configurar o perfil padrão em `$HERMES_HOME` e o perfil isolado `sexta-feira` em `$HERMES_HOME/profiles/sexta-feira` (criando apenas o diretório do perfil, `config.yaml` e `.env` com `chmod 0600`), usando `hermes config set` (se suportar o fluxo validado no Step 1) ou escrita segura/atômica de `config.yaml` + `.env` preservando qualquer arquivo já existente quando não houver `--reconfigurar`.
- Nunca criar ou modificar `SOUL.md` ou `memories/USER.md`.

- [ ] **Step 3: Tornar executável e validar sintaxe e idempotência**

Run:
```bash
chmod +x docker/hermes/scripts/bootstrap-hermes.sh
sh -n docker/hermes/scripts/bootstrap-hermes.sh
```
Testar em container efêmero com volume `/tmp` validando:
1. Falha esperada quando `OPENROUTER_API_KEY=""`.
2. Criação de `config.yaml` no global e em `profiles/sexta-feira/` sem criar `SOUL.md` nem `memories/USER.md`.
3. Segunda execução sem `--reconfigurar` preserva conteúdo existente (idempotência).

- [ ] **Step 4: Commit**

```bash
git add docker/hermes/scripts/bootstrap-hermes.sh
git commit -m "feat(docker): adiciona bootstrap nao interativo do OpenRouter no Hermes"
```

---

### Task 3: Script unificado `docker/hermes/scripts/lab.sh` e atalhos no `justfile`

**Files:**
- Create: `docker/hermes/scripts/lab.sh`
- Modify: `justfile:1-38`

**Interfaces:**
- Consumes:
  - `docker/hermes/compose.yaml`, `docker/hermes/.env.example`, `docker/hermes/scripts/bootstrap-hermes.sh`.
- Produces:
  - `./docker/hermes/scripts/lab.sh <comando> [args]` com comandos:
    - `subir [local|dia-zero] [--reconfigurar]`
    - `chat [perfil] [local|dia-zero]` (padrão: `sexta-feira`, `local`)
    - `chat-padrao [local|dia-zero]`
    - `shell [local|dia-zero]`
    - `status`
    - `logs [local|dia-zero]`
    - `parar`
    - `resetar [local|dia-zero|todos]` (remove containers + `hermes_dados*`, mantendo caches Rust)
    - `destruir [--sim]` (remove containers, rede e **todos** os volumes do projeto Compose)
    - `ajuda`
  - Receitas `hermes-subir`, `hermes-chat`, `hermes-chat-padrao`, `hermes-shell`, `hermes-status`, `hermes-logs`, `hermes-parar`, `hermes-resetar`, `hermes-destruir` no `justfile`.

- [ ] **Step 1: Implementar `docker/hermes/scripts/lab.sh`**

Garantir:
- `set -euo pipefail`, resolvendo a raiz do repo relativamente ao próprio script (`SCRIPT_DIR`, funciona de qualquer diretório).
- Exportação automática de `HERMES_UID="$(id -u)"` e `HERMES_GID="$(id -g)"` se não definidas no `.env`.
- Pré-check de `.env`: se `docker/hermes/.env` não existir em `subir`, copia `.env.example` para `.env` e sai com instrução amigável em `pt-BR` indicando onde colar `OPENROUTER_API_KEY`. Se existir e `OPENROUTER_API_KEY` estiver vazia, falha antes do `docker compose up` sem vazar chaves.
- Em `subir`: executa `docker compose ... --profile <modo> up -d --remove-orphans`, aguarda o container responder (`docker compose exec` readiness loop curto de até 30s) e executa `/opt/sexta-feira/scripts/bootstrap-hermes.sh` dentro do container como usuário adequado (respeitando `HERMES_HOME=/opt/data`), imprimindo no final os próximos comandos prontos (`chat`, `chat-padrao`, `shell`).
- Em `chat` / `chat-padrao`: verifica se o serviço do modo está rodando e executa `docker compose ... exec -it -e HERMES_HOME=/opt/data <serviço> hermes ...` (se perfil `default`, `hermes chat`; caso contrário, `hermes -p <perfil> chat`).
- Em `status`: mostra `docker compose ... ps -a` e volumes do projeto `sexta-feira-hermes`; se `hermes` (`local`) estiver rodando, executa `sexta-feira verificar hermes` (sem falhar o comando `status` se ainda não tiver sido aplicado, apenas informando o estado).
- Em `resetar`: derruba containers e remove apenas `sexta-feira-hermes_hermes_dados` / `sexta-feira-hermes_hermes_dados_dia_zero` (conforme filtro `local`, `dia-zero` ou `todos`), preservando `cargo_registry`, `cargo_git` e `target_linux`.
- Em `destruir`: pede confirmação explícita (`[s/N]`) a menos que `--sim` seja passado, e executa `docker compose -f ... --profile local --profile dia-zero down -v --remove-orphans`.

- [ ] **Step 2: Atualizar `justfile` com receitas `hermes-*`**

Adicionar ao final de `justfile` (mantendo `indent_size = 4` conforme `.editorconfig` e comentários `#` em `pt-BR` para aparecerem no `just --list`) receitas finas delegando para `./docker/hermes/scripts/lab.sh`.

- [ ] **Step 3: Validar sintaxe e fluxos de erro de `lab.sh`**

Run:
```bash
chmod +x docker/hermes/scripts/lab.sh
bash -n docker/hermes/scripts/lab.sh
./docker/hermes/scripts/lab.sh ajuda
```
Testar também o comportamento quando `docker/hermes/.env` não existe (usando estado limpo e removendo `.env` temporário de teste depois, se criado): deve criar `.env` a partir de `.env.example` e instruir o preenchimento da `OPENROUTER_API_KEY` com saída amigável.

- [ ] **Step 4: Commit**

```bash
git add docker/hermes/scripts/lab.sh justfile
git commit -m "feat(docker): adiciona script unificado e receitas just para laboratorio hermes"
```

---

### Task 4: Documentação e verificação ponta a ponta em Docker real

**Files:**
- Create: `docs/ambiente-local-docker.md`
- Modify: `README.md`

**Interfaces:**
- Consumes:
  - Comandos reais implementados em `docker/hermes/scripts/lab.sh` e `justfile`.
- Produces:
  - Guia `docs/ambiente-local-docker.md` em `pt-BR` cobrindo pré-requisitos (Docker Desktop/Engine, chave OpenRouter), início rápido em 3 passos, modos `local` vs `dia-zero`, fluxos CLI e instalação guiada via `SKILL.md`, reset rápido vs destruição completa e solução de problemas comuns.
  - Link discreto no `README.md` na seção de documentação.

- [ ] **Step 1: Escrever `docs/ambiente-local-docker.md` e linkar em `README.md`**

Manter conciso, com exemplos reais testados para `./docker/hermes/scripts/lab.sh` e nota sobre os atalhos `just hermes-*`.

- [ ] **Step 2: Executar verificação ponta a ponta em Docker real (sem expor segredos)**

Usando um `.env` temporário local (com chave sintética `sk-or-test-local` apenas para testar inicialização, bootstrap e CLI — removendo o `.env` temporário ao final ou usando `--sim` no `destruir` após validar `resetar`):
1. `./docker/hermes/scripts/lab.sh subir local` → confirmar que `builder` compila `sexta-feira`, `hermes` sobe saudável e `bootstrap-hermes.sh` configura `/opt/data/config.yaml` e `/opt/data/profiles/sexta-feira/config.yaml`.
2. Verificar dentro de `hermes`:
   - `which sexta-feira` e `sexta-feira --version`
   - `sexta-feira --json plano hermes` → `CRIAR` para `SOUL.md` e `memories/USER.md`, `conflitos: 0`
   - `sexta-feira aplicar hermes` e `sexta-feira verificar hermes` → `exit 0`
   - `hermes -p sexta-feira config` (ou `status`) reconhece o modelo OpenRouter configurado.
3. `./docker/hermes/scripts/lab.sh subir dia-zero` → verificar dentro de `hermes-dia-zero` que `which sexta-feira` **não** existe, `/workspace/sexta-feira` **não** existe e `curl`, `tar`, `sha256sum` existem.
4. `./docker/hermes/scripts/lab.sh resetar` → confirmar que `hermes_dados*` foram removidos e `target_linux` / `cargo_registry` foram preservados; subir `local` novamente e observar rebuild incremental rápido.
5. `./docker/hermes/scripts/lab.sh destruir --sim` e remover o `.env` temporário de teste se criado para não deixar lixo no working tree.

- [ ] **Step 3: Rodar verificações locais do repositório**

Run:
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```
Expected: tudo passando sem avisos nem alterações em arquivos Rust.

- [ ] **Step 4: Commit**

```bash
git add docs/ambiente-local-docker.md README.md
git commit -m "docs: documenta ambiente local em Docker com Hermes Agent"
```

---

## Self-Review Checklist

- [x] Todos os requisitos do design doc (`compose.yaml`, `.env.example`, `bootstrap-hermes.sh`, `lab.sh`, `justfile`, dois modos `local`/`dia-zero`, volumes separados, OpenRouter sem wizard, documentação) estão cobertos nas Tasks 1–4.
- [x] Nenhum placeholder vago ("TBD", "tratar edge cases") e assinaturas/comandos consistentes entre tarefas.
- [x] Todas as 5 condições do **Review Focus** possuem passos de validação explícitos antes de concluir.
