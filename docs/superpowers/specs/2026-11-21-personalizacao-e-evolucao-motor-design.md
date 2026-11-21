# Design: Personalização externa e evolução do motor declarativo

Data: 2026-11-21
Status: rascunho para revisão

## 1. Intenção e porquê

Com o Recorte 1 entregue (`v0.1.0`), o motor em `sexta-feira-core` só carrega a persona padrão embutida no binário e só sabe escrever arquivos integrais ou blocos delimitados por comentários HTML. Isso impede que quem baixa o binário personalize suas preferências sem recompilar e bloqueia o próximo incremento (configuração estruturada como JSON/TOML, scripts executáveis e evolução segura de arquivos gerenciados).

Esta entrega prepara o `sexta-feira-core` para:
1. Permitir personalização externa em camadas de `persona.toml` e `usuario.toml` com validação estrita.
2. Suportar mesclagem estruturada (JSON e TOML) com detecção de conflitos por chaves gerenciadas, permissões executáveis (`+x`), poda segura de arquivos órfãos e versionamento do arquivo de estado.
3. Corrigir casos de borda no parser de blocos delimitados e no algoritmo de diff.

## 2. Objetivos, não-objetivos e anti-objetivos

### Objetivos
- Endurecer `Persona` e `PerfilDoUsuario` com `#[serde(deny_unknown_fields)]` e adicionar carregamento em camadas via `figment` (`padrao embutido` → diretório de configuração do usuário → arquivos explícitos).
- Adicionar `EstrategiaDeEscrita::MesclagemEstruturada { formato: FormatoEstruturado }` (`Json` e `Toml`) com deep-merge recursivo, preservação de chaves do usuário e detecção de conflito em chaves gerenciadas via `.sexta-feira-estado.toml`.
- Adicionar `PermissaoDeArquivo` (`Padrao` | `Executavel`) em `ArquivoDesejado` e `AcaoPlanejada`, verificando e aplicando `+x` em sistemas Unix.
- Versionar `EstadoAplicado` (`versao = 1`, compatível com arquivos sem campo `versao` da `v0.1.0`) e implementar poda segura de arquivos integrais órfãos (`OperacaoPlanejada::Remover`).
- Tornar `mesclar_bloco` sensível apenas a marcadores em linha própria e proteger `tabela_lcs` contra alocação quadrática excessiva.
- Manter `sexta-feira-adapters` e `sexta-feira-cli` compilando e passando nos testes sem alterar flags de CLI (mudanças de UX da CLI ficam para a PR seguinte).

### Não-objetivos
- Adicionar suporte a YAML ou nova biblioteca fora de [docs/stack.md](../../stack.md) (YAML entra quando o incremento de `config.yaml` do Hermes for desenhado).
- Alterar flags públicas da CLI (`--persona`, `--usuario`, `--destino`, `--json`) ou o conteúdo gerado por `AdaptadorHermes`.
- Implementar novos adapters (`OpenClaw`, `Claude Code`, `Claude Desktop`, `OpenCode`).

### Anti-objetivos
- Quebrar compatibilidade retroativa com `.sexta-feira-estado.toml` já gravado sem o campo `versao`.
- Ignorar silenciosamente chaves desconhecidas em TOMLs de personalização do usuário.
- Sobrescrever chaves de configuração estruturada editadas manualmente pelo usuário sem sinalizar `Conflito` (e sem exigir `--forcar` com backup `.bak`).

## 3. Restrições

- Rust edição 2024, `rust-version = "1.100"`, `unsafe_code = "forbid"`, `missing_docs = "warn"`, `clippy::pedantic`, `unwrap_used` e `expect_used` proibidos fora de testes.
- Nada específico de harness entra em `sexta-feira-core`.
- Dependências novas permitidas apenas se já aprovadas em [docs/stack.md](../../stack.md): `figment` (com feature `toml`) e `serde_json`, com versões em `[workspace.dependencies]` e validadas pelo `cargo deny check`.
- Toda documentação, identificadores de domínio e mensagens ao usuário em `pt-BR`.

## 4. Abordagens consideradas

