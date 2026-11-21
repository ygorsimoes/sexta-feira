# Design: Ergonomia de setup manual, diagnóstico e modo `--verbose` da CLI

Data: 2026-11-21
Status: rascunho para revisão

## 1. Intenção e porquê

Quem instala a Sexta-feira manualmente hoje conta apenas com os quatro subcomandos de execução (`harnesses`, `plano`, `aplicar`, `verificar`) e com a variável `SEXTA_FEIRA_LOG` para depuração. Falta um caminho guiado para criar a personalização inicial (`usuario.toml`), faltam orientações claras de caminhos e próximos passos na saída padrão e nos erros, e falta um modo `--verbose` rico que exponha o contexto completo de resolução (variáveis, camadas, destino, estratégias e decisões) tanto para humanos quanto para agentes instaladores.

Esta entrega torna a CLI `sexta-feira` intuitiva, autoexplicativa, observável e robusta de ponta a ponta, mantendo o núcleo `sexta-feira-core` 100% agnóstico de harness e preservando a compatibilidade com `--json`.

## 2. Objetivos, não-objetivos e anti-objetivos

### Objetivos
- Adicionar subcomando `iniciar` para setup manual guiado (interativo em TTY ou scriptável via flags + `--nao-interativo`), gerando e validando `usuario.toml` no diretório de configuração e opcionalmente pré-visualizando/aplicando no harness escolhido.
- Adicionar subcomando `diagnostico` (texto humano e `--json`) consolidando versão, ambiente, camadas de configuração, definição efetiva, harnesses detectados no disco, estado de sincronização e próximo passo sugerido.
- Adicionar flag global acumulável `-v` / `--verbose` (`-v` contexto operacional estruturado + `info`; `-vv` diagnóstico completo de ambiente/artefatos + `debug`; `-vvv` `trace`), sempre em `stderr` para nunca poluir `stdout` (especialmente sob `--json`).
- Melhorar a ergonomia dos comandos existentes (`--help` com exemplos em `pt-BR`, cabeçalhos e próximos passos em `plano`/`aplicar`/`verificar`) e anexar dicas acionáveis às principais mensagens de erro (conflitos, harness `em breve`, arquivo ausente, chave desconhecida ou campo vazio em TOML).
- Expor helpers agnósticos mínimos em `sexta-feira-core` (inspeção de camadas carregadas em `FontesDaDefinicao`) e metadados diagnósticos em `sexta-feira-adapters` (sem vazar conhecimento de harness para o `core`).
- Atualizar [SKILL.md](../../../recursos/skills/instalar-sexta-feira/SKILL.md) e [README.md](../../../README.md) para refletir `iniciar`, `diagnostico` e `-v`/`--verbose`.
- Cobrir todos os novos comportamentos e casos de borda com testes unitários e de ponta a ponta (`assert_cmd` / `insta`).

### Não-objetivos
- Adicionar dependências fora de [docs/stack.md](../../stack.md) (tudo usa as crates já aprovadas: `clap`, `serde`, `serde_json`, `toml`, `figment`, `thiserror`, `anyhow`, `tracing`, `tracing-subscriber`).
- Implementar novos adapters (`openclaw`, `claude-code`, `claude-desktop`, `opencode`) — continuam com status `em breve`, agora com detecção indicativa de diretórios no `diagnostico` e dicas claras de fallback.
- Alterar o comportamento de substituição de arrays do `figment` ou o formato de `.sexta-feira-estado.toml`.

### Anti-objetivos
- Poluir `stdout` quando `--json` estiver ativo (toda saída de `-v`/`--verbose` e `tracing` vai exclusivamente para `stderr`).
- Travar esperando `stdin` quando o comando `iniciar` for executado fora de um terminal interativo (sem TTY).
- Sobrescrever `usuario.toml` existente em `iniciar` sem confirmação interativa ou flag `--sobrescrever` (e sem gerar backup `.bak`).
- Quebrar contratos existentes de `sexta-feira harnesses` ou das flags atuais (`--destino`, `--hermes-home`, `--perfil`, `--persona`, `--usuario`, `--forcar`, `--json`).

## 3. Restrições

- Rust edição 2024, `rust-version = "1.100"`, `unsafe_code = "forbid"`, `missing_docs = "warn"`, `clippy::pedantic`, `unwrap_used` e `expect_used` proibidos fora de testes.
- Nada específico de harness em `sexta-feira-core`.
- Documentação, identificadores de domínio, flags e mensagens ao usuário em `pt-BR`.
- Manter compatibilidade com todos os testes atuais em `crates/sexta-feira-cli/tests/cli.rs` (e atualizar/adicionar snapshots apenas quando intencional).

