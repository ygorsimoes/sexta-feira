# Ambiente local em Docker com Hermes Agent

Este laboratório permite subir um container limpo do **Hermes Agent** (`nousresearch/hermes-agent`) pré-configurado com **OpenRouter** para testar a Sexta-feira de ponta a ponta — tanto aplicando o setup diretamente via CLI quanto pedindo ao próprio agente Hermes para instalar a Sexta-feira seguindo [recursos/skills/instalar-sexta-feira/SKILL.md](../recursos/skills/instalar-sexta-feira/SKILL.md).

## Pré-requisitos

- Docker Desktop ou Docker Engine com `docker compose` (v2).
- Uma chave de API do **OpenRouter** (`OPENROUTER_API_KEY`).
- `just` é **opcional**: todos os comandos funcionam diretamente via `./docker/hermes/scripts/lab.sh` (e quem usa `just` pode usar os atalhos `just hermes-*`).

## Início rápido

### 1. Configure o `.env` (só a chave e o modelo)

```bash
cp docker/hermes/.env.example docker/hermes/.env
```

Edite `docker/hermes/.env` (arquivo ignorado pelo Git) e preencha `OPENROUTER_API_KEY=...` (e ajuste `HERMES_MODEL` se quiser trocar o padrão `anthropic/claude-sonnet-4`).

> Se você rodar `subir` antes de copiar o arquivo, o próprio script cria `docker/hermes/.env` a partir do `.env.example` e avisa o que preencher.

### 2. Suba o laboratório

```bash
./docker/hermes/scripts/lab.sh subir
```

No modo padrão (`local`), esse comando:
1. Compila a CLI `sexta-feira` em `--release` para Linux dentro de um container Rust dedicado (`builder`), reaproveitando caches em volumes nomeados para compilações incrementais rápidas.
2. Sobe o container do Hermes Agent (`HERMES_HOME=/opt/data`) com o binário `sexta-feira` no `PATH` e o repositório montado em somente leitura em `/workspace/sexta-feira`.
3. Configura o provedor **OpenRouter** no perfil padrão e no perfil isolado `sexta-feira`, sem tocar em `SOUL.md` nem em `memories/USER.md`.

### 3. Escolha como testar

#### Fluxo A — Pedir para o próprio Hermes instalar a Sexta-feira

1. Abra o chat no perfil padrão:
   ```bash
   ./docker/hermes/scripts/lab.sh chat-padrao
   ```
2. Peça ao agente para ler e seguir a skill local (ou a URL do GitHub):
   ```text
   Leia /workspace/sexta-feira/recursos/skills/instalar-sexta-feira/SKILL.md e instale a Sexta-feira neste ambiente.
   ```
3. Depois que ele concluir, abra o chat já como **Sexta-feira**:
   ```bash
   ./docker/hermes/scripts/lab.sh chat
   ```

#### Fluxo B — Aplicar manualmente pela CLI dentro do container

```bash
./docker/hermes/scripts/lab.sh shell
# Dentro do container:
sexta-feira diagnostico
sexta-feira iniciar
sexta-feira plano hermes
sexta-feira aplicar hermes
sexta-feira verificar hermes
exit

# Conversar com a Sexta-feira:
./docker/hermes/scripts/lab.sh chat
```

#### Fluxo C — Modo `dia-zero` (VPS limpa sem CLI pré-instalada)

Para testar o Hermes baixando a última Release publicada do GitHub (`SHA256SUMS.txt`) ou usando fallback assistido sem o binário local nem o repositório montado:

```bash
./docker/hermes/scripts/lab.sh subir dia-zero
./docker/hermes/scripts/lab.sh chat-padrao dia-zero
```

Depois de instalado, converse no perfil `sexta-feira` do modo `dia-zero`:

```bash
./docker/hermes/scripts/lab.sh chat sexta-feira dia-zero
```

## Gerenciamento e limpeza

| Ação | Script direto | Com `just` | O que faz |
|---|---|---|---|
| Subir (`local` ou `dia-zero`) | `./docker/hermes/scripts/lab.sh subir [modo]` | `just hermes-subir [modo]` | Compila (em `local`), sobe e configura OpenRouter |
| Reconfigurar modelo | `./docker/hermes/scripts/lab.sh subir local --reconfigurar` | `just hermes-subir local --reconfigurar` | Atualiza `model.*` e `.env` após editar `.env` |
| Chat como Sexta-feira | `./docker/hermes/scripts/lab.sh chat [perfil] [modo]` | `just hermes-chat [perfil] [modo]` | Abre `hermes -p <perfil> chat` |
| Chat no perfil padrão | `./docker/hermes/scripts/lab.sh chat-padrao [modo]` | `just hermes-chat-padrao [modo]` | Abre `hermes chat` no perfil padrão |
| Shell no container | `./docker/hermes/scripts/lab.sh shell [modo]` | `just hermes-shell [modo]` | Abre `bash` como usuário `hermes` |
| Estado e verificação | `./docker/hermes/scripts/lab.sh status` | `just hermes-status` | Lista containers, volumes e roda `sexta-feira verificar hermes` |
| Logs | `./docker/hermes/scripts/lab.sh logs [modo]` | `just hermes-logs [modo]` | Acompanha logs do container |
| Parar containers | `./docker/hermes/scripts/lab.sh parar` | `just hermes-parar` | Para os containers preservando tudo |
| Resetar dados do Hermes | `./docker/hermes/scripts/lab.sh resetar [local\|dia-zero\|todos]` | `just hermes-resetar [alvo]` | Apaga só os dados do Hermes, mantendo cache Rust |
| Deletar tudo | `./docker/hermes/scripts/lab.sh destruir [--sim]` | `just hermes-destruir [--sim]` | Remove containers, rede e **todos** os volumes (inclusive caches) |

## Solução de problemas

- **Alterei `HERMES_MODEL` ou `OPENROUTER_API_KEY` em `docker/hermes/.env`:** rode `./docker/hermes/scripts/lab.sh subir local --reconfigurar` (o bootstrap normal preserva `config.yaml` existentes para não sobrescrever ajustes manuais feitos em testes).
- **Quero testar a skill do zero novamente sem esperar recompilar o Rust:** use `./docker/hermes/scripts/lab.sh resetar` e depois `subir`.
- **Erro ao conectar no daemon Docker:** verifique se o Docker Desktop (ou Docker Engine) está em execução.
