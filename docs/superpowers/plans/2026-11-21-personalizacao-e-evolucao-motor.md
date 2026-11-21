# Plano de Implementação: Personalização externa e evolução do motor declarativo

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Evoluir `sexta-feira-core` para carregar personalizações externas em camadas (`persona.toml` e `usuario.toml`), suportar mesclagem estruturada JSON/TOML com detecção de conflitos, aplicar permissões executáveis (`+x`), podar arquivos integrais órfãos, versionar `.sexta-feira-estado.toml` e corrigir casos de borda de blocos delimitados e diff.

**Architecture:** As alterações de domínio ficam concentradas em `sexta-feira-core` (`definicao.rs`, `plano.rs` e novo submódulo privado `estruturado.rs` para isolar o parsing/merge de JSON e TOML). Os demais crates do workspace (`sexta-feira-adapters` e `sexta-feira-cli`) recebem apenas os ajustes mínimos necessários para compilar com os novos campos e variantes, sem alterar flags de CLI nem conteúdo de saída do Hermes nesta PR.

**Tech Stack:** Rust edição 2024 (`1.100.0`), `serde`, `toml`, `thiserror`, `figment` (feature `toml`), `serde_json`, `cargo test` / `cargo clippy` / `cargo fmt` / `cargo doc`.

**Spec:** [docs/superpowers/specs/2026-11-21-personalizacao-e-evolucao-motor-design.md](../specs/2026-11-21-personalizacao-e-evolucao-motor-design.md)

## Global Constraints

- Rust edição 2024, `rust-version = "1.100"`, `unsafe_code = "forbid"`, `missing_docs = "warn"`, `clippy::pedantic`, `unwrap_used` e `expect_used` proibidos fora de testes (`Cargo.toml`).
- Nada específico de harness entra em `sexta-feira-core` (`AGENTS.md`).
- Documentação, commits, comentários e identificadores de domínio em `pt-BR` (`AGENTS.md`).
- Dependências novas restritas às aprovadas em [docs/stack.md](../../stack.md) (`figment` e `serde_json`), declaradas em `[workspace.dependencies]` e só adicionadas quando o código passar a usá-las.
- Todo trabalho ocorre na branch `feat/personalizacao-e-evolucao-motor`, nunca na `main` (`AGENTS.md`).
- TDD obrigatório em todas as alterações: teste falhando (`RED`) → verificação da falha → implementação mínima (`GREEN`) → suíte verde e refatoração.

## Review Focus

- **Sobrescrita parcial vs substituição de lista em camadas (`definicao.rs`):** Sobrescrever apenas `tratamento = "chefe"` em `~/.config/sexta-feira/usuario.toml` deve manter `idioma` e `preferencias` do padrão embutido; informar `preferencias = [...]` deve substituir a lista padrão (sem concatenar), e chave desconhecida como `tratamentos` deve falhar por `deny_unknown_fields`.
- **Colisão em arquivo estruturado preexistente (`plano.rs`):** Um `settings.json` já existente com `{"tema": "escuro", "mcpServers": {"sexta": "antigo"}}` sem estado registrado, recebendo `mcpServers.sexta = "novo"`, deve gerar `Conflito` sem `--forcar` e criar `.bak` preservando `"tema": "escuro"` com `--forcar`.
- **Permissão `+x` ausente quando conteúdo já bate (`plano.rs`):** Arquivo em disco com conteúdo igual ao desejado e `permissao = Executavel`, mas sem bit `+x` em Unix, deve ser planejado como `Atualizar`, reprovado em `verificar` e corrigido em `aplicar`.
- **Arquivo órfão editado manualmente (`plano.rs`):** Arquivo integral anteriormente gerenciado que saiu da lista de desejados e foi alterado manualmente no disco deve gerar `Conflito` (abortando `aplicar` sem `--forcar` e gerando `.bak` antes da remoção com `--forcar`).
- **Retrocompatibilidade e rejeição de versão futura no estado (`plano.rs`):** `.sexta-feira-estado.toml` da `v0.1.0` sem campo `versao` deve carregar como `versao = 1`; arquivo com `versao = 0` ou `versao = 99` deve retornar `ErroDePlano::VersaoDeEstadoNaoSuportada`.

---

## Mapa de Arquivos