## 4. Abordagens consideradas

1. **Apenas enriquecer os 4 subcomandos atuais + `-v`:** manteria superfície mínima, mas obrigaria quem instala manualmente a escrever TOML à mão e forçaria o agente a combinar vários comandos para descobrir o ambiente.
2. **Subcomandos dedicados `iniciar` + `diagnostico`, modo global `-v`/`-vv` e melhoria dos 4 comandos existentes (Escolhida):** resolve o onboarding manual sem editar TOML do zero, oferece inspeção única em `--json`/texto para humanos e agentes, e dá observabilidade detalhada via `-v`/`--verbose` sem quebrar scripts existentes.

## 5. Desenho da solução

### 5.1 Inspeção agnóstica de camadas (`crates/sexta-feira-core/src/definicao.rs`)
- Adicionar estruturas públicas agnósticas para descrever camadas consultadas:
  - `OrigemDaCamada` (`Embutida`, `DiretorioConfig`, `ArquivoExplicito`)
  - `StatusDaCamada` (`Aplicada`, `Ausente`)
  - `CamadaDeDefinicao { secao: &'static str, origem: OrigemDaCamada, caminho: Option<PathBuf>, status: StatusDaCamada }`
- Método `FontesDaDefinicao::inspecionar_camadas(&self) -> Vec<CamadaDeDefinicao>` retornando na ordem exata de precedência (`persona` embutida → `diretorio_config/persona.toml` → `persona_explicita`, e análogo para `usuario`), junto de `DefinicaoDoMordomo::carregar_com_camadas(...)` (ou mantendo `carregar` intacto e reutilizando a mesma lógica de resolução).
- Expor no `sexta-feira-core/src/lib.rs` apenas se necessário pela CLI; mantém `core` sem nenhum conceito de harness.

### 5.2 Metadados diagnósticos de harnesses (`crates/sexta-feira-adapters`)
- Em `catalogo.rs` / `hermes.rs` (sem tocar `sexta-feira-core`):
  - Expor função `inspecionar_ambiente_harnesses(...)` (ou método em `Harness` / struct `DiagnosticoHarness`) que, para cada harness do catálogo:
    - Para `hermes`: resolve `ConfiguracaoHermes` (aceitando destino e perfil explícitos opcionais), verifica se `hermes_home` existe em disco, se `diretorio_do_perfil` existe e se `.sexta-feira-estado.toml` existe; quando aplicável, retorna caminhos resolvidos e origem da resolução (`--destino` vs `HERMES_HOME` vs `~/.hermes`).
    - Para harnesses `em breve` (`openclaw`, `claude-code`, `claude-desktop`, `opencode`): informa os caminhos convencionais típicos em `$HOME` (ex.: `~/.openclaw`, `~/.claude`, `~/.config/opencode`) apenas de forma indicativa (`indicio_no_disco: bool`, `caminhos_observados: Vec<PathBuf>`) sem implementar adapter completo.

### 5.3 Modo `--verbose` global (`crates/sexta-feira-cli/src/main.rs`)
- Flag global na `Cli`:
  ```rust
  #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true)]
  verbose: u8
  ```
- Inicialização do `tracing_subscriber`:
  - Se `SEXTA_FEIRA_LOG` estiver definida e não vazia, respeita `SEXTA_FEIRA_LOG` (compatibilidade total).
  - Caso contrário: `verbose == 0` → `warn`; `verbose == 1` → `info`; `verbose == 2` → `debug`; `verbose >= 3` → `trace`.
  - Sempre direcionado para `stderr`.
- Relatório verbose estruturado em `stderr` (usando um módulo/helper dedicado em `sexta-feira-cli`, ex.: `verbose.rs` ou módulo interno):
  - **`-v` (`verbose >= 1`):** imprime bloco `[verbose]` legível contendo:
    - Subcomando e alvo;
    - Camadas de configuração consultadas e status (`aplicada` vs `ausente`);
    - Resumo da definição carregada (`persona.nome`, `persona.arquetipo`, `usuario.tratamento`, `usuario.idioma`, quantidades de `tom`/`diretrizes`/`limites`/`preferencias`);
    - No adapter alvo (`plano`/`aplicar`/`verificar`): diretório base resolvido e origem, diretório do perfil, caminho do `.sexta-feira-estado.toml` (existente ou não);
    - Detalhe por arquivo (`caminho`, `estrategia`, `permissao`, `operacao`/`sincronizado`, motivo/resumo do diff).
  - **`-vv` (`verbose >= 2`):** acrescenta seção de ambiente (`SO/arch`, `HOME`, `XDG_CONFIG_HOME`, `HERMES_HOME`, `SEXTA_FEIRA_LOG`, diretório atual) e métricas dos artefatos planejados (linhas e bytes de cada arquivo, ex.: `SOUL.md: X/80 linhas`).
  - Sob `--json`, o `stdout` permanece exclusivamente com JSON válido; o bloco verbose vai apenas para `stderr`.

