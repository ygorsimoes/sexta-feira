# Design: Ambiente local em Docker para testar a Sexta-feira com o Hermes Agent

Data: 2026-11-21
Status: rascunho para revisão

## 1. Intenção e porquê

Permitir subir localmente, com um comando simples e sem configuração manual repetitiva além das credenciais/modelo do **OpenRouter**, um ambiente isolado, rápido e reprodutível do **Hermes Agent** em Docker para validar a **Sexta-feira** de ponta a ponta — tanto aplicando o setup diretamente via CLI (`sexta-feira iniciar/plano/aplicar/verificar`) quanto pedindo ao próprio agente Hermes para instalar a Sexta-feira seguindo [recursos/skills/instalar-sexta-feira/SKILL.md](../../../recursos/skills/instalar-sexta-feira/SKILL.md).

## 2. Objetivos, não-objetivos e anti-objetivos

### Objetivos
- Executar a imagem oficial multi-arch `nousresearch/hermes-agent` nativamente em `linux/arm64` (Apple Silicon) e `linux/amd64`, preservando o entrypoint `/init` (`s6-overlay`) e `HERMES_HOME=/opt/data`.
- Configurar provedor e modelo (`OpenRouter`) de forma declarativa/não interativa a partir de um `.env` local (não versionado), evitando o wizard interativo `hermes setup` a cada subida ou reset.
- Garantir que o perfil isolado `sexta-feira` (`/opt/data/profiles/sexta-feira/`) já consiga usar o modelo OpenRouter configurado ao abrir `hermes -p sexta-feira chat`, sem interferir em `SOUL.md` nem em `memories/USER.md` gerenciados pela CLI `sexta-feira`.
- Oferecer **dois modos de execução** via Docker Compose:
  - **Modo `local` (padrão):** compila a CLI `sexta-feira` para Linux em um serviço builder Rust dedicado com cache persistente de Cargo e `target`, injeta o binário atualizado no `PATH` do container do Hermes e monta o repositório em somente leitura (`/workspace/sexta-feira:ro`) para testar alterações ainda não publicadas.
  - **Modo `dia-zero`:** sobe o container do Hermes sem a CLI pré-instalada e sem o repositório montado, simulando uma VPS limpa onde o próprio Hermes segue a `SKILL.md` (baixando a Release do GitHub com verificação de `SHA256SUMS.txt` ou usando fallback).
- Fornecer atalhos simples no [justfile](../../../justfile) para subir, abrir chat (no perfil padrão ou `sexta-feira`), rodar comandos da CLI dentro do container, inspecionar logs e resetar o estado em segundos.

### Não-objetivos
- Alterar código Rust de produção (`sexta-feira-core`, `sexta-feira-adapters`, `sexta-feira-cli`) ou os fluxos de CI/CD existentes.
- Prever infraestrutura Docker para outros harnesses (`OpenClaw`, `Claude Code`, `OpenCode`) antes de seus adapters existirem.
- Executar automaticamente canais externos (ex.: Telegram/Discord) por padrão; o foco inicial é TUI/chat interativo no container com gateway ativo em background.

### Anti-objetivos
- Montar o `./target` do macOS dentro do container Linux (incompatibilidade Mach-O vs ELF e degradação severa de I/O).
- Instalar toolchain Rust completo dentro da imagem runtime do Hermes (inchaço >1,5 GB e mistura de papéis).
- Commitar qualquer segredo (`OPENROUTER_API_KEY`, `.env` real) no repositório.
- Usar `network_mode: host` (incompatível com Docker Desktop no macOS) ou sobrescrever `/init` quebrando `s6-overlay` e permissões de `/opt/data`.

## 3. Restrições

- Tudo (arquivos, comentários, receitas do `justfile` e documentação) em `pt-BR`, respeitando [AGENTS.md](../../../AGENTS.md).
- Segredos nunca entram no repositório: `docker/hermes/.env` deve permanecer ignorado pelo `.gitignore` (já coberto por `.env` e `.env.*`), versionando apenas `docker/hermes/.env.example`.
- Toolchain Rust do builder alinhada à versão fixada em [rust-toolchain.toml](../../../rust-toolchain.toml) (`1.100.0`).
- Preservar `HERMES_HOME=/opt/data` e o usuário/permissões esperados pela imagem oficial (`HERMES_UID`/`HERMES_GID`).

## 4. Abordagens consideradas

1. **Compose Híbrido: Hermes oficial + Builder Rust com cache + Modos `local` e `dia-zero` *(Escolhida)*:** combina imagem oficial leve do Hermes, compilação Linux nativa incremental com volumes nomeados para Cargo/target e alternância limpa entre testar mudanças locais e fluxo de usuário final.
2. **Container único "Tudo-em-Um" (Hermes + Rust na mesma imagem):** permite `cargo build` dentro do próprio container do Hermes, mas adiciona >1,5 GB na imagem e impede simular uma VPS limpa sem toolchain Rust.
3. **Hermes puro no Docker + CLI rodando no macOS:** simples, porém não testa a CLI no Linux nem permite ao agente Hermes dentro do container executar a instalação e a `SKILL.md` sobre código local.