- `Cargo.toml` — adiciona `figment` (Task 1) e `serde_json` (Task 5) em `[workspace.dependencies]`.
- `crates/sexta-feira-core/Cargo.toml` — habilita `figment.workspace = true` (Task 1) e `serde_json.workspace = true` (Task 5).
- `docs/stack.md` — atualiza situação de `figment` e `serde_json` para `"em uso"`.
- `crates/sexta-feira-core/src/lib.rs` — reexporta `FontesDaDefinicao`, `FormatoEstruturado` e `PermissaoDeArquivo`.
- `crates/sexta-feira-core/src/definicao.rs` — `#[serde(deny_unknown_fields)]`, `FontesDaDefinicao`, `DefinicaoDoMordomo::carregar` e novas variantes de `ErroDeDefinicao`.
- `crates/sexta-feira-core/src/estruturado.rs` — novo submódulo privado com parsing, serialização canônica/bonita, deep-merge recursivo e projeção de chaves para JSON e TOML.
- `crates/sexta-feira-core/src/plano.rs` — `VERSAO_ESTADO_ATUAL`, `PermissaoDeArquivo`, `FormatoEstruturado`, `EstrategiaDeEscrita::MesclagemEstruturada`, `OperacaoPlanejada::Remover`, correções em `mesclar_bloco` e teto em `tabela_lcs`.
- `crates/sexta-feira-adapters/src/hermes.rs` — ajuste mínimo para `ArquivoDesejado::new(...)`.
- `crates/sexta-feira-cli/src/main.rs` — ajuste mínimo no `match` de `OperacaoPlanejada::Remover`.

---

### Task 1: Carregamento em camadas e validação estrita em `definicao.rs`

**Files:**
- Modify: `Cargo.toml:13-25`
- Modify: `crates/sexta-feira-core/Cargo.toml:11-15`
- Modify: `docs/stack.md:8-12`
- Modify: `crates/sexta-feira-core/src/definicao.rs:1-189`
- Modify: `crates/sexta-feira-core/src/lib.rs:10-16`

**Interfaces:**
- Consumes: `figment = { version = "0.10.19", features = ["toml"] }`.
- Produces:
  - `pub struct FontesDaDefinicao { pub diretorio_config: Option<PathBuf>, pub persona_explicita: Option<PathBuf>, pub usuario_explicito: Option<PathBuf> }`
  - `FontesDaDefinicao::padrao_do_ambiente() -> Self`
  - `FontesDaDefinicao::resolver_diretorio_config(xdg_config_home: Option<&str>, home: Option<&str>) -> Option<PathBuf>`
  - `DefinicaoDoMordomo::carregar(fontes: &FontesDaDefinicao) -> Result<Self, ErroDeDefinicao>`
  - Novas variantes em `ErroDeDefinicao`: `ArquivoNaoEncontrado { caminho: PathBuf }`, `ConfiguracaoPersonaInvalida { detalhe: String }`, `ConfiguracaoUsuarioInvalida { detalhe: String }`

- [ ] **Step 1: Adicionar `figment` ao workspace e atualizar `docs/stack.md`**

Em `Cargo.toml` (`[workspace.dependencies]`):
```toml
figment = { version = "0.10.19", features = ["toml"] }
```
Em `crates/sexta-feira-core/Cargo.toml`:
```toml
figment.workspace = true
```
Em `docs/stack.md`, atualizar a linha de `figment` para `em uso`.

- [ ] **Step 2: Escrever testes falhando em `crates/sexta-feira-core/src/definicao.rs`**

Adicionar testes no módulo `tests` (usando diretório temporário próprio com RAII):
```rust
#[test]
fn resolve_diretorio_config_priorizando_xdg_sobre_home() {
    assert_eq!(
        FontesDaDefinicao::resolver_diretorio_config(Some("/tmp/xdg"), Some("/home/u")),
        Some(PathBuf::from("/tmp/xdg/sexta-feira"))
    );
    assert_eq!(
        FontesDaDefinicao::resolver_diretorio_config(Some("  "), Some("/home/u")),
        Some(PathBuf::from("/home/u/.config/sexta-feira"))
    );
    assert_eq!(
        FontesDaDefinicao::resolver_diretorio_config(None, None),
        None
    );
}

#[test]
fn carregar_aplica_camadas_e_substitui_listas_sem_perder_campos_omitidos() {
    let dir = DirTemporario::novo("camadas");
    fs::write(dir.path().join("usuario.toml"), "tratamento = \"chefe\"\n").expect("grava dir");
    let explicito = dir.path().join("usuario-explicito.toml");
    fs::write(&explicito, "preferencias = [\"Direto ao ponto\"]\n").expect("grava explicito");

    let fontes = FontesDaDefinicao {
        diretorio_config: Some(dir.path().to_path_buf()),
        persona_explicita: None,
        usuario_explicito: Some(explicito),
    };
    let def = DefinicaoDoMordomo::carregar(&fontes).expect("carrega em camadas");
    assert_eq!(def.persona.nome, "Sexta-feira");
    assert_eq!(def.usuario.tratamento, "chefe");
    assert_eq!(def.usuario.idioma, "pt-BR");
    assert_eq!(def.usuario.preferencias, vec!["Direto ao ponto"]);
}

#[test]
fn carregar_rejeita_arquivo_explicito_ausente_e_chave_desconhecida() {
    let fontes_ausente = FontesDaDefinicao {
        diretorio_config: None,
        persona_explicita: Some(PathBuf::from("/tmp/nao-existe-sexta-feira-persona.toml")),
        usuario_explicito: None,
    };
    assert!(matches!(
        DefinicaoDoMordomo::carregar(&fontes_ausente),
        Err(ErroDeDefinicao::ArquivoNaoEncontrado { .. })
    ));

    let dir = DirTemporario::novo("desconhecido");
    let usuario_errado = dir.path().join("usuario.toml");
    fs::write(&usuario_errado, "tratamentos = \"senhor\"\n").expect("grava chave invalida");
    let fontes_chave = FontesDaDefinicao {
        diretorio_config: None,
        persona_explicita: None,
        usuario_explicito: Some(usuario_errado),
    };
    assert!(matches!(
        DefinicaoDoMordomo::carregar(&fontes_chave),
        Err(ErroDeDefinicao::ConfiguracaoUsuarioInvalida { .. })
    ));
}
```