### 4.1 Personalização em camadas (`definicao.rs`)
1. **Camadas via `figment` (Escolhida):** `Figment` mescla `Toml::string` (embutido), `Toml::file` do diretório padrão do usuário (`$XDG_CONFIG_HOME/sexta-feira/` ou `$HOME/.config/sexta-feira/`, quando existir) e `Toml::file` explícito (obrigatório quando informado). Listas (`tom`, `diretrizes`, `limites`, `preferencias`) substituem a lista base ao serem sobrescritas.
2. **Mesclagem manual sobre `toml::Value`:** evitaria adicionar `figment`, mas reimplementaria lógica de provedores em camadas e perderia a proveniência detalhada de erro por arquivo já prevista em [docs/stack.md](../../stack.md).

### 4.2 Mesclagem estruturada (`plano.rs`)
1. **`MesclagemEstruturada` com rastreamento de projeção gerenciada no estado (Escolhida):** faz deep-merge de objetos/tabelas no destino e registra no `.sexta-feira-estado.toml` o checksum da projeção das chaves gerenciadas pela Sexta-feira. Permite atualizar chaves da Sexta-feira preservando configurações do usuário e acusar `Conflito` quando o usuário editar manualmente uma chave gerenciada.
2. **Mesclagem cega sem estado:** sobrescreveria edições manuais do usuário em chaves gerenciadas sem gerar conflito nem backup `.bak`.

## 5. Desenho da solução

### 5.1 Carregamento em camadas (`crates/sexta-feira-core/src/definicao.rs`)

- Adicionar `#[serde(deny_unknown_fields)]` em `Persona` e `PerfilDoUsuario`.
- Definir `FontesDaDefinicao`:
  - `diretorio_config: Option<PathBuf>`
  - `persona_explicita: Option<PathBuf>`
  - `usuario_explicito: Option<PathBuf>`
  - Construtor `FontesDaDefinicao::padrao_do_ambiente()` e variante injetável `resolver_diretorio_config(xdg_config_home: Option<&str>, home: Option<&str>) -> Option<PathBuf>` (prioriza `XDG_CONFIG_HOME` não vazio; caso contrário usa `<HOME>/.config`, retornando `<base>/sexta-feira`).
- Adicionar método `DefinicaoDoMordomo::carregar(fontes: &FontesDaDefinicao) -> Result<Self, ErroDeDefinicao>`:
  - Para cada seção (`persona` e `usuario`), inicia o `Figment` com `Toml::string` do recurso embutido.
  - Se `diretorio_config` existir e `<dir>/<arquivo>.toml` existir em disco, faz `.merge(Toml::file_exact(...))` (sem subir diretórios ancestrais).
  - Se o caminho explícito (`persona_explicita` / `usuario_explicito`) for `Some(path)`, valida que existe em disco (caso contrário retorna `ErroDeDefinicao::ArquivoNaoEncontrado { caminho }`) e aplica `.merge(Toml::file_exact(path))`.
  - Extrai o tipo alvo (`Persona` ou `PerfilDoUsuario`), mapeia erros do `figment` para novas variantes de `ErroDeDefinicao` (`ConfiguracaoPersonaInvalida` / `ConfiguracaoUsuarioInvalida`) e executa `self.validar()?`.
- Preservar `DefinicaoDoMordomo::padrao()` e `DefinicaoDoMordomo::de_toml(...)` sem quebra de assinatura.

### 5.2 Permissões de arquivo (`crates/sexta-feira-core/src/plano.rs`)

- Definir `PermissaoDeArquivo`:
  - `Padrao` (`#[default]`)
  - `Executavel`
- Incluir `pub permissao: PermissaoDeArquivo` em `ArquivoDesejado` e `AcaoPlanejada`.
- Fornecer `ArquivoDesejado::new(caminho, conteudo, estrategia)` (inicializando `permissao = PermissaoDeArquivo::Padrao`) e método encadeável `pub const fn com_permissao(mut self, permissao: PermissaoDeArquivo) -> Self`.
- Em plataformas Unix (`#[cfg(unix)]`), verificar `fs::metadata(&caminho)?.permissions().mode() & 0o111 != 0`:
  - Se `permissao == Executavel` e o conteúdo já estiver igual em disco, mas o bit `+x` estiver ausente:
    - `planejar` classifica como `OperacaoPlanejada::Atualizar` com `resumo_diff = Some("~ ajuste de permissão para executável (+x)")`.
    - `verificar` reporta `sincronizado: false` com detalhe `"permissão executável ausente"`.
    - `aplicar` garante a permissão executável (`mode | 0o111`) após gravar ou atualizar os metadados do arquivo existente sem sobrescrever desnecessariamente quando só a permissão falta.
  - Nota: se `permissao == Padrao`, `plano.rs` não remove bits de execução já existentes no arquivo (não interfere em umask/permissões externas além de exigir `+x` quando `Executavel`).

