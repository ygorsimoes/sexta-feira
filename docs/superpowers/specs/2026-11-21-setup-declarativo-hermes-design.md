# Design: Setup declarativo da Sexta-feira para o Hermes (Recorte 1 — Persona)

Data: 2026-11-21
Status: rascunho para revisão

## 1. Intenção e porquê

A Sexta-feira é um pacote opinativo e agnóstico de harness para configurar um mordomo virtual pessoal sem ter que montar o ambiente do zero em cada máquina ou VPS ([visao.md](../../visao.md)).

Neste primeiro recorte do MVP, o objetivo é tirar o projeto do papel com um fluxo completo de ponta a ponta para o **Hermes Agent**, entregando a **personalidade Sexta-feira (estilo J.A.R.V.I.S., em `pt-BR`, tratando o usuário como "senhor")** por meio de um motor declarativo de setup em Rust (`core` + `adapters` + `cli`).

## 2. Objetivos, não-objetivos e anti-objetivos

### Objetivos
- Mover o catálogo concreto de harnesses (`Harness`) de `sexta-feira-core` para `sexta-feira-adapters`, cumprindo a regra de isolamento do `core` ([AGENTS.md](../../../AGENTS.md)).
- Modelar no `sexta-feira-core` a definição agnóstica do mordomo (identidade/persona e preferências de tratamento do usuário) e o modelo declarativo de plano de setup (`plano`, `aplicar`, `verificar`).
- Versionar a definição padrão da Sexta-feira em `recursos/mordomo/` e embuti-la no binário para que a CLI seja autossuficiente na VPS.
- Implementar o adapter do Hermes emitindo os arquivos nativos de persona dentro de um **perfil isolado** (`$HERMES_HOME/profiles/<perfil>/`, padrão `sexta-feira`): `SOUL.md` e `memories/USER.md`.
- Disponibilizar subcomandos declarativos na CLI `sexta-feira` com testes unitários e de ponta a ponta.

### Não-objetivos (ficam para incrementos futuros)
- Gerenciamento de `config.yaml`, skills (`SKILL.md`), plugins Python (`plugin.yaml` / `__init__.py`), MCPs (`ai-memory`, Composio), Telegram ou `cron`.
- Instalação ou download do próprio Hermes Agent, chamadas de rede ou daemon contínuo.
- Implementação dos adapters de `OpenClaw`, `Claude Code`, `Claude Desktop` e `OpenCode`.

### Anti-objetivos
- Sobrescrever o perfil global padrão (`~/.hermes/SOUL.md` ou `~/.hermes/memories/USER.md`) quando o usuário usar um perfil dedicado.
- Destruir memórias aprendidas dinamicamente pelo Hermes dentro de `memories/USER.md` ao reaplicar o setup.
- Reintroduzir tipos ou caminhos específicos de qualquer harness dentro de `sexta-feira-core`.
- Introduzir bibliotecas fora de [stack.md](../../stack.md) sem aprovação prévia.

## 3. Restrições

- **Linguagem e toolchain**: Rust edição 2024 conforme [Cargo.toml](../../../Cargo.toml) e [rust-toolchain.toml](../../../rust-toolchain.toml).
- **Segurança e lints**: `unsafe` proibido; sem `unwrap`/`expect` fora de testes; `clippy::pedantic` e `missing_docs` como erro no CI.
- **Erros e idioma**: `thiserror` nas bibliotecas, `anyhow` na CLI; documentação e identificadores de domínio em `pt-BR`.
- **Dependências**: usar apenas bibliotecas já em uso ou previstas em [stack.md](../../stack.md) (`serde`, `toml`, `thiserror`, `anyhow`, `clap`, `tracing-subscriber`, além de `assert_cmd` e `insta` nos testes). Nenhuma biblioteca nova fora de [stack.md](../../stack.md) será adicionada neste recorte: diretórios temporários de teste usam `std::env::temp_dir()` com limpeza por RAII, e a detecção de alteração manual usa função interna determinística sem crate externa.
- **Limpeza de metadados**: com `sexta-feira-cli` passando a consumir `sexta-feira-adapters`, remover a exceção temporária de `cargo-machete` em `crates/sexta-feira-cli/Cargo.toml`.
- **Fluxo de entrega**: todo código entra por branch a partir da `main` e PR com CI verde.

## 4. Abordagens consideradas

