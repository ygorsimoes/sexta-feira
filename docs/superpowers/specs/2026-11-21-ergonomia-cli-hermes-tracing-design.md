# Design: Ergonomia da CLI, adaptador Hermes e instrumentação com `tracing`

Data: 2026-11-21
Status: rascunho para revisão

## 1. Intenção e porquê

Com a PR #14 mergeada, o núcleo `sexta-feira-core` já suporta personalização em camadas (`FontesDaDefinicao`) e remoção segura de órfãos (`OperacaoPlanejada::Remover`), porém a CLI ainda usa apenas `DefinicaoDoMordomo::padrao()`, expõe `--hermes-home` diretamente no comando compartilhado, não informa quais harnesses estão implementados, não oferece saída estruturada `--json` nem emite eventos para o `tracing-subscriber` já inicializado. Além disso, o adaptador do Hermes repete regras de tratamento e idioma entre `SOUL.md`, os metadados de `USER.md` e `recursos/mordomo/usuario.toml`.

## 2. Objetivos, não-objetivos e anti-objetivos

### Objetivos
- Conectar `FontesDaDefinicao::padrao_do_ambiente()` com flags opcionais `--persona <caminho>` e `--usuario <caminho>` em `plano`, `aplicar` e `verificar`.
- Substituir a flag visível `--hermes-home` por `--destino <caminho>` mantendo `alias = "hermes-home"` no `clap` para retrocompatibilidade.
- Exibir status de disponibilidade (`disponível` vs `em breve`) em `sexta-feira harnesses`.
- Suportar saída estruturada `--json` nos subcomandos (`harnesses`, `plano`, `aplicar`, `verificar`), incluir contagem de remoções no resumo do `plano` e dica acionável quando `verificar` encontrar pendências.
- Eliminar repetição literal de tratamento/idioma em `recursos/mordomo/usuario.toml` e expandir `~/` em `ConfiguracaoHermes::resolver_com_env` quando `HOME` estiver definido.
- Adicionar `tracing` (previsto em [docs/stack.md](../../stack.md)) ao workspace (`core`, `adapters`, `cli`) emitindo logs `debug`/`info` relevantes para `SEXTA_FEIRA_LOG`.

### Não-objetivos
- Implementar novos harnesses (`OpenClaw`, `Claude Code`, `Claude Desktop`, `OpenCode`).
- Adicionar `config.yaml`, skills ou scripts no adaptador do Hermes.
- Modificar workflows do GitHub Actions ou o `README.md` (ficam para as PRs 3 e 4).

### Anti-objetivos
- Quebrar scripts existentes que passem `--hermes-home` (preservado via alias).
- Poluir `stdout` com logs do `tracing` (logs continuam indo para `stderr` conforme `main.rs`, mantendo `--json` limpo em `stdout`).

## 3. Restrições

- Rust edição 2024, `rust-version = "1.100"`, `unsafe_code = "forbid"`, `missing_docs = "warn"`, `clippy::pedantic`, `unwrap_used` e `expect_used` proibidos fora de testes.
- Nada específico de harness entra em `sexta-feira-core`.
- Dependências novas: apenas `tracing` (aprovada em [docs/stack.md](../../stack.md)) e `serde_json` já presente no `[workspace.dependencies]` (para `sexta-feira-cli`).
- Toda documentação, identificadores de domínio e mensagens ao usuário em `pt-BR`.

## 4. Abordagens consideradas

1. **Flag genérica `--destino` com `alias = "hermes-home"` + `--json` global no `Cli` (Escolhida):** mantém um único campo em `OpcoesAlvo`, evita que flags específicas por harness poluam `--help` e garante retrocompatibilidade com `--hermes-home`.
2. **Subcomandos ou flags separadas por harness (`--hermes-home`, `--openclaw-home`, ...):** aumenta a superfície de CLI antes mesmo de os outros adapters existirem e duplica validação.

## 5. Desenho da solução

### 5.1 Catálogo de harnesses (`crates/sexta-feira-adapters/src/catalogo.rs`)

- Adicionar método `#[must_use] pub const fn disponivel(self) -> bool` em `Harness` (`true` apenas para `Harness::Hermes`).
- Adicionar método `#[must_use] pub const fn rotulo_status(self) -> &'static str` retornando `"disponível"` quando `self.disponivel()` e `"em breve"` caso contrário.

### 5.2 Adaptador Hermes e recurso padrão (`hermes.rs` e `recursos/mordomo/usuario.toml`)