- [ ] **Step 3: Rodar os testes para confirmar falha (`RED`)**

Run: `cargo test -p sexta-feira-core definicao::tests`
Expected: FAIL de compilação (`FontesDaDefinicao`, `carregar` e variantes de erro não definidos).

- [ ] **Step 4: Implementar `FontesDaDefinicao`, `deny_unknown_fields` e `DefinicaoDoMordomo::carregar` + reexport em `lib.rs`**

Em `crates/sexta-feira-core/src/definicao.rs`:
- Adicionar `#[serde(deny_unknown_fields)]` em `Persona` e `PerfilDoUsuario`.
- Implementar `FontesDaDefinicao` com `Default`, `padrao_do_ambiente()` (lendo `XDG_CONFIG_HOME` e `HOME`) e `resolver_diretorio_config`.
- Implementar `DefinicaoDoMordomo::carregar`: helper interno `carregar_secao<T: serde::de::DeserializeOwned>` iniciando `Figment::from(Toml::string(base_toml))`, mesclando `<dir>/<arquivo>.toml` se existir via `Toml::file_exact` e validando/mesclando `explicito` via `Toml::file_exact` (retornando `ArquivoNaoEncontrado` se `explicito` não existir no disco). Extrair e chamar `definicao.validar()?`.
- Reexportar `FontesDaDefinicao` em `crates/sexta-feira-core/src/lib.rs`.

- [ ] **Step 5: Rodar testes e checagens para verificar sucesso (`GREEN`)**

Run: `cargo test -p sexta-feira-core && cargo clippy -p sexta-feira-core --all-targets -- -D warnings`
Expected: PASS com todos os testes verdes e zero warnings.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock docs/stack.md crates/sexta-feira-core/Cargo.toml crates/sexta-feira-core/src/definicao.rs crates/sexta-feira-core/src/lib.rs
git commit -m "feat(core): suporta personalização em camadas com figment e schema estrito"
```

---

### Task 2: Versionamento do estado, correções em `mesclar_bloco` e teto em `tabela_lcs`

**Files:**
- Modify: `crates/sexta-feira-core/src/plano.rs:109-160,401-452,537-550,637-706`

**Interfaces:**
- Produces:
  - Constante privada `VERSAO_ESTADO_ATUAL: u32 = 1`
  - Campo `versao: u32` em `EstadoAplicado` (`#[serde(default = "versao_estado_atual")]`)
  - `ErroDePlano::VersaoDeEstadoNaoSuportada { caminho: PathBuf, versao: u32 }`
  - `LIMITE_CELULAS_LCS: usize = 250_000`

- [ ] **Step 1: Escrever testes falhando em `crates/sexta-feira-core/src/plano.rs`**

