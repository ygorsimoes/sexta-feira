#!/usr/bin/env bash
# Gerenciador único do laboratório local Docker (Hermes Agent + Sexta-feira).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCKER_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
COMPOSE_FILE="$DOCKER_DIR/compose.yaml"
ENV_FILE="$DOCKER_DIR/.env"
ENV_EXAMPLE="$DOCKER_DIR/.env.example"
PROJETO="sexta-feira-hermes"

erro() {
    echo "Erro: $*" >&2
    exit 1
}

ajuda() {
    cat <<'EOF'
Uso: ./docker/hermes/scripts/lab.sh <comando> [opções]

Comandos principais:
  subir [local|dia-zero] [--reconfigurar]
      Compila a CLI (no modo local), sobe o Hermes e configura o OpenRouter.
  chat [perfil] [local|dia-zero]
      Abre `hermes chat` (padrão: perfil `sexta-feira`, modo `local`).
  chat-padrao [local|dia-zero]
      Abre `hermes chat` no perfil padrão (para testar instalação via SKILL.md).
  shell [local|dia-zero]
      Abre shell interativo dentro do container Hermes.
  status
      Mostra containers, volumes e verificação da Sexta-feira (quando no modo local).
  logs [local|dia-zero]
      Acompanha os logs do container Hermes.
  parar
      Para os containers mantendo dados e cache de compilação.
  resetar [local|dia-zero|todos]
      Remove os containers e os dados do Hermes, preservando o cache Rust.
  destruir [--sim]
      Remove tudo do laboratório no Docker (containers, rede e todos os volumes).
  ajuda
      Exibe esta ajuda.
EOF
}

validar_modo() {
    case "${1:-local}" in
        local|dia-zero) echo "${1:-local}" ;;
        *) erro "modo inválido '$1'. Use 'local' ou 'dia-zero'." ;;
    esac
}

servico_do_modo() {
    case "$1" in
        local) echo "hermes" ;;
        dia-zero) echo "hermes-dia-zero" ;;
    esac
}

arquivo_env_ativo() {
    if [ -f "$ENV_FILE" ]; then
        echo "$ENV_FILE"
    else
        echo "$ENV_EXAMPLE"
    fi
}

ler_chave_openrouter() {
    awk -F= '/^[[:space:]]*OPENROUTER_API_KEY[[:space:]]*=/{
        sub(/^[^=]*=/, "", $0)
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", $0)
        gsub(/^["'\'']|["'\'']$/, "", $0)
        val=$0
    } END{ print val }' "$ENV_FILE"
}

garantir_env_para_subir() {
    if [ ! -f "$ENV_FILE" ]; then
        cp "$ENV_EXAMPLE" "$ENV_FILE"
        chmod 0600 "$ENV_FILE" || true
        echo "Criei $ENV_FILE a partir de .env.example."
        echo "Preencha sua OPENROUTER_API_KEY nesse arquivo e rode o comando novamente."
        exit 1
    fi

    local chave
    chave="$(ler_chave_openrouter)"
    if [ -z "$chave" ]; then
        erro "OPENROUTER_API_KEY está vazia em $ENV_FILE. Preencha sua chave antes de subir."
    fi
}

compose() {
    local env_ativo
    env_ativo="$(arquivo_env_ativo)"
    docker compose -f "$COMPOSE_FILE" --env-file "$env_ativo" "$@"
}

servico_rodando() {
    local modo="$1"
    local servico="$2"
    [ -n "$(compose --profile "$modo" ps -q --status running "$servico" 2>/dev/null || true)" ]
}

aguardar_container() {
    local modo="$1"
    local servico="$2"
    local tentativa=0
    while [ "$tentativa" -lt 30 ]; do
        if compose --profile "$modo" exec -T -u hermes "$servico" test -w /opt/data >/dev/null 2>&1; then
            return 0
        fi
        tentativa=$((tentativa + 1))
        sleep 1
    done
    erro "o serviço '$servico' não ficou pronto em /opt/data dentro de 30s."
}

cmd_subir() {
    local modo="local"
    local reconfigurar=""

    for arg in "$@"; do
        case "$arg" in
            local|dia-zero) modo="$arg" ;;
            --reconfigurar) reconfigurar="--reconfigurar" ;;
            *) erro "opção inválida em 'subir': '$arg'." ;;
        esac
    done

    garantir_env_para_subir
    local servico
    servico="$(servico_do_modo "$modo")"

    if [ "$modo" = "local" ]; then
        echo "[lab] Compilando CLI sexta-feira para Linux (cache ativo)..."
        compose --profile local run --rm builder
    fi

    echo "[lab] Subindo serviço '$servico' (modo $modo)..."
    compose --profile "$modo" up -d --no-deps --remove-orphans "$servico"

    aguardar_container "$modo" "$servico"

    echo "[lab] Aplicando configuração OpenRouter..."
    if [ -n "$reconfigurar" ]; then
        compose --profile "$modo" exec -T -u hermes -e HOME=/opt/data -e HERMES_HOME=/opt/data \
            "$servico" /opt/sexta-feira/scripts/bootstrap-hermes.sh --reconfigurar
    else
        compose --profile "$modo" exec -T -u hermes -e HOME=/opt/data -e HERMES_HOME=/opt/data \
            "$servico" /opt/sexta-feira/scripts/bootstrap-hermes.sh
    fi

    cat <<EOF