- Em `ConfiguracaoHermes::resolver_com_env`:
  - Expandir prefixo `~` ou `~/` usando `env_home` tanto para `hermes_home_explicito` quanto para `env_hermes_home`. Se o caminho começar com `~` ou `~/` e `env_home` estiver ausente ou vazio, retornar `Err(ErroDeHermes::DiretorioBaseIndisponivel)` (em vez de criar diretório literal `./~`). *(Decisão técnica registrada.)*
- Em `recursos/mordomo/usuario.toml`:
  - Remover das `preferencias` as frases redundantes que já constam nos campos tipados `tratamento = "senhor"` e `idioma = "pt-BR"`, mantendo preferências complementares (estrutura clara e concisa, decisões práticas destacando riscos reais, resposta direta sem rodeios).
- Em `hermes.rs`:
  - Emitir eventos `tracing::debug!` na resolução de `ConfiguracaoHermes` e geração dos artefatos.

### 5.3 CLI (`crates/sexta-feira-cli/src/main.rs`)

- Adicionar flag global `#[arg(long, global = true)] json: bool` em `Cli`.
- Em `OpcoesAlvo`:
  - Substituir `hermes_home: Option<PathBuf>` por `#[arg(long, alias = "hermes-home")] destino: Option<PathBuf>`.
  - Adicionar `#[arg(long)] persona: Option<PathBuf>` e `#[arg(long)] usuario: Option<PathBuf>`.
- Extrair helper `carregar_contexto(alvo: &OpcoesAlvo) -> anyhow::Result<(DefinicaoDoMordomo, Box<dyn AdaptadorDeSetup>)>` usando `FontesDaDefinicao` (`padrao_do_ambiente()` sobrescrito por `alvo.persona` e `alvo.usuario`).
- Saída texto e `--json` (contrato JSON em `snake_case`, omitindo campos opcionais ausentes com `skip_serializing_if = "Option::is_none"` — *decisão técnica registrada*):
  - `harnesses`: lista `"{:<15} {}"` no modo texto; no modo `--json`, array de objetos `{"id": "<id>", "disponivel": <bool>, "status": "<disponível|em breve>"}`.
  - `plano`: conta `remover` além de `criar`, `atualizar`, `sem_alteracoes` e `conflitos` (exibindo `, {remover} remover` quando `remover > 0`). No modo `--json`: `{"harness": "<id>", "acoes": [{"caminho": "<path>", "operacao": "<criar|atualizar|sem_alteracoes|remover|conflito>", "motivo": "<opcional>", "diff": "<opcional>"}], "resumo": {"criar": n, "atualizar": n, "sem_alteracoes": n, "remover": n, "conflitos": n}}`.
  - `aplicar`: no modo `--json`, `{"harness": "<id>", "criados": n, "atualizados": n, "backups_criados": ["<path>", ...]}`.
  - `verificar`: no modo texto, se `!resultado.sincronizado`, imprime dica (`Dica: execute \`sexta-feira aplicar {}\` para sincronizar.` em `stderr` antes de falhar). No modo `--json`, imprime `{"harness": "<id>", "sincronizado": <bool>, "itens": [{"caminho": "<path>", "sincronizado": <bool>, "detalhe": "<texto>"}]}` em `stdout` antes de sair com erro quando `!resultado.sincronizado`.
- Ajustar `Cargo.toml` raiz e dos crates para adicionar `tracing` (e `serde_json.workspace = true` em `sexta-feira-cli/Cargo.toml`), atualizando [docs/stack.md](../../stack.md) para marcar `tracing` como `em uso`.

### 5.4 Instrumentação com `tracing`

- `sexta-feira-cli`: manter o filtro padrão de `SEXTA_FEIRA_LOG` silencioso no uso normal (`EnvFilter::new("warn")` por padrão) e emitir eventos de execução da CLI em `tracing::debug!` (para que `stderr` fique limpo por padrão e logs detalhados apareçam com `SEXTA_FEIRA_LOG=debug` — *decisão técnica registrada*).
- `sexta-feira-core`: `tracing::debug!` em `DefinicaoDoMordomo::carregar` (camadas aplicadas), `planejar` (quantidade de arquivos desejados e órfãos) e `aplicar` (backups criados e gravação do estado).
- `sexta-feira-adapters`: `tracing::debug!` em `ConfiguracaoHermes::resolver_com_env` e `AdaptadorHermes::planejar_arquivos`.

## 6. O que fica a critério do construtor

- Nomes exatos das structs privadas de serialização `--json` em `main.rs`.
- Ajustes nos testes de snapshot e integração em `crates/sexta-feira-cli/tests/cli.rs` para refletir o novo formato de `harnesses` e cobrir `--destino`, `--hermes-home`, `--persona`, `--usuario`, `--json` e `remover`.