```rust
#[test]
fn carregar_estado_aceita_formato_legado_sem_versao_e_rejeita_versao_nao_suportada() {
    let dir = DirTemporario::novo("versao-estado");
    let estado = dir.path().join(".sexta-feira-estado.toml");
    let soul = dir.path().join("SOUL.md");
    fs::write(
        &estado,
        format!("[arquivos_integrais]\n\"SOUL.md\" = \"{}\"\n", checksum_conteudo("# V1\n")),
    )
    .expect("grava estado legado");
    fs::write(&soul, "# V1\n").expect("grava soul");

    let plano = planejar(
        vec![ArquivoDesejado {
            caminho: soul.clone(),
            conteudo: "# V2\n".to_owned(),
            estrategia: EstrategiaDeEscrita::ArquivoIntegral,
        }],
        estado.clone(),
    )
    .expect("estado legado sem versao deve ser aceito");
    assert!(matches!(plano.acoes[0].operacao, OperacaoPlanejada::Atualizar));
    aplicar(&plano, false).expect("grava estado versionado");
    let estado_salvo = fs::read_to_string(&estado).expect("le estado");
    assert!(estado_salvo.contains("versao = 1"));

    fs::write(&estado, "versao = 99\n").expect("grava versao futura");
    let err = planejar(
        vec![ArquivoDesejado {
            caminho: soul,
            conteudo: "# V2\n".to_owned(),
            estrategia: EstrategiaDeEscrita::ArquivoIntegral,
        }],
        estado,
    )
    .expect_err("versao futura deve falhar");
    assert!(matches!(
        err,
        ErroDePlano::VersaoDeEstadoNaoSuportada { versao: 99, .. }
    ));
}

#[test]
fn bloco_delimitado_ignora_marcador_inline_e_espacos_a_direita_no_marcador() {
    let dir = DirTemporario::novo("marcador-por-linha");
    let estado = dir.path().join(".sexta-feira-estado.toml");
    let user = dir.path().join("USER.md");
    let inicio = "<!-- sexta-feira:inicio -->";
    let fim = "<!-- sexta-feira:fim -->";

    fs::write(
        &user,
        format!("{inicio}  \nantigo\n{fim}\t \nNota: usar `{inicio}` no exemplo.\n"),
    )
    .expect("grava arquivo");

    let desejados = vec![ArquivoDesejado {
        caminho: user.clone(),
        conteudo: "novo\n".to_owned(),
        estrategia: EstrategiaDeEscrita::BlocoDelimitado { inicio, fim },
    }];
    let plano = planejar(desejados, estado).expect("nao deve confundir marcador inline");
    aplicar(&plano, false).expect("aplica");

    let final_txt = fs::read_to_string(&user).expect("le final");
    assert_eq!(
        final_txt,
        format!("{inicio}\nnovo\n{fim}\nNota: usar `{inicio}` no exemplo.\n")
    );
}

#[test]
fn resumo_diff_usa_fallback_linear_acima_do_limite_lcs() {
    let mut antes = String::new();
    let mut depois = String::new();
    for idx in 0..600 {
        let _ = writeln!(antes, "antiga {idx}");
        let _ = writeln!(depois, "nova {idx}");
    }
    let diff = gerar_resumo_diff(&antes, &depois);
    assert!(diff.contains("- antiga 0"));
    assert!(diff.contains("+ nova 599"));
}
```

- [ ] **Step 2: Rodar os testes para confirmar falha (`RED`)**

Run: `cargo test -p sexta-feira-core plano::tests`
Expected: FAIL (`VersaoDeEstadoNaoSuportada` não existe e marcador inline causa `DelimitadoresInvalidos`).

- [ ] **Step 3: Implementar versionamento do estado, matching por linha em `mesclar_bloco` e teto LCS**

Em `crates/sexta-feira-core/src/plano.rs`:
- Adicionar `VERSAO_ESTADO_ATUAL = 1`, `versao` em `EstadoAplicado` com `#[serde(default = "versao_estado_atual")]` (mantendo o campo serializado no topo), e validação `if estado.versao != VERSAO_ESTADO_ATUAL` em `carregar_estado`.
- Em `mesclar_bloco`: percorrer linhas com offsets (`split_inclusive('\n')` ou rastreamento de offset por linha) verificando `linha.trim_end_matches('\n').trim_end_matches([' ', '\t']) == marcador`. Garantir exatamente `(0, 0)` ou `(1, 1)` linhas marcadoras com `inicio_offset < fim_offset`, substituindo exatamente o intervalo entre o início da linha `inicio` e o fim da linha `fim` (consumindo a `\n` final da linha `fim`, se houver).
- Em `gerar_resumo_diff`: se `(miolo_antes.len() + 1).saturating_mul(miolo_depois.len() + 1) > LIMITE_CELULAS_LCS`, emitir `-` para `miolo_antes` seguido de `+` para `miolo_depois` sem chamar `tabela_lcs`.

- [ ] **Step 4: Rodar testes para verificar sucesso (`GREEN`)**

Run: `cargo test -p sexta-feira-core && cargo clippy -p sexta-feira-core --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-core/src/plano.rs
git commit -m "fix(core): versiona estado, casa delimitadores por linha e limita matriz LCS"
```

---

### Task 3: Permissões executáveis (`+x`) em `ArquivoDesejado`

**Files:**
- Modify: `crates/sexta-feira-core/src/plano.rs:27-70,242-310,312-400,476-510,745-1179`
- Modify: `crates/sexta-feira-core/src/lib.rs:10-16`
- Modify: `crates/sexta-feira-adapters/src/hermes.rs:6-9,150-166`

**Interfaces:**
- Produces:
  - `pub enum PermissaoDeArquivo { #[default] Padrao, Executavel }`
  - Campo `pub permissao: PermissaoDeArquivo` em `ArquivoDesejado` e `AcaoPlanejada`
  - `ArquivoDesejado::new(caminho: PathBuf, conteudo: String, estrategia: EstrategiaDeEscrita) -> Self`
  - `ArquivoDesejado::com_permissao(self, permissao: PermissaoDeArquivo) -> Self`

- [ ] **Step 1: Escrever testes falhando em `crates/sexta-feira-core/src/plano.rs`**