Ambiente '$modo' pronto! Próximos comandos úteis:
  ./docker/hermes/scripts/lab.sh chat-padrao $modo   # conversar no perfil padrão (instalar via SKILL.md)
  ./docker/hermes/scripts/lab.sh shell $modo         # abrir shell no container
  ./docker/hermes/scripts/lab.sh chat sexta-feira $modo  # conversar como Sexta-feira (após aplicar)
EOF
}

exec_interativo() {
    local modo="$1"
    local servico="$2"
    shift 2
    if [ -t 0 ] && [ -t 1 ]; then
        compose --profile "$modo" exec -u hermes -e HOME=/opt/data -e HERMES_HOME=/opt/data "$servico" "$@"
    else
        compose --profile "$modo" exec -T -u hermes -e HOME=/opt/data -e HERMES_HOME=/opt/data "$servico" "$@"
    fi
}

cmd_chat() {
    local perfil="sexta-feira"
    local modo="local"

    if [ $# -ge 1 ]; then
        if [ "$1" = "local" ] || [ "$1" = "dia-zero" ]; then
            modo="$1"
        else
            perfil="$1"
            if [ $# -ge 2 ]; then
                modo="$(validar_modo "$2")"
            fi
        fi
    fi

    local servico
    servico="$(servico_do_modo "$modo")"
    servico_rodando "$modo" "$servico" || erro "o serviço '$servico' ($modo) não está rodando. Execute 'subir $modo' primeiro."

    if [ "$perfil" = "default" ] || [ "$perfil" = "padrao" ]; then
        exec_interativo "$modo" "$servico" hermes chat
    else
        exec_interativo "$modo" "$servico" hermes -p "$perfil" chat
    fi
}

cmd_chat_padrao() {
    local modo
    modo="$(validar_modo "${1:-local}")"
    cmd_chat "default" "$modo"
}

cmd_shell() {
    local modo
    modo="$(validar_modo "${1:-local}")"
    local servico
    servico="$(servico_do_modo "$modo")"
    servico_rodando "$modo" "$servico" || erro "o serviço '$servico' ($modo) não está rodando. Execute 'subir $modo' primeiro."
    exec_interativo "$modo" "$servico" /bin/bash
}

cmd_status() {
    echo "=== Containers ($PROJETO) ==="
    compose --profile local --profile dia-zero ps -a
    echo ""
    echo "=== Volumes ($PROJETO) ==="
    docker volume ls --filter "name=${PROJETO}_"
    if servico_rodando "local" "hermes"; then
        echo ""
        echo "=== Verificação Sexta-feira (modo local) ==="
        compose --profile local exec -T -u hermes -e HOME=/opt/data -e HERMES_HOME=/opt/data \
            hermes sexta-feira verificar hermes || true
    fi
}

cmd_logs() {
    local modo
    modo="$(validar_modo "${1:-local}")"
    local servico
    servico="$(servico_do_modo "$modo")"
    compose --profile "$modo" logs -f --tail=100 "$servico"
}

cmd_parar() {
    compose --profile local --profile dia-zero stop
}

cmd_resetar() {
    local alvo="${1:-todos}"
    case "$alvo" in
        local|dia-zero|todos) ;;
        *) erro "alvo inválido em 'resetar': '$alvo'. Use 'local', 'dia-zero' ou 'todos'." ;;
    esac

    echo "[lab] Parando containers..."
    compose --profile local --profile dia-zero down --remove-orphans

    if [ "$alvo" = "local" ] || [ "$alvo" = "todos" ]; then
        docker volume rm -f "${PROJETO}_hermes_dados" >/dev/null 2>&1 || true
        echo "[lab] Volume de dados 'local' removido."
    fi
    if [ "$alvo" = "dia-zero" ] || [ "$alvo" = "todos" ]; then
        docker volume rm -f "${PROJETO}_hermes_dados_dia_zero" >/dev/null 2>&1 || true
        echo "[lab] Volume de dados 'dia-zero' removido."
    fi
    echo "[lab] Cache de compilação Rust preservado."
}

cmd_destruir() {
    local confirmar=1
    for arg in "$@"; do
        case "$arg" in
            --sim|-y) confirmar=0 ;;
            *) erro "opção inválida em 'destruir': '$arg'. Use '--sim'." ;;
        esac
    done

    if [ "$confirmar" -eq 1 ]; then
        printf "Isso vai apagar todos os containers, redes e volumes (incluindo cache Rust) de '%s'. Continuar? [s/N] " "$PROJETO"
        read -r resp
        case "$resp" in
            s|S|sim|SIM) ;;
            *) echo "Cancelado."; exit 0 ;;
        esac
    fi

    compose --profile local --profile dia-zero down -v --remove-orphans
    echo "[lab] Ambiente Docker removido por completo."
}

comando="${1:-ajuda}"
shift || true

case "$comando" in
    subir) cmd_subir "$@" ;;
    chat) cmd_chat "$@" ;;
    chat-padrao) cmd_chat_padrao "$@" ;;
    shell) cmd_shell "$@" ;;
    status) cmd_status "$@" ;;
    logs) cmd_logs "$@" ;;
    parar) cmd_parar "$@" ;;
    resetar) cmd_resetar "$@" ;;
    destruir) cmd_destruir "$@" ;;
    ajuda|-h|--help) ajuda ;;
    *) erro "comando desconhecido '$comando'. Rode '$0 ajuda'." ;;
esac
