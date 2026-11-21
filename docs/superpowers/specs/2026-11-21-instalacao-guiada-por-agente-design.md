# Design: Instalação guiada pelo próprio agente (`SKILL.md`)

Data: 2026-11-21
Status: rascunho para revisão

## 1. Intenção e porquê

Permitir que o usuário instale a Sexta-feira pedindo ao próprio agente de IA (inclusive no dia zero, colando uma instrução/URL no chat), sem ficar preso a scripts rígidos cheios de condicionais para cada sistema operacional, caminho ou harness — mas sem abrir mão do motor declarativo e idempotente da CLI em Rust quando houver suporte.

## 2. Objetivos, não-objetivos e anti-objetivos

### Objetivos
- Criar `recursos/skills/instalar-sexta-feira/SKILL.md` como fonte única de instruções no padrão Agent Skills, utilizável tanto por leitura direta via URL no dia zero quanto como skill instalada no harness.
- Orquestrar o caminho completo da CLI `sexta-feira` (`harnesses --json`, `plano`, `aplicar`, `verificar`, camadas em `~/.config/sexta-feira/*.toml`) quando o harness atual tiver adapter disponível.
- Implementar fallback transparente quando o harness atual ainda não tiver adapter na CLI (ou quando a CLI não puder ser obtida/compilada no ambiente): avisar o usuário que ainda não há suporte nativo na CLI, oferecer a instalação assistida e traduzir `recursos/mordomo/persona.toml` e `recursos/mordomo/usuario.toml` (somados a eventuais overrides locais) para o formato do harness.
- Atualizar o [README.md](../../../README.md) com o fluxo de instalação pelo próprio agente em destaque no **Início rápido**, além de refletir `recursos/skills/` na estrutura documentada no `README.md` e em [docs/visao.md](../../visao.md).

### Não-objetivos
- Alterar código Rust (`sexta-feira-core`, `sexta-feira-adapters`, `sexta-feira-cli`) ou workflows de CI/CD.
- Criar novos adapters compilados (`OpenClaw`, `Claude Code`, `Claude Desktop`, `OpenCode`) nesta entrega.
- Inventar ou fixar flags de CLI de outros harnesses sem inspeção local do ambiente.

### Anti-objetivos
- Sobrescrever arquivos existentes do harness ou do usuário no fallback sem mostrar o destino/mudança e confirmar antes (ou usar bloco delimitado/backup `.bak`).
- Baixar binários de release ignorando verificação de `SHA256SUMS.txt` sem aviso e consentimento explícito.
- Tratar a leitura da `SKILL.md` via URL de instalação apenas como "salvar a skill" em vez de executar a instalação imediatamente.

## 3. Abordagens consideradas

1. **Arquivo de instruções avulso na raiz (`INSTALAR.md`):** URL mais curta e sem ambiguidade sobre execução imediata, mas não segue o formato `SKILL.md` nem permite reaproveitamento direto em catálogos de skills.
2. **`SKILL.md` como fonte única (Escolhida):** `recursos/skills/instalar-sexta-feira/SKILL.md` funciona como instrução direta por URL no dia zero (com diretriz explícita no topo para execução imediata) e já nasce no formato `SKILL.md` previsto em `docs/visao.md`, evitando duplicação.

## 4. Desenho da solução e escolhas propostas

- **Localização da skill (escolha proposta):** `recursos/skills/instalar-sexta-feira/SKILL.md`, mantendo `recursos/` como fonte única dos ativos do mordomo conforme [docs/visao.md](../../visao.md).
- **Fluxo da skill em 6 passos:**
  1. **Ativação imediata:** ao ser lida para instalar a Sexta-feira, executar os passos agora em vez de apenas armazenar a skill.
  2. **Diagnóstico do ambiente:** identificar SO/arquitetura (`uname -s`, `uname -m`), harness em execução, diretórios do harness e ferramentas disponíveis (`curl`, `tar`, `sha256sum`/`shasum`, `git`, `cargo`).
  3. **Obtenção da CLI `sexta-feira`:** usar binário já instalado no `PATH` ou no repositório local; se ausente, baixar da Release mais recente (`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `aarch64-apple-darwin`) conferindo `SHA256SUMS.txt`, ou compilar com `cargo build --release --package sexta-feira-cli`. Se não for possível obter/compilar a CLI, registrar o motivo e seguir para o fallback assistido.
  4. **Suporte do harness e personalização:** rodar `sexta-feira --json harnesses`; perguntar brevemente ao usuário se deseja personalizar `~/.config/sexta-feira/usuario.toml` (ou `$XDG_CONFIG_HOME/sexta-feira/usuario.toml`) respeitando estritamente o schema (`tratamento`, `idioma`, `preferencias`, e opcionalmente `persona.toml` sem chaves desconhecidas).
  5. **Aplicação (dois caminhos):**
     - **Harness suportado (`disponivel: true`, ex.: `hermes`):** confirmar `--destino` e `--perfil` (padrão `sexta-feira`), rodar `sexta-feira --json plano ...`, mostrar resumo e eventuais conflitos (confirmando antes de usar `--forcar`), aplicar com `aplicar` e validar com `verificar` (`exit 0`).
     - **Harness sem adapter (`disponivel: false`, não listado ou CLI indisponível):** avisar explicitamente que ainda não existe adapter oficial na CLI para o harness atual, confirmar se o usuário quer tentar a instalação assistida, ler `recursos/mordomo/persona.toml` e `recursos/mordomo/usuario.toml` (localmente ou via URL raw no GitHub) mesclando com `~/.config/sexta-feira/*.toml`, propor o destino/formato nativo do harness (usando bloco delimitado `<!-- sexta-feira:inicio -->` / `<!-- sexta-feira:fim -->` ou backup `.bak` para arquivos existentes) e aplicar após confirmação.
  6. **Pós-instalação opcional:** se o harness suportar skills, oferecer salvar a própria skill localmente para facilitar futuras atualizações e reconfigurações.
- **Documentação:** atualizar [README.md](../../../README.md) colocando um bloco copiável de **"Instalar com seu próprio agente"** no início do **Início rápido** (apontando para a URL raw de `recursos/skills/instalar-sexta-feira/SKILL.md`), mantendo a instalação manual pela CLI logo abaixo, e atualizar a árvore de diretórios no `README.md` e em [docs/visao.md](../../visao.md).

## 5. O que fica a critério do construtor

- Redação final da `SKILL.md` (concisa, testável mentalmente contra os cenários Hermes + fallback, com links raw corretos para `ygorsimoes/sexta-feira` na branch `main`).
- Organização visual da seção no `README.md` mantendo o estilo atual.