## 5. Desenho da solução

### 5.1 Organização dos arquivos

```text
docker/
└── hermes/
    ├── compose.yaml               # Serviços: builder, hermes (perfil local) e hermes-dia-zero (perfil dia-zero)
    ├── .env.example               # Modelo de configuração (OPENROUTER_API_KEY, HERMES_MODEL)
    └── scripts/
        ├── lab.sh                 # Ponto único de gerenciamento (subir, chat, shell, status, resetar, destruir)
        └── bootstrap-hermes.sh    # Configura config.yaml/.env sem wizard para default e perfil sexta-feira
```

Além de:
- Receitas no [justfile](../../../justfile) com prefixo `hermes-*` aparecendo no `just --list` e delegando para `docker/hermes/scripts/lab.sh` (assim funciona tanto com `just hermes-subir` quanto direto via `./docker/hermes/scripts/lab.sh subir`).
- Guia enxuto de uso em `docs/` (ou seção curta linkada no [README.md](../../../README.md)) explicando pré-requisitos, criação do `.env` e os fluxos de teste.

### 5.2 Serviços e volumes no `compose.yaml`

- **Volumes nomeados (performance no macOS e reset rápido):**
  - `hermes_dados` → montado em `/opt/data` (`HERMES_HOME`). Evita lentidão de VirtioFS em `state.db` (SQLite) e permite limpar todo o estado do Hermes apagando apenas esse volume.
  - `cargo_registry` e `cargo_git` → caches do Cargo no serviço `builder`.
  - `target_linux` → diretório `CARGO_TARGET_DIR=/target-linux` persistente do builder para recompilações incrementais em poucos segundos.
  - `sexta_feira_bin` → volume pequeno compartilhado onde o `builder` grava `/out/sexta-feira` (`chmod 0755`) e o serviço `hermes` (modo `local`) monta em `/opt/sexta-feira/bin:ro`, incluído no `PATH`.
- **Serviço `builder` (profile `local`):**
  - Imagem `rust:1.100.0-slim-bookworm` (compatível com [rust-toolchain.toml](../../../rust-toolchain.toml) e arquiteturas `arm64`/`amd64`).
  - Monta a raiz do repositório em `/src:ro`.
  - Executa `cargo build --release --locked --manifest-path /src/Cargo.toml --package sexta-feira-cli` com `CARGO_TARGET_DIR=/target-linux`, copia `/target-linux/release/sexta-feira` para `/out/sexta-feira` e finaliza (`restart: "no"`).
  - Decisão técnica assumida: usar `--release` (em vez de debug) garante que o binário entregue ao container tenha comportamento e tamanho próximos da release real, com custo incremental baixo graças ao volume `target_linux`.
- **Serviço `hermes` (profile `local`) e `hermes-dia-zero` (profile `dia-zero`):**
  - Compartilham ancoragem YAML (`x-hermes-base`) com imagem `nousresearch/hermes-agent:latest` (ajustável via variável `HERMES_IMAGE` no `.env` para fixar tag se desejado).
  - Comando padrão: `gateway run` (mantém o container ativo via `s6-overlay`, servindo gateway/dashboard nas portas configuráveis `8642` e `9119`, e aceitando sessões interativas simultâneas via `docker compose exec -it ...`).
  - Variáveis de ambiente passadas do `.env`: `OPENROUTER_API_KEY`, `HERMES_MODEL`, `HERMES_UID`, `HERMES_GID`, `HERMES_DASHBOARD`, além de `HERMES_HOME=/opt/data`.
  - **Diferenças entre os dois modos:**
    - `hermes` (`local`): `depends_on: builder (service_completed_successfully)`, monta `sexta_feira_bin:/opt/sexta-feira/bin:ro` (adicionado ao `PATH`), monta `../..:/workspace/sexta-feira:ro` e define `WORKDIR` ou acesso fácil a `/workspace/sexta-feira`.
    - `hermes-dia-zero` (`dia-zero`): não depende de `builder`, não monta `sexta_feira_bin` nem o repositório local. Usa volume separado `hermes_dados_dia_zero` para evitar contaminar um ambiente limpo de dia-zero com testes anteriores do modo local — e `just hermes-resetar` / `hermes-destruir` limpam ambos ou o modo informado.

### 5.3 Bootstrap não interativo do OpenRouter (sem wizard)

Como o Hermes isola perfis em `/opt/data/profiles/<nome>/` e não herda `config.yaml` do perfil global:

- Um script idempotente (`docker/hermes/scripts/bootstrap-hermes.sh`, montado em somente leitura e executado logo após o container ficar pronto pelo comando `just hermes-subir`):
  - **Decisão técnica assumida:** rodar o bootstrap via `docker compose exec` dentro do container já inicializado pelo `/init` (encapsulado em `lab.sh subir`, com espera curta pela inicialização de `/opt/data`). Isso evita interceptar ou modificar `/init` / `main-wrapper.sh` da imagem oficial, reduzindo risco de quebra em atualizações do Hermes.
  - O que o bootstrap faz dentro do container (`HERMES_HOME=/opt/data`):
    1. Garante que `OPENROUTER_API_KEY` esteja definida (falha com mensagem clara em `pt-BR` se estiver vazia).
    2. Escreve/atualiza `/opt/data/config.yaml` (somente se ausente, ou via flag `--reconfigurar`, preservando edições manuais feitas durante testes) configurando `model.provider: openrouter`, `model.default: "${HERMES_MODEL}"` e `model.base_url: "https://openrouter.ai/api/v1"` usando `hermes config set` (ou fallback gerando YAML mínimo se necessário).
    3. Grava também a configuração mínima de modelo em `/opt/data/profiles/sexta-feira/config.yaml` (apenas `model.*`, **sem** criar `SOUL.md` nem `memories/USER.md`). Assim:
       - `sexta-feira plano hermes` continua vendo `SOUL.md` e `memories/USER.md` como `CRIAR`;
       - `sexta-feira aplicar hermes` e `verificar hermes` operam sem conflitos;
       - Ao rodar `hermes -p sexta-feira chat`, o perfil da Sexta-feira já responde usando o modelo configurado no OpenRouter sem exigir novo `hermes model`/`hermes setup`.
       - A chave `OPENROUTER_API_KEY` vem do ambiente do container (`.env` do Compose) e/ou é gravada em `/opt/data/.env` e `/opt/data/profiles/sexta-feira/.env` via `hermes config set` ou arquivo com permissão restrita (`0600`).

### 5.4 Ergonomia de gerenciamento e fluxos de teste

Toda a operação do laboratório fica encapsulada em `docker/hermes/scripts/lab.sh` (funcionando apenas com `bash` + `docker`, sem exigir `just` instalado) e espelhada por comandos curtos no [justfile](../../../justfile):

- **Pré-check automático de `.env` e `UID`/`GID`:** na primeira execução de `subir`, se `docker/hermes/.env` ainda não existir, o script copia `.env.example` para `.env` e orienta em uma linha para preencher `OPENROUTER_API_KEY`. `HERMES_UID` e `HERMES_GID` são preenchidos automaticamente com `id -u` / `id -g` do host se não estiverem definidos.
- **Comandos principais (`just --list`):**
  - `just hermes-subir [modo="local"]` — compila (no modo `local`), sobe o container e roda `bootstrap-hermes.sh` automaticamente.
  - `just hermes-chat [perfil="sexta-feira"] [modo="local"]` — abre `hermes -p <perfil> chat` (ou `hermes chat` se `perfil="default"`).
  - `just hermes-chat-padrao [modo="local"]` — atalho direto para abrir o chat no perfil padrão (para testar a instalação guiada pedindo ao próprio Hermes).
  - `just hermes-shell [modo="local"]` — abre shell no container com `sexta-feira` no `PATH` e o repo em `/workspace/sexta-feira`.
  - `just hermes-status` — mostra estado dos containers/volumes e, no modo `local`, executa `sexta-feira verificar hermes` se o binário existir.
  - `just hermes-logs [modo="local"]` — acompanha os logs do container.
  - `just hermes-parar` — para os containers preservando dados e cache.
  - `just hermes-resetar` — apaga os containers e apenas os volumes de dados do Hermes (`hermes_dados*`), mantendo o cache de compilação Rust para retestar instalações limpas em segundos.
  - `just hermes-destruir` — **deleta tudo** do ambiente Docker (containers, rede, todos os volumes incluindo caches Rust e imagens pulladas/locais opcionais se solicitado, com confirmação simples ou `--sim`), deixando a máquina limpa.

#### Fluxos na prática
1. **Setup direto pela CLI dentro do container (modo `local`):** `just hermes-subir` → `just hermes-shell` (rodar `sexta-feira plano hermes`, `aplicar hermes`, `verificar hermes`) → `just hermes-chat`.
2. **Instalação guiada pelo próprio Hermes (`local` ou `dia-zero`):** `just hermes-subir` (ou `just hermes-subir dia-zero`) → `just hermes-chat-padrao` → pedir ao Hermes para ler a `SKILL.md` (local em `/workspace/sexta-feira/recursos/skills/instalar-sexta-feira/SKILL.md` ou a URL do GitHub) → depois testar `just hermes-chat` (com `modo=dia-zero` quando aplicável).
3. **Limpeza:** `just hermes-resetar` para reiniciar só o Hermes rapidamente, ou `just hermes-destruir` para remover tudo do Docker.

## 6. O que fica a critério do construtor

- Nomes exatos das variáveis internas e flags auxiliares do `bootstrap-hermes.sh`.
- Detalhes de formatação das receitas no `justfile` (ex.: parâmetro opcional `modo=local` vs receitas separadas).
- Ajustes pontuais nos comandos `hermes config set` / estrutura exata de `config.yaml` caso a inspeção prática da imagem `nousresearch/hermes-agent:latest` revele alguma flag ou ferramenta ausente no container (ex.: instalar `curl` / `tar` em uma camada fina somente se não vierem na imagem oficial — registrando o achado na execução).
- Verificação real subindo o container localmente durante a implementação (confirmando `builder`, `hermes`, `sexta-feira plano/aplicar/verificar` e inicialização do `hermes -p sexta-feira`).