```rust
#[cfg(unix)]
#[test]
fn arquivo_executavel_aplica_e_verifica_permissao_unix() {
    use std::os::unix::fs::PermissionsExt;

    let dir = DirTemporario::novo("permissao-exec");
    let estado = dir.path().join(".sexta-feira-estado.toml");
    let script = dir.path().join("bin/rotina.sh");

    let desejados = vec![
        ArquivoDesejado::new(
            script.clone(),
            "#!/bin/sh\necho ok\n".to_owned(),
            EstrategiaDeEscrita::ArquivoIntegral,
        )
        .com_permissao(PermissaoDeArquivo::Executavel),
    ];

    let p1 = planejar(desejados.clone(), estado.clone()).expect("plano criacao");
    aplicar(&p1, false).expect("aplica script");
    let mode = fs::metadata(&script).expect("meta").permissions().mode();
    assert_ne!(mode & 0o111, 0, "script criado deve ter bit +x");
    assert!(verificar(&desejados, &estado).expect("verifica").sincronizado);

    let mut perms = fs::metadata(&script).expect("meta").permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&script, perms).expect("remove +x");

    let verif_sem_x = verificar(&desejados, &estado).expect("verifica sem +x");
    assert!(!verif_sem_x.sincronizado);
    assert!(verif_sem_x.itens[0].detalhe.contains("permissão"));

    let p2 = planejar(desejados.clone(), estado.clone()).expect("plano ajuste +x");
    assert!(matches!(p2.acoes[0].operacao, OperacaoPlanejada::Atualizar));
    let res = aplicar(&p2, false).expect("reaplica +x");
    assert_eq!(res.atualizados, 1);
    assert!(verificar(&desejados, &estado).expect("verifica final").sincronizado);
}
```

- [ ] **Step 2: Rodar os testes para confirmar falha (`RED`)**

Run: `cargo test -p sexta-feira-core permissao_exec`
Expected: FAIL de compilação (`PermissaoDeArquivo`, `ArquivoDesejado::new`, `com_permissao` inexistentes).

- [ ] **Step 3: Implementar `PermissaoDeArquivo`, atualizar `ArquivoDesejado` / `AcaoPlanejada` e adaptar `hermes.rs` e `lib.rs`**

- Em `plano.rs`:
  - Definir `PermissaoDeArquivo` (`Padrao`, `Executavel`), adicionar `permissao` a `ArquivoDesejado` e `AcaoPlanejada`, criar `new` e `com_permissao`.
  - Helper `permissao_pendente(caminho: &Path, permissao: PermissaoDeArquivo) -> Result<bool, ErroDePlano>` (em `#[cfg(unix)]`, checa `mode & 0o111 == 0` quando `Executavel`; em `#[cfg(not(unix))]`, retorna `Ok(false)`).
  - Helper `garantir_permissao(caminho: &Path, permissao: PermissaoDeArquivo) -> Result<(), ErroDePlano>` (em `#[cfg(unix)]`, se `Executavel`, seta `mode | 0o111`, ou `0o755` se ainda zero — `perms.set_mode(perms.mode() | 0o111)`).
  - Em `planejar_integral` e `planejar_bloco_delimitado`: quando conteúdo já está igual em disco, consultar `permissao_pendente`; se `true`, retornar `OperacaoPlanejada::Atualizar` com `resumo_diff = Some("~ ajuste de permissão para executável (+x)".to_owned())`.
  - Em `verificar`: distinguir detalhe `"permissão executável ausente"` quando `Atualizar` tiver esse motivo (ou helper checando `permissao_pendente` / `acao.resumo_diff`).
  - Em `aplicar_acao`: após `gravar_arquivo` (ou ajustando permissão se conteúdo já estava idêntico — mesmo regravar atomicamente e chamar `garantir_permissao` garante `+x`), chamar `garantir_permissao(&acao.caminho, acao.permissao)?`.
  - Atualizar os testes existentes em `plano.rs` e as duas structs em `hermes.rs` para preencher `permissao: PermissaoDeArquivo::Padrao` ou `ArquivoDesejado::new(...)`.
  - Reexportar `PermissaoDeArquivo` em `crates/sexta-feira-core/src/lib.rs`.

- [ ] **Step 4: Rodar testes em todo o workspace (`GREEN`)**

Run: `cargo test --workspace --locked && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-core/src/plano.rs crates/sexta-feira-core/src/lib.rs crates/sexta-feira-adapters/src/hermes.rs
git commit -m "feat(core): suporta permissão executável em ArquivoDesejado"
```

---

### Task 4: Poda segura de arquivos integrais órfãos (`OperacaoPlanejada::Remover`)

**Files:**
- Modify: `crates/sexta-feira-core/src/plano.rs:38-68,164-310`
- Modify: `crates/sexta-feira-cli/src/main.rs:94-125`