### 5.4 Subcomando `sexta-feira iniciar`
- Propósito: facilitar a configuração inicial manual sem editar TOML à mão, mantendo opção 100% não-interativa para scripts e testes.
- Opções:
  - `--tratamento <texto>`: sobrescreve `tratamento` (ex.: `"comandante"`).
  - `--idioma <texto>`: sobrescreve `idioma` (padrão herdado: `"pt-BR"`).
  - `--preferencia <texto>` (repetível): lista explícita de preferências; quando informada, substitui a lista de preferências no arquivo gerado.
  - `--config-dir <dir>`: sobrescreve o diretório onde `usuario.toml` será gravado (útil para testes/isolamento; padrão `${XDG_CONFIG_HOME:-~/.config}/sexta-feira`).
  - `--harness <harness>`: opcional; se informado, após resolver/gravar `usuario.toml`, executa `plano` e, se `--aplicar` estiver ativo, aplica o setup (respeitando `--destino`, `--perfil`, `--forcar`).
  - `--destino <dir>` / `--perfil <nome>` / `--aplicar` / `--forcar`: controles opcionais para já concluir o setup no harness.
  - `--sobrescrever`: autoriza atualizar `usuario.toml` já existente (criando `usuario.toml.bak` numerado antes de substituir caso o conteúdo efetivamente mude).
  - `--nao-interativo`: executa sem prompts no terminal.
- Regra de mesclagem, existência prévia e idempotência:
  - **Quando `usuario.toml` ainda não existe:** gera com os campos informados (ou `tratamento = "senhor"` por padrão quando nada foi passado) e valida antes de gravar, reportando estado `"criado"`.
  - **Quando `usuario.toml` já existe e é TOML válido:**
    - Faz **mesclagem (patch)** sobre os campos já presentes no arquivo existente: campos omitidos nesta execução (`--tratamento`, `--idioma`, `--preferencia`) preservam o que já estava salvo em `usuario.toml`.
    - Se o conteúdo resultante for idêntico ao que já está em disco, **não reescreve o arquivo nem cria `.bak`**, reportando `"mantido"` (exit `0`) e prosseguindo normalmente para `--harness` se informado.
    - Se houver alterações efetivas sem `--sobrescrever`: em `--nao-interativo`, falha com erro claro pedindo `--sobrescrever`; no modo interativo, pergunta se deseja sobrescrever — se o usuário recusar, mantém o arquivo existente (`"mantido"`, exit `0`) e segue oferecendo o próximo passo no harness.
    - Com `--sobrescrever` (ou confirmação interativa) e conteúdo alterado: cria backup `.bak` numerado e grava o novo conteúdo, reportando `"atualizado"`.
  - **Quando `usuario.toml` já existe, mas está corrompido ou com chaves inválidas:** sem `--sobrescrever` (ou confirmação interativa), falha com erro claro explicando o problema no arquivo atual e sugerindo `--sobrescrever`; com `--sobrescrever`/confirmação, cria backup `.bak` do arquivo antigo e grava uma nova configuração limpa válida (`"atualizado"`).
- Comportamento de TTY e segurança:
  - Se `--nao-interativo` não foi passado e `stdin` **não** é um terminal interativo (`!std::io::stdin().is_terminal()`): falha imediatamente com erro claro explicando que o modo interativo requer TTY e sugerindo `--nao-interativo` (com exemplos de flags), nunca travando à espera de entrada.
  - No modo interativo (TTY sem `--nao-interativo`): mostra o diretório de configuração, pergunta tratamento (Enter mantém atual ou padrão), pergunta se deseja definir preferências customizadas (alertando que substitui a lista padrão, ou Enter para manter atual/herdar padrão), pergunta se já deseja aplicar em um harness disponível (ex.: `hermes`) e confirma antes se precisar sobrescrever arquivo existente ou usar `--forcar`.
  - Antes de gravar `usuario.toml` no disco, valida o conteúdo em memória usando `DefinicaoDoMordomo::carregar` com camada simulada (garantindo que nunca grava TOML inválido ou campo vazio).
  - Suporta `--json` retornando caminho do `usuario.toml`, estado (`"criado"`, `"atualizado"` ou `"mantido"`), caminho do backup se houver, e resultado opcional do plano/aplicação no harness.

