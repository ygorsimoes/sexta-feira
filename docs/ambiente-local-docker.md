# Ambiente Docker (CI, CLI e laboratório Hermes)

Toda a infraestrutura Docker e de automação do projeto é operada diretamente da raiz pelo [justfile](../justfile), sem necessidade de entrar em subpastas ou instalar ferramentas Rust/CI na máquina local.

## Pré-requisitos

- Docker Desktop ou Docker Engine com `docker compose` (v2).
- [`just`](https://github.com/casey/just) na máquina local (no macOS: `brew install just`).
- Uma chave de API do **OpenRouter** (`OPENROUTER_API_KEY`) para o laboratório do Hermes Agent.

## Arquitetura do ambiente Docker

- `docker/compose.yaml` — orquestração unificada com perfis `ci`, `cli`, `local` e `dia-zero`, compartilhando caches de Cargo e separando `target_ci` e `target_linux` em volumes nomeados (sem conflitar com `./target` do host).
- `docker/ci/Dockerfile` — imagem padronizada de desenvolvimento e CI (`Rust 1.100.0`, `rustfmt`, `clippy`, `musl-tools`, `cargo-nextest`, `cargo-deny`, `cargo-machete`, `zizmor`, `just`).
- `docker/cli/Dockerfile` — build multi-stage gerando o binário estático `musl` e uma imagem runtime mínima em Alpine (`sexta-feira-cli:local`) sob usuário não-root.
- `docker/hermes/` — bootstrap não interativo (`bootstrap.sh`) e orquestrador (`lab.sh`) do laboratório Hermes Agent.
- `.env.example` na raiz — modelo único de configuração local (copiado para `.env` na raiz, ignorado pelo Git).

---

## 1. Desenvolvimento, CI e Build em Docker

Sem instalar `cargo-nextest`, `cargo-deny`, `cargo-machete` ou `zizmor` no seu host:

```bash
just docker-ci      # executa fmt, clippy, machete, doc, nextest, doctests, cargo-deny e zizmor
just docker-lint    # apenas formatação, clippy e machete
just docker-test    # apenas nextest e doctests
just docker-build   # gera imagem `sexta-feira-cli:local` e exporta binário estático em ./dist/sexta-feira
just docker-shell   # abre shell interativo dentro do container de CI
```

## 2. CLI `sexta-feira` containerizada

Para rodar comandos da CLI em uma sandbox isolada (`cli_dados:/dados` e repositório em `/workspace:ro`):

```bash
just cli --json harnesses
just cli diagnostico
just cli iniciar --nao-interativo --tratamento "senhor"
just cli aplicar hermes
just cli verificar hermes
```

---

## 3. Laboratório local com Hermes Agent

### Passo 1 — Configure `.env` na raiz

```bash
cp .env.example .env
```

Preencha `OPENROUTER_API_KEY=...` em `.env` (e ajuste `HERMES_MODEL` se quiser trocar o padrão `anthropic/claude-sonnet-4`).

> Se rodar `just subir` antes de copiar `.env`, o fluxo cria `.env` a partir de `.env.example` e informa o que preencher.

### Passo 2 — Suba o laboratório

```bash
just subir
```

No modo padrão (`local`), `just subir` compila a CLI Linux em cache, sobe `nousresearch/hermes-agent:latest` (`HERMES_HOME=/opt/data`) com o repositório montado em `/workspace/sexta-feira:ro`, aguarda a inicialização do `s6-overlay` e configura o provedor **OpenRouter** tanto no perfil padrão quanto no perfil isolado `sexta-feira` (sem criar `SOUL.md` nem `memories/USER.md`).

### Passo 3 — Escolha o fluxo de teste

#### Fluxo A — Pedir ao próprio Hermes para instalar a Sexta-feira

1. Abra o chat no perfil padrão:
   ```bash
   just chat-padrao
   ```
2. Peça ao agente para ler e seguir a skill local (ou a URL do GitHub):
   ```text
   Leia /workspace/sexta-feira/recursos/skills/instalar-sexta-feira/SKILL.md e instale a Sexta-feira neste ambiente.
   ```
3. Depois que ele concluir, converse já no perfil **Sexta-feira**:
   ```bash
   just chat
   ```

#### Fluxo B — Aplicar manualmente pela CLI dentro do container

```bash
just shell
# Dentro do container:
sexta-feira diagnostico
sexta-feira iniciar
sexta-feira plano hermes
sexta-feira aplicar hermes
sexta-feira verificar hermes
exit

just chat
```

> **Dica:** evite rodar comandos `hermes -p sexta-feira ...` (como `status`, `config show` ou `chat`) *antes* de aplicar o setup com `sexta-feira aplicar hermes`, pois o Hermes cria automaticamente um `SOUL.md` padrão ao inicializar o perfil — o que gerará `CONFLITO` no próximo `plano` (resolvível com `--forcar`).

#### Fluxo C — Modo `dia-zero` (VPS limpa sem CLI pré-instalada)

Para testar o Hermes baixando a Release do GitHub (`SHA256SUMS.txt`) ou usando fallback assistido sem o binário local nem o repositório montado:

```bash
just subir dia-zero
just chat-padrao dia-zero
# Após instalar:
just chat sexta-feira dia-zero
```

---

## Gerenciamento rápido (`just --list`)

| Comando | O que faz |
|---|---|
| `just subir [local\|dia-zero] [--reconfigurar]` | Compila (em `local`), sobe o Hermes e configura OpenRouter |
| `just chat [perfil] [local\|dia-zero]` | Abre `hermes -p <perfil> chat` (padrão: `sexta-feira`, `local`) |
| `just chat-padrao [local\|dia-zero]` | Abre `hermes chat` no perfil padrão |
| `just shell [local\|dia-zero]` | Abre `bash` como usuário `hermes` no container |
| `just status` | Mostra containers, volumes e roda `sexta-feira verificar hermes` |
| `just logs [local\|dia-zero]` | Acompanha os logs do container Hermes |
| `just parar` | Para apenas os containers do laboratório preservando dados e caches |
| `just resetar [local\|dia-zero\|todos]` | Apaga só os dados do Hermes, preservando CI, CLI e caches Rust |
| `just destruir [--sim]` | Remove **todos** os containers, redes e volumes Docker do projeto |

## Solução de problemas

- **Alterei `HERMES_MODEL` em `.env`:** rode `just subir local --reconfigurar` para atualizar os `config.yaml` existentes. Se mudou apenas `OPENROUTER_API_KEY`, `just subir` já sincroniza a chave automaticamente.
- **Quero retestar a instalação do zero rápido:** rode `just resetar` e depois `just subir` (preserva o cache de compilação Rust).
- **Ainda não tenho `just` instalado no macOS:** instale com `brew install just` ou chame diretamente `./docker/hermes/lab.sh <comando>`.