**Interfaces:**
- Produces:
  - Nova variante `OperacaoPlanejada::Remover`
  - Campo privado ou método auxiliar interno em `AcaoPlanejada` (ou campo `remocao` privado não exposto, mantendo campos públicos estáveis) para que `aplicar` saiba remover em vez de gravar quando `Conflito` vem de órfão forçado.

- [ ] **Step 1: Escrever testes falhando em `crates/sexta-feira-core/src/plano.rs`**

```rust
#[test]
fn poda_arquivo_integral_orfao_com_seguranca_e_respeita_conflito() {
    let dir = DirTemporario::novo("poda-orfao");
    let estado = dir.path().join(".sexta-feira-estado.toml");
    let a = dir.path().join("A.md");
    let b = dir.path().join("B.md");

    let v1 = vec![
        ArquivoDesejado::new(a.clone(), "# A\n".to_owned(), EstrategiaDeEscrita::ArquivoIntegral),
        ArquivoDesejado::new(b.clone(), "# B\n".to_owned(), EstrategiaDeEscrita::ArquivoIntegral),
    ];
    let p1 = planejar(v1, estado.clone()).expect("plano inicial");
    aplicar(&p1, false).expect("aplica inicial");

    let v2 = vec![
        ArquivoDesejado::new(a.clone(), "# A\n".to_owned(), EstrategiaDeEscrita::ArquivoIntegral),
    ];
    let p_rem = planejar(v2.clone(), estado.clone()).expect("plano remocao");
    assert!(p_rem.acoes.iter().any(|ac| ac.caminho == b && matches!(ac.operacao, OperacaoPlanejada::Remover)));
    assert!(!verificar(&v2, &estado).expect("verifica antes").sincronizado);

    aplicar(&p_rem, false).expect("aplica remocao");
    assert!(!b.exists(), "arquivo orfao deve ser removido");
    assert!(verificar(&v2, &estado).expect("verifica depois").sincronizado);

    // Agora testa conflito quando orfao foi editado manualmente antes da poda
    let v3 = vec![
        ArquivoDesejado::new(a.clone(), "# A\n".to_owned(), EstrategiaDeEscrita::ArquivoIntegral),
        ArquivoDesejado::new(b.clone(), "# B\n".to_owned(), EstrategiaDeEscrita::ArquivoIntegral),
    ];
    aplicar(&planejar(v3, estado.clone()).expect("recria B"), false).expect("aplica B");
    fs::write(&b, "# B editado manualmente\n").expect("edita B");

    let p_conflito = planejar(v2.clone(), estado.clone()).expect("plano conflito orfao");
    assert!(p_conflito.acoes.iter().any(|ac| ac.caminho == b && matches!(ac.operacao, OperacaoPlanejada::Conflito { .. })));
    assert!(matches!(aplicar(&p_conflito, false), Err(ErroDePlano::ConflitoPendente { .. })));
    assert!(b.exists(), "sem --forcar nao pode apagar arquivo editado");

    let res_forcar = aplicar(&p_conflito, true).expect("forcar remocao com backup");
    assert!(!b.exists());
    assert_eq!(res_forcar.backups_criados, vec![dir.path().join("B.md.bak")]);
    assert!(verificar(&v2, &estado).expect("verifica final").sincronizado);
}
```

- [ ] **Step 2: Rodar o teste para confirmar falha (`RED`)**

Run: `cargo test -p sexta-feira-core poda_arquivo_integral`
Expected: FAIL (`OperacaoPlanejada::Remover` inexistente).

- [ ] **Step 3: Implementar detecção e aplicação de remoção segura de órfãos + ajuste mínimo em `cli/main.rs`**

- Em `plano.rs`:
  - Adicionar `OperacaoPlanejada::Remover`.
  - Em `planejar`: guardar `BTreeSet<String>` com as chaves dos `ArquivoIntegral` presentes em `arquivos`. Após o loop, iterar por `estado.arquivos_integrais`: para cada `(chave, checksum_reg)` fora do conjunto, resolver caminho (`raiz_estado.join(chave)` se relativo); se existir em disco, ler e comparar `checksum_conteudo` com `checksum_reg`. Emitir `Remover` (ou `Conflito` marcado internamente como remoção pendente) conforme igualdade.
  - Em `aplicar`: ao remover (seja `Remover` ou `Conflito` de remoção com `forcar = true`, copiando `.bak` antes), chamar `fs::remove_file` e apagar a chave de `estado.arquivos_integrais`. Também remover do estado quaisquer chaves órfãs de `arquivos_integrais` cujo arquivo não exista mais no disco (e chaves órfãs de `arquivos_estruturados`).
  - Em `verificar`: mapear `OperacaoPlanejada::Remover` para `(false, "arquivo obsoleto pendente de remoção".to_owned())`.
- Em `crates/sexta-feira-cli/src/main.rs`:
  - No `match` de `executar_plano`, adicionar braço `OperacaoPlanejada::Remover => "REMOVER"` (sem alterar contadores ou saída dos casos existentes).

