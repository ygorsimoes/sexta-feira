#!/bin/sh
# Configura de forma não interativa o provedor OpenRouter no perfil padrão e no
# perfil isolado `sexta-feira` sem criar `SOUL.md` nem `memories/USER.md`.
set -eu

RECONFIGURAR=0
for arg in "$@"; do
    case "$arg" in
        --reconfigurar)
            RECONFIGURAR=1
            ;;
        -h|--help)
            echo "Uso: $0 [--reconfigurar]"
            exit 0
            ;;
        *)
            echo "Erro: opção desconhecida '$arg'. Use --reconfigurar." >&2
            exit 2
            ;;
    esac
done

HERMES_HOME="${HERMES_HOME:-/opt/data}"
HERMES_MODEL="${HERMES_MODEL:-anthropic/claude-sonnet-4}"
HERMES_BASE_URL="${HERMES_BASE_URL:-https://openrouter.ai/api/v1}"
PYTHON_BIN="${HERMES_PYTHON:-/opt/hermes/.venv/bin/python3}"

OPENROUTER_API_KEY_LIMPA="$(printf '%s' "${OPENROUTER_API_KEY:-}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
if [ -z "$OPENROUTER_API_KEY_LIMPA" ]; then
    echo "Erro: OPENROUTER_API_KEY não está definida ou está vazia." >&2
    echo "Preencha OPENROUTER_API_KEY no arquivo .env na raiz e tente novamente." >&2
    exit 1
fi
export OPENROUTER_API_KEY="$OPENROUTER_API_KEY_LIMPA"

if [ ! -x "$PYTHON_BIN" ]; then
    PYTHON_BIN="$(command -v python3)"
fi

configurar_perfil() {
    dir="$1"
    rotulo="$2"
    cfg="$dir/config.yaml"
    env_arq="$dir/.env"
    marcador="$dir/.sexta-feira-lab-bootstrap"

    mkdir -p "$dir"

    if [ ! -f "$cfg" ] || [ ! -f "$marcador" ] || [ "$RECONFIGURAR" -eq 1 ]; then
        "$PYTHON_BIN" - "$cfg" "$HERMES_MODEL" "$HERMES_BASE_URL" <<'PY'
import os
import sys
from ruamel.yaml import YAML

cfg_path, model_default, base_url = sys.argv[1:4]
yaml = YAML()
yaml.preserve_quotes = True

data = {}
if os.path.exists(cfg_path):
    with open(cfg_path, "r", encoding="utf-8") as fh:
        loaded = yaml.load(fh)
        if isinstance(loaded, dict):
            data = loaded

model = data.get("model")
if not isinstance(model, dict):
    model = {}
model["provider"] = "openrouter"
model["default"] = model_default
model["base_url"] = base_url
data["model"] = model

tmp_path = f"{cfg_path}.tmp"
with open(tmp_path, "w", encoding="utf-8") as fh:
    yaml.dump(data, fh)
os.replace(tmp_path, cfg_path)
PY
        echo "[bootstrap] config.yaml atualizado para $rotulo ($HERMES_MODEL)."
    else
        echo "[bootstrap] config.yaml de $rotulo já existe; preservado."
    fi

    "$PYTHON_BIN" - "$env_arq" <<'PY'
import os
import sys

env_path = sys.argv[1]
api_key = os.environ["OPENROUTER_API_KEY"]
target_line = f"OPENROUTER_API_KEY={api_key}\n"
lines = []
found = False
changed = not os.path.exists(env_path)

if os.path.exists(env_path):
    with open(env_path, "r", encoding="utf-8") as fh:
        for line in fh:
            if line.startswith("OPENROUTER_API_KEY="):
                if line != target_line:
                    changed = True
                lines.append(target_line)
                found = True
            else:
                lines.append(line)

if not found:
    if lines and not lines[-1].endswith("\n"):
        lines[-1] += "\n"
    lines.append(target_line)
    changed = True

if changed:
    tmp_path = f"{env_path}.tmp"
    fd = os.open(tmp_path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as fh:
        fh.writelines(lines)
    os.chmod(tmp_path, 0o600)
    os.replace(tmp_path, env_path)
PY

    : > "$marcador"
}

configurar_perfil "$HERMES_HOME" "perfil padrão"
configurar_perfil "$HERMES_HOME/profiles/sexta-feira" "perfil sexta-feira"