### 5.5 Subcomando `sexta-feira diagnostico`
- Assinatura: `sexta-feira diagnostico [harness] [--destino <dir>] [--perfil <nome>] [--persona <arq>] [--usuario <arq>]`.
- Coleta e apresenta (em texto estruturado ou `--json`):
  1. **CLI e ambiente:** versão da CLI, SO/arquitetura, variáveis (`HOME`, `XDG_CONFIG_HOME`, `HERMES_HOME`, `SEXTA_FEIRA_LOG`).
  2. **Configuração:** diretório base resolvido, status de `persona.toml` e `usuario.toml` (presente/ausente), camadas explícitas, resultado da validação da definição (se válida: resumo; se inválida: erro detalhado sem abortar o diagnóstico inteiro!).
  3. **Harnesses:** todos (ou apenas o `[harness]` filtrado), mostrando disponibilidade na CLI, caminhos observados/resolvidos, existência de estado `.sexta-feira-estado.toml` e estado de sincronização (para harnesses disponíveis cujo destino possa ser resolvido).
  4. **Próximo passo recomendado:** sugestão objetiva em texto e campo `proximo_passo` no JSON.
- Exit code: `0` quando o diagnóstico foi executado (mesmo que o setup ainda não esteja aplicado ou haja config inválida reportada no relatório, para servir como ferramenta de inspeção confiável; retorna erro apenas se um argumento explícito como `[harness]` desconhecido for passado via `clap`).

### 5.6 Ergonomia dos comandos existentes, `--help` e erros acionáveis
- `--help`: adicionar `after_help` com exemplos curtos em `pt-BR` na raiz e nos subcomandos principais.
- `harnesses`: manter a saída em texto compatível com o snapshot atual (e `json` estruturado).
- `plano`: exibir o diretório alvo resolvido (no Hermes: caminho do perfil) antes da listagem e uma dica objetiva ao final (ex.: comando exato para `aplicar`, ou instrução clara quando houver conflitos).
- `aplicar`: exibir o diretório do perfil aplicado e bloco curto "Próximos passos" (ex.: `sexta-feira verificar ...` e caminho de `~/.config/sexta-feira/usuario.toml`).
- `verificar`: além dos itens `[OK]` / `[PENDENTE]`, resumir quantos itens estão sincronizados/pendentes e manter a dica em `stderr` quando falhar.
- Erros acionáveis: envolver falhas comuns na CLI com contexto + `"Dica: ..."` em `stderr`:
  - Harness `em breve`: listar o comando `sexta-feira harnesses`, avisar que ainda não há adapter oficial e mencionar a skill de instalação assistida (`recursos/skills/instalar-sexta-feira/SKILL.md`).
  - Conflito em `aplicar`: orientar rodar `sexta-feira plano ...` para inspecionar o diff ou `--forcar` para substituir com backup `.bak`.
  - Erro de TOML/chave desconhecida/campo vazio: lembrar as chaves aceitas e sugerir `sexta-feira diagnostico` para inspecionar as camadas.
  - Arquivo `--persona`/`--usuario` ausente: sugerir conferir o caminho ou usar `sexta-feira iniciar`.

### 5.7 Atualização de `SKILL.md` e `README.md`
- Atualizar [SKILL.md](../../../recursos/skills/instalar-sexta-feira/SKILL.md) para recomendar `sexta-feira --json diagnostico` no Passo 1/3, `sexta-feira iniciar --nao-interativo ...` como alternativa opcional segura ao escrever `usuario.toml`, e `-v` / `-vv` para diagnóstico de falhas.
- Atualizar [README.md](../../../README.md) documentando `iniciar`, `diagnostico` e `-v`/`-vv` na seção de início manual e na tabela de referência da CLI.

## 6. O que fica a critério do construtor

- Organização modular interna dentro de `crates/sexta-feira-cli/src/` (ex.: separar `verbose.rs`, `iniciar.rs`, `diagnostico.rs` ou manter módulos coesos sem duplicação).
- Formatação textual exata dos blocos humanos de `diagnostico`, `iniciar` e `[verbose]` (desde que clara, consistente em `pt-BR` e coberta por testes).
- Helper interno de backup `.bak` para `usuario.toml` no `iniciar` (ou reutilização da convenção de backup numerado do projeto).