- [ ] **Step 4: Rodar testes de todo o workspace (`GREEN`)**

Run: `cargo test --workspace --locked && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sexta-feira-core/src/plano.rs crates/sexta-feira-cli/src/main.rs
git commit -m "feat(core): poda arquivos integrais órfãos com detecção de conflito"
```

---

### Task 5: Mesclagem estruturada JSON e TOML (`estruturado.rs` + `plano.rs`)

**Files:**
- Modify: `Cargo.toml:13-25`
- Modify: `crates/sexta-feira-core/Cargo.toml:11-16`
- Modify: `docs/stack.md:8-12`
- Create: `crates/sexta-feira-core/src/estruturado.rs`
- Modify: `crates/sexta-feira-core/src/plano.rs`
- Modify: `crates/sexta-feira-core/src/lib.rs:6-16`

**Interfaces:**
- Consumes: `serde_json = "1.0.154"`, `toml` (já presente).
- Produces:
  - `pub enum FormatoEstruturado { Json, Toml }`
  - `EstrategiaDeEscrita::MesclagemEstruturada { formato: FormatoEstruturado }`
  - `ErroDePlano::ConteudoEstruturadoInvalido { caminho: PathBuf, detalhe: String }`
  - Módulo privado `estruturado` com funções de parse/mesclagem/projeção/serialização canônica e legível.

- [ ] **Step 1: Adicionar `serde_json` ao workspace e atualizar `docs/stack.md`**

Em `Cargo.toml` (`[workspace.dependencies]`):
```toml
serde_json = "1.0.154"
```
Em `crates/sexta-feira-core/Cargo.toml`:
```toml
serde_json.workspace = true
```
Em `docs/stack.md`, atualizar a linha de `serde_json` para `em uso`.

- [ ] **Step 2: Escrever testes falhando em `crates/sexta-feira-core/src/plano.rs`**

```rust
#[test]
fn mesclagem_estruturada_json_preserva_chaves_usuario_e_detecta_conflito_em_chaves_gerenciadas() {
    let dir = DirTemporario::novo("merge-json");
    let estado = dir.path().join(".sexta-feira-estado.toml");
    let cfg = dir.path().join("settings.json");

    fs::write(&cfg, "{\n    \"tema\": \"escuro\",\n    \"mcpServers\": {\n        \"outro\": { \"cmd\": \"x\" }\n    }\n}\n")
        .expect("grava json previo");

    let desejados_v1 = vec![ArquivoDesejado::new(
        cfg.clone(),
        r#"{"mcpServers":{"sexta-feira":{"command":"sexta-feira"}}}"#.to_owned(),
        EstrategiaDeEscrita::MesclagemEstruturada {
            formato: FormatoEstruturado::Json,
        },
    )];

    let p1 = planejar(desejados_v1.clone(), estado.clone()).expect("plano v1");
    assert!(matches!(p1.acoes[0].operacao, OperacaoPlanejada::Atualizar));
    aplicar(&p1, false).expect("aplica v1");

    let lido: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&cfg).expect("le cfg")).expect("json valido");
    assert_eq!(lido["tema"], "escuro");
    assert_eq!(lido["mcpServers"]["outro"]["cmd"], "x");
    assert_eq!(lido["mcpServers"]["sexta-feira"]["command"], "sexta-feira");
    assert!(verificar(&desejados_v1, &estado).expect("verifica").sincronizado);

    // Usuário altera chave própria mantendo formatação customizada: deve continuar SemAlteracoes
    let custom = fs::read_to_string(&cfg).expect("le").replace("\"escuro\"", "\"claro\"");
    fs::write(&cfg, custom).expect("altera chave do usuario");
    let p_user = planejar(desejados_v1.clone(), estado.clone()).expect("plano apos chave propria");
    assert!(matches!(p_user.acoes[0].operacao, OperacaoPlanejada::SemAlteracoes));

    // Usuário altera chave gerenciada: deve gerar Conflito e exigir --forcar
    let adulterado = fs::read_to_string(&cfg)
        .expect("le")
        .replace("\"command\": \"sexta-feira\"", "\"command\": \"manual\"");
    fs::write(&cfg, adulterado).expect("adultera chave gerenciada");
    let p_conflito = planejar(desejados_v1, estado.clone()).expect("plano conflito");
    assert!(matches!(p_conflito.acoes[0].operacao, OperacaoPlanejada::Conflito { .. }));
    assert!(matches!(aplicar(&p_conflito, false), Err(ErroDePlano::ConflitoPendente { .. })));

    let res_forcar = aplicar(&p_conflito, true).expect("forcar merge");
    assert_eq!(res_forcar.backups_criados.len(), 1);
    let final_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&cfg).expect("le final")).expect("json valido");
    assert_eq!(final_json["tema"], "claro");
    assert_eq!(final_json["mcpServers"]["sexta-feira"]["command"], "sexta-feira");
}

#[test]
fn mesclagem_estruturada_rejeita_colisao_preexistente_sem_estado_e_suporta_toml() {
    let dir = DirTemporario::novo("merge-toml");
    let estado = dir.path().join(".sexta-feira-estado.toml");
    let cfg = dir.path().join("config.toml");

    fs::write(&cfg, "[usuario]\ntratamento = \"manual\"\n[extra]\nmanter = true\n").expect("toml previo");
    let desejados = vec![ArquivoDesejado::new(
        cfg.clone(),
        "[usuario]\ntratamento = \"senhor\"\n".to_owned(),
        EstrategiaDeEscrita::MesclagemEstruturada {
            formato: FormatoEstruturado::Toml,
        },
    )];

    let p_colisao = planejar(desejados.clone(), estado.clone()).expect("plano colisao");
    assert!(matches!(p_colisao.acoes[0].operacao, OperacaoPlanejada::Conflito { .. }));
    aplicar(&p_colisao, true).expect("forca sobre colisao");

    let conteudo = fs::read_to_string(&cfg).expect("le toml");
    assert!(conteudo.contains("manter = true"));
    assert!(conteudo.contains("tratamento = \"senhor\""));
    assert!(verificar(&desejados, &estado).expect("verifica").sincronizado);

    // JSON preexistente com colisão e JSON inválido em disco também devem gerar Conflito
    let json_cfg = dir.path().join("colisao.json");
    fs::write(&json_cfg, r#"{"tema":"escuro","mcpServers":{"sexta":"antigo"}}"#).expect("json previo");
    let json_desejado = vec![ArquivoDesejado::new(
        json_cfg.clone(),
        r#"{"mcpServers":{"sexta":"novo"}}"#.to_owned(),
        EstrategiaDeEscrita::MesclagemEstruturada {
            formato: FormatoEstruturado::Json,
        },
    )];
    let p_json_col = planejar(json_desejado.clone(), estado.clone()).expect("plano colisao json");
    assert!(matches!(p_json_col.acoes[0].operacao, OperacaoPlanejada::Conflito { .. }));
    aplicar(&p_json_col, true).expect("forca sobre colisao json");
    let json_final: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_cfg).expect("le json")).expect("valido");
    assert_eq!(json_final["tema"], "escuro");
    assert_eq!(json_final["mcpServers"]["sexta"], "novo");

    fs::write(&json_cfg, "{ quebrado").expect("json corrompido");
    let p_quebrado = planejar(json_desejado, estado).expect("plano json corrompido");
    assert!(matches!(p_quebrado.acoes[0].operacao, OperacaoPlanejada::Conflito { .. }));
}
```