1. **Perfil isolado do Hermes gerenciado diretamente em Rust (sem modelo declarativo)**: cria `~/.hermes/profiles/sexta-feira/`, mas acopla cada adapter a lógica imperativa ad-hoc sem plano/diff unificado.
2. **Distribuição nativa do Hermes (`distribution.yaml` + Plugin Python)**: delega tudo ao formato do Hermes; reduz código em Rust, mas acopla o projeto ao ecossistema Python do Hermes e dificulta os demais harnesses.
3. **Compilador declarativo de mordomo em Rust → artefatos nativos por harness (Escolhida)**: o `core` valida a definição agnóstica e calcula um plano declarativo (`Criar`, `Atualizar`, `SemAlteracoes`, `Conflito`), enquanto cada adapter traduz essa definição para o formato nativo do harness alvo. No Hermes, o adapter emite o perfil isolado (`SOUL.md` + `memories/USER.md`).

**Por que a Abordagem 3 foi escolhida:** mantém o binário único autossuficiente em Rust, preserva o isolamento estrito entre `core` e `adapters`, garante pré-visualização (`plano`) e idempotência (`aplicar`/`verificar`), e já usa a separação oficial do Hermes entre identidade (`SOUL.md`) e preferências do usuário (`memories/USER.md`) sem afetar o perfil padrão.

## 5. Desenho da solução

### 5.1 Separação entre crates

#### `crates/sexta-feira-adapters` (catálogo e adapters concretos)
- Recebe `Harness` (`Hermes`, `Openclaw`, `ClaudeCode`, `ClaudeDesktop`, `Opencode`) e `HarnessDesconhecido`, hoje em `crates/sexta-feira-core/src/harness.rs`.
- Expõe o adapter do Hermes, que implementa a tradução da definição do mordomo para um conjunto de arquivos declarativos no perfil do Hermes.

#### `crates/sexta-feira-core` (domínio agnóstico e motor declarativo)
Não conhece `Hermes`, `~/.hermes`, `SOUL.md` ou `USER.md`. Organiza-se em três responsabilidades:

1. **Definição do mordomo (`definicao`)**:
   - `DefinicaoDoMordomo`: agrega `Persona` (nome `"Sexta-feira"`, arquétipo/inspiração J.A.R.V.I.S., idioma `"pt-BR"`, tom de voz, postura proativa, regras de conduta e limites) e `PerfilDoUsuario` (forma de tratamento `"senhor"`, preferências iniciais de comunicação).
   - Carregada a partir dos ativos embutidos de `recursos/mordomo/` (ex.: `persona.toml` e `usuario.toml`, validados na construção).
2. **Recursos desejados e plano declarativo (`plano`)**:
   - `EstrategiaDeEscrita`:
     - `ArquivoIntegral`: o arquivo pertence integralmente à Sexta-feira (ex.: `SOUL.md`). Um estado registrado com checksum determinístico interno (ex.: FNV-1a 64 bits + tamanho) permite detectar se o arquivo foi alterado manualmente fora do setup; sem `--forcar`, divergência gera `OperacaoPlanejada::Conflito` em vez de sobrescrever silenciosamente.
     - `BlocoDelimitado { inicio, fim }`: a Sexta-feira gerencia apenas o trecho entre os delimitadores dentro do arquivo (ex.: `<!-- sexta-feira:inicio -->` e `<!-- sexta-feira:fim -->`), preservando o restante do conteúdo antes ou depois do bloco.
   - `ArquivoDesejado`: caminho de destino (`PathBuf`), conteúdo textual esperado e `EstrategiaDeEscrita`.
   - `OperacaoPlanejada`: `Criar`, `Atualizar`, `SemAlteracoes` ou `Conflito` (com resumo do motivo/diff textual).
   - `PlanoDeSetup`: lista ordenada de ações planejadas para um destino, com métodos para:
     - `planejar(desejados)` — lê o disco apenas para comparação e produz o plano sem efeitos colaterais;
     - `aplicar(plano, forcar)` — grava arquivos (criando diretórios pai quando necessário; em caso de `Conflito` sem `forcar`, falha antes de modificar os arquivos em conflito; com `forcar`, gera backup `<arquivo>.bak` antes de atualizar);
     - `verificar(desejados)` — retorna diagnóstico indicando se todos os arquivos existem e estão sincronizados.
3. **Contrato do adaptador (`adaptador`)**:
   - Trait `AdaptadorDeSetup`: dado `&DefinicaoDoMordomo` e a configuração do destino, retorna `Result<Vec<ArquivoDesejado>, ErroDeSetup>`.

### 5.2 Fonte única da persona (`recursos/mordomo/`)

