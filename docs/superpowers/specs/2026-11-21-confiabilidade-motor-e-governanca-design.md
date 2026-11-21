# Design: Confiabilidade do motor declarativo e governança

Data: 2026-11-21
Status: aprovado pelo usuário

## 1. Intenção e porquê

Endurecer os pontos frágeis identificados no motor declarativo (`sexta-feira-core`) após a entrega do Recorte 1 e alinhar a governança do repositório (`AGENTS.md`, `CLAUDE.md` e `Cargo.toml`) antes de avançar para o Recorte 2.

## 2. Objetivos, não-objetivos e anti-objetivos

### Objetivos
- Registrar no `AGENTS.md` a obrigatoriedade de seguir as skills do Superpowers (`brainstorming` → `writing-plans` → `test-driven-development` → `verification-before-completion`).
- Eliminar a duplicação manual entre `AGENTS.md` e `CLAUDE.md` mantendo `CLAUDE.md` como symlink para `AGENTS.md`.
- Alinhar `rust-version = "1.100"` no `Cargo.toml` da raiz com o `rust-toolchain.toml` (`1.100.0`).
- Tornar a gravação de arquivos e a persistência de `.sexta-feira-estado.toml` em `sexta-feira-core::plano` atômicas e resilientes a falhas parciais, perda do arquivo de estado e múltiplos backups `--forcar`.
- Tornar `chave_estado` estável para caminhos relativos/absolutos, `mesclar_bloco` estrito quanto a delimitadores duplicados/quebras de linha e `gerar_resumo_diff` sensível a ordem e multiplicidade de linhas.

### Não-objetivos
- Alterar o comportamento da CLI (`sexta-feira-cli`), do adaptador do Hermes (`sexta-feira-adapters`) ou dos arquivos em `recursos/mordomo/`.
- Introduzir novas bibliotecas externas fora de `docs/stack.md`.
- Implementar recursos do Recorte 2 (`config.yaml`, skills, memória persistente) ou customização local de `persona.toml`/`usuario.toml`.

### Anti-objetivos
- Quebrar compatibilidade com `.sexta-feira-estado.toml` já gerado no Recorte 1 para o caso comum (chaves relativas ao diretório de estado continuam idênticas).
- Sobrescrever silenciosamente backups `.bak` anteriores ao reaplicar com `--forcar`.

## 3. Restrições

- Rust edição 2024, `rust-version = "1.100"`, `unsafe_code = "forbid"`, `missing_docs = "warn"`, `clippy::pedantic`, `unwrap_used` e `expect_used` proibidos fora de testes.
- Nenhuma dependência externa adicionada; toda lógica nova usa exclusivamente `std`.
- Entrega em 1 PR única na branch `fix/confiabilidade-motor-e-governanca`, com TDD (`RED` → `GREEN` → `REFACTOR`) e validação completa antes de abrir a PR.

## 4. Abordagens consideradas

1. **Duas PRs separadas** (`chore(docs)` + `fix(core)`): separa governança de correção de código.
2. **Uma PR única (Escolhida pelo usuário):** reúne os ajustes rápidos de governança e as correções focadas no motor `plano.rs` na mesma entrega para agilizar a base antes do Recorte 2.

## 5. Desenho da solução

### 5.1 Governança e MSRV
- `AGENTS.md`: adicionar item explícito nas regras exigindo o fluxo das skills do Superpowers.
- `CLAUDE.md`: substituir a cópia duplicada por symlink relativo para `AGENTS.md`.
- `Cargo.toml`: atualizar `rust-version = "1.97"` para `rust-version = "1.100"`.

### 5.2 Motor declarativo (`crates/sexta-feira-core/src/plano.rs`)
1. **Escrita atômica (`gravar_arquivo`):** gravar o conteúdo em um arquivo temporário único no mesmo diretório do arquivo alvo (criando o diretório pai se necessário) e executar `fs::rename` para substituir o destino atomicamente; remover o arquivo temporário em caso de erro de escrita.
2. **Backup sem sobrescrita silenciosa (`caminho_backup`):** usar `<arquivo>.bak` quando ainda não existir; caso já exista, incrementar sufixo numérico (`<arquivo>.bak.1`, `<arquivo>.bak.2`, ...) até encontrar um caminho livre.
3. **Persistência resiliente do estado (`aplicar`):**
   - Quando `OperacaoPlanejada::SemAlteracoes` ocorrer para `EstrategiaDeEscrita::ArquivoIntegral` e a entrada estiver ausente ou desatualizada em `.sexta-feira-estado.toml`, atualizar o registro de estado.
   - Em falha de E/S no meio do loop de `aplicar`, se algum arquivo integral já tiver sido gravado ou atualizado no estado (`alterou_estado == true`), persistir `.sexta-feira-estado.toml` antes de retornar o erro.
4. **Chave de estado estável (`chave_estado`):** normalizar lexicalmente (removendo `.` e `..`) e resolver caminhos relativos contra `std::env::current_dir()` antes de calcular `strip_prefix` contra o diretório pai de `arquivo_estado`, mantendo chaves relativas idênticas às atuais quando dentro da raiz e determinísticas quando fora.
5. **Blocos delimitados estritos (`mesclar_bloco`):** exigir exatamente `(0, 0)` ou `(1, 1)` ocorrências dos marcadores `inicio` e `fim` (com `i + inicio.len() <= f`). Múltiplas ocorrências ou ordem invertida retornam `ErroDePlano::DelimitadoresInvalidos`. Ao substituir bloco existente quando `antes` não está vazio e não termina em `\n`, inserir `\n` antes de `inicio`.
6. **Diff por sequência (`gerar_resumo_diff`):** substituir a checagem por `.contains(...)` por algoritmo LCS (Longest Common Subsequence) em Rust puro sobre as linhas, preservando multiplicidade e reordenações.

## 6. O que fica a critério do construtor

- Nomes internos das funções auxiliares privadas em `plano.rs` (desde que respeitem `pt-BR`, `clippy::pedantic` e tamanho enxuto).
- Formato da geração do nome do arquivo temporário irmão em `gravar_arquivo` (ex.: uso de PID + contador atômico para segurança em testes paralelos).