- [ ] **Step 3: Rodar os testes para confirmar falha (`RED`)**

Run: `cargo test -p sexta-feira-core mesclagem_estruturada`
Expected: FAIL (`FormatoEstruturado`, `MesclagemEstruturada` inexistentes).

- [ ] **Step 4: Criar `crates/sexta-feira-core/src/estruturado.rs` e integrar em `plano.rs` + `lib.rs`**

- `estruturado.rs`:
  - Normalizar tanto JSON quanto TOML para uma representação estruturada ou operar sobre `serde_json::Value` / `toml::Value` com helpers comuns (deep-merge recursivo de mapas, projeção restrita às chaves presentes no patch desejado, comparação estrutural, detecção de colisão de chave folha preexistente, serialização canônica para checksum determinístico e serialização formatada com `\n` final).
  - Rejeitar patch cuja raiz não seja objeto/tabela (`ConteudoEstruturadoInvalido`).
  - Quando arquivo existente em disco estiver sintaticamente inválido (e não vazio), retornar `OperacaoPlanejada::Conflito` com `conteudo_final` = patch formatado, permitindo recuperação segura com `--forcar` + `.bak`.
- `plano.rs`:
  - Adicionar `FormatoEstruturado`, `EstrategiaDeEscrita::MesclagemEstruturada`, `ErroDePlano::ConteudoEstruturadoInvalido`, `arquivos_estruturados: BTreeMap<String, String>` em `EstadoAplicado`.
  - Registrar e recuperar checksum canônico do patch desejado em `aplicar` (inclusive quando `SemAlteracoes` sem entrada prévia no estado).
- `lib.rs`: reexportar `FormatoEstruturado`.

- [ ] **Step 5: Rodar toda a suíte de validação local (`GREEN`)**

Run:
```bash
cargo fmt --all --check && \
cargo clippy --workspace --all-targets --locked -- -D warnings && \
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked && \
cargo test --workspace --locked
```
Expected: PASS com zero warnings e todos os testes verdes.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock docs/stack.md crates/sexta-feira-core/Cargo.toml crates/sexta-feira-core/src/estruturado.rs crates/sexta-feira-core/src/plano.rs crates/sexta-feira-core/src/lib.rs
git commit -m "feat(core): adiciona mesclagem estruturada JSON e TOML com detecção de conflitos"
```