Os arquivos embutidos no binário definem a Sexta-feira com precisão:
- **Identidade (`SOUL.md` gerado, mantido enxuto < 80 linhas para eficiência de cache/tokens no Hermes)**:
  - Nome: **Sexta-feira**.
  - Personalidade inspirada no **J.A.R.V.I.S.** (Homem de Ferro): calma, formal, leal, precisa, proativa, com ironia/humor britânico sutil apenas quando oportuno, sem bajulação nem rodeios.
  - Idioma padrão: `pt-BR`.
  - Conduta operacional: confirmar antes de ações irreversíveis ou externas relevantes, expor incertezas com franqueza e manter respostas claras e objetivas.
- **Perfil do usuário (`memories/USER.md` gerado no bloco delimitado)**:
  - Tratamento explícito: dirigir-se ao usuário sempre como **"senhor"**.
  - Idioma e estilo preferidos: `pt-BR`, comunicação direta e estruturada.

### 5.3 Adapter do Hermes

- **Resolução do diretório base (`$HERMES_HOME`)**:
  1. Flag explícita `--hermes-home <caminho>` na CLI (ou campo equivalente na struct de configuração do adapter);
  2. Variável de ambiente `HERMES_HOME`, se definida e não vazia;
  3. Diretório padrão `~/.hermes` (resolvido a partir de `HOME`).
- **Diretório do perfil**: `<HERMES_HOME>/profiles/<perfil>/`, com valor padrão `<perfil> = "sexta-feira"` (personalizável via `--perfil <nome>`).
- **Artefatos emitidos**:
  1. `<perfil_dir>/SOUL.md` — estratégia `ArquivoIntegral`.
     - *Decisão do design*: usar `<perfil_dir>/.sexta-feira-estado.toml` para registrar o checksum aplicado dos arquivos gerenciados integralmente, mantendo `SOUL.md` limpo sem comentários artificiais no prompt do LLM.
     - Se `SOUL.md` já existir antes da primeira execução do `sexta-feira` (sem `.sexta-feira-estado.toml`), é tratado como `Conflito` (exige `--forcar`, que cria `SOUL.md.bak` antes de gravar).
  2. `<perfil_dir>/memories/USER.md` — estratégia `BlocoDelimitado` com `<!-- sexta-feira:inicio -->` e `<!-- sexta-feira:fim -->`:
     - Se `USER.md` não existir, cria o arquivo contendo o bloco gerenciado.
     - Se já existir com os marcadores, substitui apenas o trecho interno se houver mudança; caso contrário, `SemAlteracoes`.
     - Se já existir sem marcadores (ex.: memórias prévias do usuário/Hermes), insere o bloco gerenciado no topo preservando integralmente o texto existente abaixo (ou marca como `Atualizar` preservando o conteúdo).

### 5.4 CLI (`crates/sexta-feira-cli`)

Subcomandos previstos:
- `sexta-feira harnesses` — lista os harnesses previstos (via `sexta-feira-adapters`).
- `sexta-feira plano <harness> [--hermes-home <dir>] [--perfil <nome>]` — imprime o plano declarativo (`CRIAR`, `ATUALIZAR`, `OK`, `CONFLITO`) e o diff legível sem escrever em disco. Retorna código 0 (ou 2 se houver conflito? Decisão: 0 para consulta informativa, listando claramente `[CONFLITO]`).
- `sexta-feira aplicar <harness> [--hermes-home <dir>] [--perfil <nome>] [--forcar]` — executa o plano; se houver conflito sem `--forcar`, falha com código não-zero antes de gravar qualquer arquivo.
- `sexta-feira verificar <harness> [--hermes-home <dir>] [--perfil <nome>]` — valida se os arquivos do setup existem e batem com o esperado; sai com 0 quando sincronizado e não-zero quando pendente/divergente.

Para harnesses ainda não implementados (`openclaw`, `claude-code`, `claude-desktop`, `opencode`), os comandos `plano`, `aplicar` e `verificar` retornam erro explícito informando que o adapter desse harness ainda não está implementado.

## 6. O que fica a critério do construtor

- Organização interna de submódulos dentro de `sexta-feira-core`, `sexta-feira-adapters` e `sexta-feira-cli`.
- Formato exato da saída textual de `plano`, `aplicar` e `verificar` (desde que determinístico para testes de snapshot).
- Implementação interna da função determinística de checksum para o arquivo de estado (sem adicionar dependências externas fora de [stack.md](../../stack.md)).
- Detalhes de redação do texto em `pt-BR` da persona e das mensagens de erro, mantendo todas as regras desta especificação.