### 5.3 Mesclagem estruturada JSON e TOML (`crates/sexta-feira-core/src/plano.rs`)

- Definir `FormatoEstruturado` (`Json`, `Toml`) e adicionar `EstrategiaDeEscrita::MesclagemEstruturada { formato: FormatoEstruturado }`.
- Novo erro `ErroDePlano::ConteudoEstruturadoInvalido { caminho: PathBuf, detalhe: String }` para conteúdo desejado inválido ou cuja raiz não seja objeto JSON / tabela TOML.
- Em `planejar`:
  - Faz parse de `arquivo.conteudo` conforme `formato` (exigindo objeto JSON ou tabela TOML na raiz).
  - Calcula `checksum_desejado` a partir de uma serialização canônica do patch desejado (chaves ordenadas).
  - Se o arquivo alvo não existe em disco:
    - `operacao = OperacaoPlanejada::Criar`
    - `conteudo_final = serializar_bonito(patch_desejado)` (com quebra de linha final `\n`).
  - Se o arquivo alvo existe em disco:
    - Se vazio ou apenas espaços em branco, trata o documento atual como objeto/tabela vazio `{}`.
    - Se o parse do arquivo em disco falhar ou sua raiz não for objeto/tabela: retorna `OperacaoPlanejada::Conflito { motivo: "o arquivo estruturado existente não pôde ser interpretado no formato esperado" }`, mantendo `conteudo_final = serializar_bonito(patch_desejado)` (assim `--forcar` gera `.bak` antes de substituir).
    - Caso o parse do disco tenha sucesso:
      - Executa `mesclar_recursivo(&mut mesclado, &patch_desejado)`: para chaves presentes em ambos onde ambos valores são objetos/tabelas, mescla recursivamente; caso contrário, substitui pelo valor desejado.
      - Se `mesclado == atual_estruturado` (igualdade estrutural): `OperacaoPlanejada::SemAlteracoes` (preservando formatação existente em disco em `conteudo_final`, sem reescrever arquivo por diferença estética).
      - Caso `mesclado != atual_estruturado`:
        - Calcula a projeção do `atual_estruturado` restrita às chaves/folhas definidas em `patch_desejado` (`projetar_chaves_gerenciadas(&atual_estruturado, &patch_desejado)`).
        - Se não existe entrada em `estado.arquivos_estruturados`:
          - Verifica se alguma chave folha gerenciada já existe com valor distinto em `atual_estruturado`. Se nenhuma chave gerenciada colide (apenas chaves do usuário existem), classifica como `OperacaoPlanejada::Atualizar`; caso contrário, `OperacaoPlanejada::Conflito { motivo: "chaves gerenciadas já existiam com valores distintos antes do setup" }`.
        - Se existe entrada `registrado` em `estado.arquivos_estruturados`:
          - Compara `registrado` com `checksum_canonico(&projecao_atual)`. Se igual, `OperacaoPlanejada::Atualizar`; se diferente, `OperacaoPlanejada::Conflito { motivo: "chaves gerenciadas foram modificadas manualmente desde a última aplicação" }`.
        - `conteudo_final = serializar_bonito(&mesclado)` (com `\n` final) e `resumo_diff = Some(gerar_resumo_diff(&atual_normalizado, &conteudo_final))`.
- Em `aplicar`:
  - Atualiza `estado.arquivos_estruturados` com o checksum canônico do patch desejado (para `Criar`, `Atualizar`, `Conflito` forçado e recuperação de `SemAlteracoes` quando ausente/desatualizado no estado).
- Limitação documentada: na mesclagem TOML usando a crate `toml` (`toml::Table`), comentários preexistentes no arquivo alvo não são preservados ao reescrever chaves modificadas; arquivos TOML com comentários do usuário devem continuar usando `BlocoDelimitado`.

### 5.4 Versionamento do estado e poda de órfãos (`crates/sexta-feira-core/src/plano.rs`)

- `EstadoAplicado`:
  - `versao: u32` com `#[serde(default = "versao_estado_atual")]` (`VERSAO_ESTADO_ATUAL = 1`).
  - `arquivos_integrais: BTreeMap<String, String>`
  - `arquivos_estruturados: BTreeMap<String, String>`
- Ao carregar o estado, se `estado.versao != VERSAO_ESTADO_ATUAL` (ex.: `0` ou `> 1`), retornar `ErroDePlano::VersaoDeEstadoNaoSuportada { caminho, versao }`. Estados da `v0.1.0` sem o campo `versao` recebem `1` pelo `serde(default)`.
- Nova variante `OperacaoPlanejada::Remover`:
  - Ao final de `planejar`, para cada `(chave, checksum_registrado)` em `estado.arquivos_integrais` que não pertence a nenhum `ArquivoIntegral` da chamada atual:
    - Resolve o caminho alvo (`resolver_caminho_estado(chave, raiz_estado)`).
    - Se o arquivo existir em disco:
      - Lê o conteúdo normalizado; se `checksum_conteudo == checksum_registrado`, emite `AcaoPlanejada` com `OperacaoPlanejada::Remover` e `resumo_diff` removendo todas as linhas (`- ...`).
      - Se divergir, emite `OperacaoPlanejada::Conflito { motivo: "o arquivo obsoleto foi modificado manualmente desde a última aplicação" }`.
  - Em `aplicar`:
    - Para `OperacaoPlanejada::Remover`: remove o arquivo do disco (e remove a chave de `estado.arquivos_integrais`).
    - Para `OperacaoPlanejada::Conflito` originado de arquivo obsoleto (identificado por não constar nos `arquivos` desejados ou por flag/estrutura interna da ação): com `forcar = true`, cria backup `.bak` antes de remover o arquivo e apaga a chave do estado.
    - Para chaves órfãs de `estado.arquivos_integrais` cujo arquivo já não existe no disco (e chaves órfãs de `estado.arquivos_estruturados`), limpa as entradas obsoletas do estado ao aplicar.
  - Em `verificar`:
    - `OperacaoPlanejada::Remover` conta como `sincronizado: false` com detalhe `"arquivo obsoleto pendente de remoção"`.

### 5.5 Correções pontuais em `mesclar_bloco` e `gerar_resumo_diff` (`plano.rs`)

- **`mesclar_bloco`:** identificar marcadores por linha completa (comparando cada linha após `trim_end_matches([' ', '\t'])` com `inicio` / `fim`), calculando os índices exatos das linhas dos marcadores no texto normalizado. Isso evita falsos positivos quando um marcador aparece dentro de código inline ou texto e impede que espaços à direita na linha de `fim` vazem para a linha seguinte.
- **`gerar_resumo_diff`:** definir teto `LIMITE_CELULAS_LCS = 250_000`. Quando `(miolo_antes.len() + 1).saturating_mul(miolo_depois.len() + 1) > LIMITE_CELULAS_LCS`, emitir diretamente todas as linhas de `miolo_antes` com `- ` seguidas das linhas de `miolo_depois` com `+ `, sem alocar a matriz quadrática.

### 5.6 Ajustes mínimos de compatibilidade no workspace

- Atualizar [crates/sexta-feira-adapters/src/hermes.rs](../../../crates/sexta-feira-adapters/src/hermes.rs) para usar `ArquivoDesejado::new(...)`.
- Atualizar [crates/sexta-feira-cli/src/main.rs](../../../crates/sexta-feira-cli/src/main.rs) no `match` exaustivo de `OperacaoPlanejada` para tratar `OperacaoPlanejada::Remover` (`"REMOVER"` e contador no resumo, ou ajustado conforme padrão atual sem alterar outras flags).

## 6. O que fica a critério do construtor

- Organização de helpers internos em `definicao.rs` e `plano.rs` (mantendo funções enxutas, `clippy::pedantic` e sem duplicação).
- Nomeação exata dos métodos auxiliares privados de projeção/mesclagem JSON/TOML e formatação canônica.
- Detalhes da estrutura interna de `AcaoPlanejada` para distinguir remoção forçada vs substituição forçada caso necessário (sem expor complexidade desnecessária na API pública).
