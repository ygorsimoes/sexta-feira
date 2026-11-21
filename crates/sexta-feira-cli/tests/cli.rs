//! Testes de ponta a ponta do binário `sexta-feira`.

#![allow(clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use assert_cmd::Command;

static CONTADOR_TMP: AtomicU64 = AtomicU64::new(0);

struct DirTemporario {
    caminho: PathBuf,
}

impl DirTemporario {
    fn novo(prefixo: &str) -> Self {
        let seq = CONTADOR_TMP.fetch_add(1, Ordering::Relaxed);
        let caminho = std::env::temp_dir().join(format!(
            "sexta-feira-cli-{prefixo}-{}-{seq}",
            std::process::id()
        ));
        fs::create_dir_all(&caminho).expect("deve criar diretório temporário");
        Self { caminho }
    }

    fn path(&self) -> &Path {
        &self.caminho
    }
}

impl Drop for DirTemporario {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.caminho);
    }
}

fn comando_cli() -> Command {
    let mut cmd = Command::cargo_bin("sexta-feira").expect("o binário deve existir");
    cmd.env(
        "XDG_CONFIG_HOME",
        std::env::temp_dir().join("sexta-feira-cli-xdg-inexistente"),
    )
    .env_remove("HERMES_HOME")
    .env_remove("SEXTA_FEIRA_LOG");
    cmd
}

fn saida(args: &[&str]) -> String {
    let saida = comando_cli()
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    String::from_utf8(saida).expect("a saída deve ser UTF-8")
}

fn normalizar_caminho(texto: &str, raiz: &Path) -> String {
    texto.replace(&raiz.display().to_string(), "<HERMES_HOME>")
}

#[test]
fn lista_os_harnesses() {
    insta::assert_snapshot!(saida(&["harnesses"]), @"
    hermes          disponível
    openclaw        em breve
    claude-code     em breve
    claude-desktop  em breve
    opencode        em breve
    ");
}

#[test]
fn plano_hermes_mostra_criacao_dos_arquivos_sem_escrever_no_disco() {
    let dir = DirTemporario::novo("plano");
    let hermes_home = dir.path().display().to_string();

    let out = saida(&["plano", "hermes", "--hermes-home", &hermes_home]);
    let normalizado = normalizar_caminho(&out, dir.path());

    assert!(normalizado.contains("[CRIAR] <HERMES_HOME>/profiles/sexta-feira/SOUL.md"));
    assert!(normalizado.contains("[CRIAR] <HERMES_HOME>/profiles/sexta-feira/memories/USER.md"));
    assert!(normalizado.contains("Resumo: 2 criar, 0 atualizar, 0 sem alterações, 0 conflitos"));
    assert!(!dir.path().join("profiles").exists());
}

#[test]
fn aplicar_e_verificar_hermes_sao_idempotentes() {
    let dir = DirTemporario::novo("idempotente");
    let hermes_home = dir.path().display().to_string();

    let out_aplicar = saida(&["aplicar", "hermes", "--hermes-home", &hermes_home]);
    assert!(out_aplicar.contains("2 criado(s), 0 atualizado(s), 0 backup(s)"));

    let out_verificar = saida(&["verificar", "hermes", "--hermes-home", &hermes_home]);
    let verif_norm = normalizar_caminho(&out_verificar, dir.path());
    assert!(verif_norm.contains("[OK] <HERMES_HOME>/profiles/sexta-feira/SOUL.md"));
    assert!(verif_norm.contains("[OK] <HERMES_HOME>/profiles/sexta-feira/memories/USER.md"));

    let out_plano = saida(&["plano", "hermes", "--hermes-home", &hermes_home]);
    assert!(out_plano.contains("Resumo: 0 criar, 0 atualizar, 2 sem alterações, 0 conflitos"));

    let soul = fs::read_to_string(dir.path().join("profiles/sexta-feira/SOUL.md"))
        .expect("SOUL.md deve existir");
    assert!(soul.contains("Sexta-feira"));
    assert!(soul.contains("J.A.R.V.I.S."));
    assert!(soul.contains("senhor"));

    let user = fs::read_to_string(dir.path().join("profiles/sexta-feira/memories/USER.md"))
        .expect("USER.md deve existir");
    assert!(user.contains("senhor"));
}

#[test]
fn aplicar_hermes_preserva_memorias_existentes_e_respeita_conflito_em_soul() {
    let dir = DirTemporario::novo("conflito");
    let hermes_home = dir.path().display().to_string();
    let perfil_dir = dir.path().join("profiles/sexta-feira");
    let soul_path = perfil_dir.join("SOUL.md");
    let user_path = perfil_dir.join("memories/USER.md");

    fs::create_dir_all(user_path.parent().expect("pai")).expect("cria memories");
    fs::write(&soul_path, "# Persona prévia\n").expect("grava SOUL prévio");
    fs::write(&user_path, "- Prefere café sem açúcar.\n").expect("grava USER prévio");

    let falha = comando_cli()
        .args(["aplicar", "hermes", "--hermes-home", &hermes_home])
        .assert()
        .failure();
    let stderr = String::from_utf8(falha.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr.contains("conflito"));
    assert!(!perfil_dir.join("SOUL.md.bak").exists());

    let out_forcar = saida(&[
        "aplicar",
        "hermes",
        "--hermes-home",
        &hermes_home,
        "--forcar",
    ]);
    assert!(out_forcar.contains("0 criado(s), 2 atualizado(s), 1 backup(s)"));
    assert_eq!(
        fs::read_to_string(perfil_dir.join("SOUL.md.bak")).expect("lê backup"),
        "# Persona prévia\n"
    );

    let user_atualizado = fs::read_to_string(&user_path).expect("lê USER atualizado");
    assert!(user_atualizado.contains("senhor"));
    assert!(user_atualizado.contains("- Prefere café sem açúcar."));
}

#[test]
fn verificar_hermes_falha_quando_nao_aplicado_ou_divergente() {
    let dir = DirTemporario::novo("verificar-falha");
    let hermes_home = dir.path().display().to_string();

    let verif_texto_falha = comando_cli()
        .args(["verificar", "hermes", "--hermes-home", &hermes_home])
        .assert()
        .failure();
    let stderr_verif =
        String::from_utf8(verif_texto_falha.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr_verif.contains("sexta-feira aplicar hermes"));

    let _ = saida(&["aplicar", "hermes", "--hermes-home", &hermes_home]);
    fs::write(
        dir.path().join("profiles/sexta-feira/SOUL.md"),
        "# Alterado fora do setup\n",
    )
    .expect("altera SOUL.md");

    let verif_texto_falha = comando_cli()
        .args(["verificar", "hermes", "--hermes-home", &hermes_home])
        .assert()
        .failure();
    let stderr_verif =
        String::from_utf8(verif_texto_falha.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr_verif.contains("sexta-feira aplicar hermes"));
}

#[test]
fn harness_nao_implementado_retorna_erro_claro() {
    let falha = comando_cli()
        .args([
            "plano",
            "openclaw",
            "--usuario",
            "/tmp/nao-existe-sexta-feira-usuario.toml",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8(falha.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr.contains("openclaw"));
    assert!(stderr.contains("ainda não implementado"));
}

#[test]
fn logs_tracing_respeitam_sexta_feira_log_e_nao_sujam_stderr_por_padrao() {
    let padrao = comando_cli().arg("harnesses").assert().success();
    let stderr_padrao = String::from_utf8(padrao.get_output().stderr.clone()).expect("UTF-8");
    assert_eq!(stderr_padrao.trim(), "");

    let debug = comando_cli()
        .env("SEXTA_FEIRA_LOG", "debug")
        .arg("harnesses")
        .assert()
        .success();
    let stderr_debug = String::from_utf8(debug.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr_debug.contains("DEBUG"));
}

#[test]
fn harnesses_com_json_retorna_lista_estruturada() {
    let out = saida(&["--json", "harnesses"]);
    let itens: serde_json::Value = serde_json::from_str(&out).expect("JSON válido");
    let lista = itens.as_array().expect("deve ser array");
    assert_eq!(lista.len(), 5);
    assert_eq!(lista[0]["id"], "hermes");
    assert_eq!(lista[0]["disponivel"], true);
    assert_eq!(lista[0]["status"], "disponível");
    assert_eq!(lista[1]["id"], "openclaw");
    assert_eq!(lista[1]["disponivel"], false);
    assert_eq!(lista[1]["status"], "em breve");
}

#[test]
fn destino_e_alias_hermes_home_funcionam_igualmente() {
    let dir = DirTemporario::novo("destino-alias");
    let destino = dir.path().display().to_string();

    let via_destino = saida(&["plano", "hermes", "--destino", &destino]);
    let via_alias = saida(&["plano", "hermes", "--hermes-home", &destino]);
    assert_eq!(via_destino, via_alias);
}

#[test]
fn personalizacao_via_persona_e_usuario_reflete_em_plano_e_falha_se_ausente() {
    let dir = DirTemporario::novo("personalizacao");
    let destino = dir.path().join("hermes");
    let persona = dir.path().join("persona.toml");
    let usuario = dir.path().join("usuario.toml");
    fs::write(&persona, "nome = \"Jarvis-BR\"\n").expect("grava persona");
    fs::write(&usuario, "tratamento = \"comandante\"\n").expect("grava usuario");

    let destino_str = destino.display().to_string();
    let persona_str = persona.display().to_string();
    let usuario_str = usuario.display().to_string();

    let out = saida(&[
        "aplicar",
        "hermes",
        "--destino",
        &destino_str,
        "--persona",
        &persona_str,
        "--usuario",
        &usuario_str,
    ]);
    assert!(out.contains("2 criado(s)"));

    let soul = fs::read_to_string(destino.join("profiles/sexta-feira/SOUL.md")).expect("SOUL");
    assert!(soul.contains("# Jarvis-BR"));
    assert!(soul.contains("comandante"));

    let user =
        fs::read_to_string(destino.join("profiles/sexta-feira/memories/USER.md")).expect("USER");
    assert!(user.contains("**Tratamento**: comandante"));

    let ausente = comando_cli()
        .args([
            "plano",
            "hermes",
            "--destino",
            &destino_str,
            "--usuario",
            "/tmp/nao-existe-sexta-feira-usuario.toml",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8(ausente.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr.contains("não encontrado"));
}

#[test]
fn plano_aplicar_e_verificar_em_json_cobrem_sucesso_pendencia_e_remover() {
    let dir = DirTemporario::novo("json-e2e");
    let destino = dir.path().display().to_string();
    let perfil_dir = dir.path().join("profiles/sexta-feira");

    let verif_pendente = comando_cli()
        .args(["--json", "verificar", "hermes", "--destino", &destino])
        .assert()
        .failure();
    let verif_json: serde_json::Value =
        serde_json::from_slice(&verif_pendente.get_output().stdout).expect("JSON em stdout");
    assert_eq!(verif_json["harness"], "hermes");
    assert_eq!(verif_json["sincronizado"], false);
    assert_eq!(verif_json["itens"].as_array().expect("itens").len(), 2);

    let plano_out = saida(&["--json", "plano", "hermes", "--destino", &destino]);
    let plano_json: serde_json::Value = serde_json::from_str(&plano_out).expect("JSON plano");
    assert_eq!(plano_json["resumo"]["criar"], 2);
    assert_eq!(plano_json["resumo"]["remover"], 0);
    assert_eq!(plano_json["acoes"][0]["operacao"], "criar");

    let aplicar_out = saida(&["--json", "aplicar", "hermes", "--destino", &destino]);
    let aplicar_json: serde_json::Value = serde_json::from_str(&aplicar_out).expect("JSON aplicar");
    assert_eq!(aplicar_json["harness"], "hermes");
    assert_eq!(aplicar_json["criados"], 2);
    assert_eq!(aplicar_json["atualizados"], 0);

    // Simula arquivo órfão registrado em `[arquivos_integrais]` para validar remoção no resumo e no JSON
    let obsoleto = perfil_dir.join("OBSOLETO.md");
    fs::write(&obsoleto, "").expect("grava obsoleto vazio");
    let estado_path = perfil_dir.join(".sexta-feira-estado.toml");
    let estado_atual = fs::read_to_string(&estado_path).expect("lê estado");
    let estado_com_orfao = estado_atual.replacen(
        "[arquivos_integrais]\n",
        "[arquivos_integrais]\n\"OBSOLETO.md\" = \"fnv1a64:cbf29ce484222325:0\"\n",
        1,
    );
    fs::write(&estado_path, estado_com_orfao).expect("atualiza estado com órfão");

    let plano_rem_txt = saida(&["plano", "hermes", "--destino", &destino]);
    assert!(plano_rem_txt.contains("[REMOVER]"));
    assert!(plano_rem_txt.contains("1 remover"));

    let plano_rem_json: serde_json::Value = serde_json::from_str(&saida(&[
        "--json",
        "plano",
        "hermes",
        "--destino",
        &destino,
    ]))
    .expect("JSON remoção");
    assert_eq!(plano_rem_json["resumo"]["remover"], 1);
}

#[test]
fn verbose_emite_contexto_rico_em_stderr_sem_poluir_json_em_stdout() {
    let dir = DirTemporario::novo("verbose-json");
    let destino = dir.path().join("hermes");
    let destino_str = destino.display().to_string();

    let saida_cmd = comando_cli()
        .args([
            "--json",
            "-vv",
            "plano",
            "hermes",
            "--destino",
            &destino_str,
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(saida_cmd.get_output().stdout.clone()).expect("UTF-8 stdout");
    let stderr = String::from_utf8(saida_cmd.get_output().stderr.clone()).expect("UTF-8 stderr");

    let json: serde_json::Value = serde_json::from_str(&stdout).expect("stdout deve ser JSON puro");
    assert_eq!(json["harness"], "hermes");

    assert!(stderr.contains("[verbose]"));
    assert!(stderr.contains("camadas"));
    assert!(stderr.contains("destino"));
    assert!(stderr.contains("ArquivoIntegral"));
    assert!(stderr.contains("BlocoDelimitado"));
    assert!(stderr.contains("SOUL.md"));
    assert!(stderr.contains("linhas"));
    assert!(stderr.contains("XDG_CONFIG_HOME"));
}

#[test]
fn sexta_feira_log_mantem_precedencia_sobre_verbose() {
    let com_env = comando_cli()
        .env("SEXTA_FEIRA_LOG", "warn")
        .args(["-vv", "harnesses"])
        .assert()
        .success();
    let stderr_env = String::from_utf8(com_env.get_output().stderr.clone()).expect("UTF-8");
    assert!(!stderr_env.contains("DEBUG"));
    assert!(stderr_env.contains("[verbose]"));

    let sem_env = comando_cli().args(["-vv", "harnesses"]).assert().success();
    let stderr_sem_env = String::from_utf8(sem_env.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr_sem_env.contains("DEBUG"));
}

#[test]
fn plano_aplicar_e_erros_exibem_dicas_acionaveis() {
    let dir = DirTemporario::novo("dicas-cli");
    let destino = dir.path().join("hermes");
    let destino_str = destino.display().to_string();

    let out_plano = saida(&["plano", "hermes", "--destino", &destino_str]);
    assert!(out_plano.contains("profiles/sexta-feira"));
    assert!(out_plano.contains("sexta-feira aplicar hermes"));

    let out_aplicar = saida(&["aplicar", "hermes", "--destino", &destino_str]);
    assert!(out_aplicar.contains("profiles/sexta-feira"));
    assert!(out_aplicar.contains("sexta-feira verificar hermes"));

    let erro_harness = comando_cli().args(["plano", "openclaw"]).assert().failure();
    let stderr_harness =
        String::from_utf8(erro_harness.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr_harness.contains("sexta-feira harnesses"));
    assert!(stderr_harness.contains("SKILL.md"));

    fs::create_dir_all(destino.join("profiles/sexta-feira")).expect("cria perfil");
    fs::write(destino.join("profiles/sexta-feira/SOUL.md"), "# Manual\n").expect("grava SOUL");
    let erro_conflito = comando_cli()
        .args(["aplicar", "hermes", "--destino", &destino_str])
        .assert()
        .failure();
    let stderr_conflito =
        String::from_utf8(erro_conflito.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr_conflito.contains("--forcar"));
    assert!(stderr_conflito.contains("plano"));

    let toml_errado = dir.path().join("usuario-errado.toml");
    fs::write(&toml_errado, "chave_errada = 1\n").expect("grava toml errado");
    let erro_toml = comando_cli()
        .args([
            "plano",
            "hermes",
            "--destino",
            &destino_str,
            "--usuario",
            &toml_errado.display().to_string(),
        ])
        .assert()
        .failure();
    let stderr_toml = String::from_utf8(erro_toml.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr_toml.contains("tratamento"));
    assert!(stderr_toml.contains("diagnostico"));
}

#[test]
fn diagnostico_em_json_e_texto_cobre_ambiente_config_invalida_e_harnesses() {
    let dir = DirTemporario::novo("diagnostico");
    let home = dir.path().join("home");
    let xdg = dir.path().join("xdg");
    let destino = dir.path().join("hermes");
    fs::create_dir_all(home.join(".claude")).expect("cria .claude");
    fs::create_dir_all(xdg.join("sexta-feira")).expect("cria xdg");

    let home_str = home.display().to_string();
    let xdg_str = xdg.display().to_string();
    let destino_str = destino.display().to_string();

    // 1. Setup ainda não aplicado -> exit 0, JSON estruturado completo
    let diag_inicial = comando_cli()
        .env("HOME", &home_str)
        .env("XDG_CONFIG_HOME", &xdg_str)
        .args(["--json", "diagnostico", "--destino", &destino_str])
        .assert()
        .success();
    let json_inicial: serde_json::Value =
        serde_json::from_slice(&diag_inicial.get_output().stdout).expect("JSON diagnóstico");
    assert!(json_inicial["versao"].is_string());
    assert!(json_inicial["plataforma"].is_string());
    assert!(json_inicial["configuracao"]["diretorio"].is_string());
    assert_eq!(json_inicial["definicao"]["valida"], true);
    assert_eq!(json_inicial["harnesses"][0]["id"], "hermes");
    assert_eq!(json_inicial["harnesses"][0]["sincronizado"], false);
    assert!(
        json_inicial["proximo_passo"]
            .as_str()
            .expect("proximo_passo")
            .contains("aplicar")
    );

    // 2. usuario.toml inválido -> exit 0, definicao.valida == false, continua inspecionando
    fs::write(xdg.join("sexta-feira/usuario.toml"), "chave_invalida = 1\n")
        .expect("grava toml inválido");
    let diag_invalido_json = comando_cli()
        .env("HOME", &home_str)
        .env("XDG_CONFIG_HOME", &xdg_str)
        .args(["--json", "diagnostico", "--destino", &destino_str])
        .assert()
        .success();
    let json_inv: serde_json::Value =
        serde_json::from_slice(&diag_invalido_json.get_output().stdout).expect("JSON inválido");
    assert_eq!(json_inv["definicao"]["valida"], false);
    assert!(json_inv["definicao"]["erro"].as_str().is_some());

    let diag_invalido_txt = comando_cli()
        .env("HOME", &home_str)
        .env("XDG_CONFIG_HOME", &xdg_str)
        .args(["diagnostico", "--destino", &destino_str])
        .assert()
        .success();
    let txt_inv = String::from_utf8(diag_invalido_txt.get_output().stdout.clone()).expect("UTF-8");
    assert!(txt_inv.contains("inválida"));
    assert!(txt_inv.contains("hermes"));
    assert!(!txt_inv.contains("destino indisponível"));
    assert!(txt_inv.contains("não verificado"));

    // 3. Corrige config, aplica no Hermes e filtra `diagnostico hermes`
    fs::write(
        xdg.join("sexta-feira/usuario.toml"),
        "tratamento = \"comandante\"\n",
    )
    .expect("grava usuario válido");
    let _ = comando_cli()
        .env("HOME", &home_str)
        .env("XDG_CONFIG_HOME", &xdg_str)
        .args(["aplicar", "hermes", "--destino", &destino_str])
        .assert()
        .success();

    let diag_hermes = comando_cli()
        .env("HOME", &home_str)
        .env("XDG_CONFIG_HOME", &xdg_str)
        .args(["--json", "diagnostico", "hermes", "--destino", &destino_str])
        .assert()
        .success();
    let json_hermes: serde_json::Value =
        serde_json::from_slice(&diag_hermes.get_output().stdout).expect("JSON hermes");
    assert_eq!(json_hermes["harnesses"].as_array().expect("arr").len(), 1);
    assert_eq!(json_hermes["harnesses"][0]["id"], "hermes");
    assert_eq!(json_hermes["harnesses"][0]["sincronizado"], true);
    assert!(
        json_hermes["proximo_passo"]
            .as_str()
            .expect("proximo_passo")
            .contains("sincronizado")
    );
}

#[test]
fn iniciar_sem_tty_falha_sem_nao_interativo() {
    let dir = DirTemporario::novo("iniciar-sem-tty");
    let falha = comando_cli()
        .args(["iniciar", "--config-dir", &dir.path().display().to_string()])
        .assert()
        .failure();
    let stderr = String::from_utf8(falha.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr.contains("--nao-interativo"));
}

#[test]
fn iniciar_nao_interativo_cria_mantem_mescla_com_backup_e_rejeita_invalido() {
    let dir = DirTemporario::novo("iniciar-ciclo");
    let config_dir = dir.path().join("cfg");
    let config_str = config_dir.display().to_string();
    let usuario_toml = config_dir.join("usuario.toml");

    // 1. Cria usuario.toml novo
    let out_criar = comando_cli()
        .args([
            "--json",
            "iniciar",
            "--config-dir",
            &config_str,
            "--tratamento",
            "comandante",
            "--nao-interativo",
        ])
        .assert()
        .success();
    let json_criar: serde_json::Value =
        serde_json::from_slice(&out_criar.get_output().stdout).expect("JSON criar");
    assert_eq!(json_criar["acao"], "criado");
    assert!(usuario_toml.exists());

    // 2. Idempotente sem alterações -> mantido sem backup
    let out_mantido = comando_cli()
        .args([
            "--json",
            "iniciar",
            "--config-dir",
            &config_str,
            "--nao-interativo",
        ])
        .assert()
        .success();
    let json_mantido: serde_json::Value =
        serde_json::from_slice(&out_mantido.get_output().stdout).expect("JSON mantido");
    assert_eq!(json_mantido["acao"], "mantido");
    assert!(!config_dir.join("usuario.toml.bak").exists());

    // 3. Alteração sem --sobrescrever falha
    let falha_sobrescrever = comando_cli()
        .args([
            "iniciar",
            "--config-dir",
            &config_str,
            "--tratamento",
            "chefe",
            "--nao-interativo",
        ])
        .assert()
        .failure();
    let stderr_sob =
        String::from_utf8(falha_sobrescrever.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr_sob.contains("--sobrescrever"));

    // 4. Patch com --sobrescrever preserva tratamento existente e cria .bak
    let out_patch = comando_cli()
        .args([
            "--json",
            "iniciar",
            "--config-dir",
            &config_str,
            "--preferencia",
            "Direto ao ponto",
            "--sobrescrever",
            "--nao-interativo",
        ])
        .assert()
        .success();
    let json_patch: serde_json::Value =
        serde_json::from_slice(&out_patch.get_output().stdout).expect("JSON patch");
    assert_eq!(json_patch["acao"], "atualizado");
    assert!(config_dir.join("usuario.toml.bak").exists());

    let conteudo_atual = fs::read_to_string(&usuario_toml).expect("lê usuario.toml");
    assert!(conteudo_atual.contains("comandante"));
    assert!(conteudo_atual.contains("Direto ao ponto"));
}

#[test]
fn iniciar_rejeita_valor_vazio_e_regenera_corrompido_com_backup() {
    let dir = DirTemporario::novo("iniciar-corrompido");
    let config_dir = dir.path().join("cfg");
    let config_str = config_dir.display().to_string();
    let usuario_toml = config_dir.join("usuario.toml");

    fs::create_dir_all(&config_dir).expect("cria dir");
    fs::write(&usuario_toml, "tratamento = \"comandante\"\n").expect("grava valido");

    // 1. Valor vazio é rejeitado antes de gravar ou sobrescrever
    let falha_vazio = comando_cli()
        .args([
            "iniciar",
            "--config-dir",
            &config_str,
            "--tratamento",
            "   ",
            "--sobrescrever",
            "--nao-interativo",
        ])
        .assert()
        .failure();
    let stderr_vazio = String::from_utf8(falha_vazio.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr_vazio.contains("vazio"));
    assert!(!config_dir.join("usuario.toml.bak").exists());
    let conteudo_preservado = fs::read_to_string(&usuario_toml).expect("preservado");
    assert!(conteudo_preservado.contains("comandante"));

    // 2. Arquivo corrompido exige --sobrescrever e regenera com .bak
    fs::write(&usuario_toml, "chave_errada = 1\n").expect("corrompe arquivo");
    let _ = comando_cli()
        .args(["iniciar", "--config-dir", &config_str, "--nao-interativo"])
        .assert()
        .failure();

    let out_regen = comando_cli()
        .args([
            "--json",
            "iniciar",
            "--config-dir",
            &config_str,
            "--tratamento",
            "senhor",
            "--sobrescrever",
            "--nao-interativo",
        ])
        .assert()
        .success();
    let json_regen: serde_json::Value =
        serde_json::from_slice(&out_regen.get_output().stdout).expect("JSON regen");
    assert_eq!(json_regen["acao"], "atualizado");
    assert!(config_dir.join("usuario.toml.bak").exists());
}

#[test]
fn iniciar_nao_interativo_pode_aplicar_no_hermes_e_emitir_json() {
    let dir = DirTemporario::novo("iniciar-hermes");
    let xdg = dir.path().join("xdg");
    let destino = dir.path().join("hermes");

    let out = comando_cli()
        .env("XDG_CONFIG_HOME", &xdg)
        .args([
            "--json",
            "iniciar",
            "--tratamento",
            "almirante",
            "--harness",
            "hermes",
            "--destino",
            &destino.display().to_string(),
            "--aplicar",
            "--nao-interativo",
        ])
        .assert()
        .success();

    let json: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).expect("JSON");
    assert_eq!(json["acao"], "criado");
    assert_eq!(json["aplicado"]["criados"], 2);

    let soul = fs::read_to_string(destino.join("profiles/sexta-feira/SOUL.md")).expect("SOUL");
    assert!(soul.contains("almirante"));
}

#[test]
fn iniciar_recupera_arquivo_semanticamente_invalido_rotaciona_bak_e_rejeita_preferencia_vazia() {
    let dir = DirTemporario::novo("iniciar-semantico");
    let config_dir = dir.path().join("cfg");
    let config_str = config_dir.display().to_string();
    let usuario_toml = config_dir.join("usuario.toml");
    fs::create_dir_all(&config_dir).expect("cria dir");

    // 1. Rejeita --preferencia "" antes de escrever
    let falha_pref = comando_cli()
        .args([
            "iniciar",
            "--config-dir",
            &config_str,
            "--preferencia",
            "",
            "--nao-interativo",
        ])
        .assert()
        .failure();
    assert!(!usuario_toml.exists());
    let stderr_pref = String::from_utf8(falha_pref.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr_pref.contains("preferencias"));

    // 2. Arquivo com chaves válidas mas valores semanticamente vazios
    fs::write(&usuario_toml, "tratamento = \"\"\npreferencias = []\n").expect("grava vazio");
    let falha_sem_sob = comando_cli()
        .args(["iniciar", "--config-dir", &config_str, "--nao-interativo"])
        .assert()
        .failure();
    let stderr_sem_sob =
        String::from_utf8(falha_sem_sob.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr_sem_sob.contains("--sobrescrever"));

    let out_regen1 = comando_cli()
        .args([
            "--json",
            "iniciar",
            "--config-dir",
            &config_str,
            "--tratamento",
            "comandante",
            "--sobrescrever",
            "--nao-interativo",
        ])
        .assert()
        .success();
    let j1: serde_json::Value =
        serde_json::from_slice(&out_regen1.get_output().stdout).expect("JSON 1");
    assert_eq!(j1["acao"], "atualizado");
    assert!(config_dir.join("usuario.toml.bak").exists());

    // 3. Segunda sobrescrita gera backup numerado .bak.1
    let out_regen2 = comando_cli()
        .args([
            "--json",
            "iniciar",
            "--config-dir",
            &config_str,
            "--tratamento",
            "capitão",
            "--sobrescrever",
            "--nao-interativo",
        ])
        .assert()
        .success();
    let j2: serde_json::Value =
        serde_json::from_slice(&out_regen2.get_output().stdout).expect("JSON 2");
    assert_eq!(j2["acao"], "atualizado");
    assert!(config_dir.join("usuario.toml.bak.1").exists());
}

#[test]
fn diagnostico_reporta_erro_de_perfil_ou_estado_sem_dizer_destino_indisponivel() {
    let dir = DirTemporario::novo("diag-erro-estado");
    let destino = dir.path().join("hermes");
    let destino_str = destino.display().to_string();

    // 1. Perfil inválido no argumento --perfil
    let diag_perfil = comando_cli()
        .args([
            "diagnostico",
            "hermes",
            "--destino",
            &destino_str,
            "--perfil",
            "../invalido",
        ])
        .assert()
        .success();
    let txt_perfil = String::from_utf8(diag_perfil.get_output().stdout.clone()).expect("UTF-8");
    assert!(!txt_perfil.contains("destino indisponível"));
    assert!(txt_perfil.contains("erro"));

    // 2. Estado corrompido no Hermes
    let perfil_dir = destino.join("profiles/sexta-feira");
    fs::create_dir_all(&perfil_dir).expect("cria perfil");
    fs::write(
        perfil_dir.join(".sexta-feira-estado.toml"),
        "versao = 999\n",
    )
    .expect("grava estado inválido");

    let diag_estado = comando_cli()
        .args(["--json", "diagnostico", "hermes", "--destino", &destino_str])
        .assert()
        .success();
    let json_estado: serde_json::Value =
        serde_json::from_slice(&diag_estado.get_output().stdout).expect("JSON estado");
    assert!(json_estado["harnesses"][0]["erro"].as_str().is_some());
    assert!(
        !json_estado["proximo_passo"]
            .as_str()
            .expect("proximo_passo")
            .contains("Informe `--destino")
    );
}

#[test]
fn json_com_vvv_e_trace_nao_contamina_stdout_em_nenhum_subcomando() {
    let dir = DirTemporario::novo("json-vvv-todos");
    let xdg = dir.path().join("xdg");
    let destino = dir.path().join("hermes");
    let destino_str = destino.display().to_string();

    let casos: Vec<Vec<&str>> = vec![
        vec!["--json", "-vvv", "harnesses"],
        vec!["--json", "-vvv", "diagnostico", "--destino", &destino_str],
        vec![
            "--json",
            "-vvv",
            "iniciar",
            "--tratamento",
            "senhor",
            "--nao-interativo",
        ],
        vec![
            "--json",
            "-vvv",
            "plano",
            "hermes",
            "--destino",
            &destino_str,
        ],
        vec![
            "--json",
            "-vvv",
            "aplicar",
            "hermes",
            "--destino",
            &destino_str,
        ],
        vec![
            "--json",
            "-vvv",
            "verificar",
            "hermes",
            "--destino",
            &destino_str,
        ],
    ];

    for args in casos {
        let res = comando_cli()
            .env("XDG_CONFIG_HOME", &xdg)
            .env("SEXTA_FEIRA_LOG", "trace")
            .args(&args)
            .assert()
            .success();
        let stdout = String::from_utf8(res.get_output().stdout.clone()).expect("UTF-8");
        let parsed: serde_json::Value =
            serde_json::from_str(&stdout).expect("stdout deve ser JSON puro com -vvv/trace");
        assert!(!parsed.is_null());
    }

    // Caso de falha controlada em verificar com --json -vvv
    fs::write(
        destino.join("profiles/sexta-feira/SOUL.md"),
        "# Divergente\n",
    )
    .expect("altera SOUL");
    let verif_falha = comando_cli()
        .env("XDG_CONFIG_HOME", &xdg)
        .args([
            "--json",
            "-vvv",
            "verificar",
            "hermes",
            "--destino",
            &destino_str,
        ])
        .assert()
        .failure();
    let stdout_falha =
        String::from_utf8(verif_falha.get_output().stdout.clone()).expect("UTF-8 falha");
    let json_falha: serde_json::Value =
        serde_json::from_str(&stdout_falha).expect("stdout de verificar falha deve ser JSON puro");
    assert_eq!(json_falha["sincronizado"], false);
}

#[test]
fn iniciar_valida_harness_antes_de_gravar_exibe_dica_conflito_e_emite_destino_verbose() {
    let dir = DirTemporario::novo("iniciar-melhorias");
    let config_dir = dir.path().join("cfg");
    let destino = dir.path().join("hermes");

    // 1. Harness sem adapter falha antes de criar usuario.toml
    let _ = comando_cli()
        .args([
            "iniciar",
            "--config-dir",
            &config_dir.display().to_string(),
            "--harness",
            "openclaw",
            "--nao-interativo",
        ])
        .assert()
        .failure();
    assert!(!config_dir.join("usuario.toml").exists());

    // 2. Conflito no destino + -v emite destino em stderr e dica com --forcar
    fs::create_dir_all(destino.join("profiles/sexta-feira")).expect("cria perfil");
    fs::write(destino.join("profiles/sexta-feira/SOUL.md"), "# Manual\n").expect("grava SOUL");
    let ok_conflito = comando_cli()
        .args([
            "-v",
            "iniciar",
            "--config-dir",
            &config_dir.display().to_string(),
            "--harness",
            "hermes",
            "--destino",
            &destino.display().to_string(),
            "--nao-interativo",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(ok_conflito.get_output().stdout.clone()).expect("UTF-8");
    let stderr = String::from_utf8(ok_conflito.get_output().stderr.clone()).expect("UTF-8");
    assert!(stdout.contains("--forcar"));
    assert!(stderr.contains("[verbose] destino:"));
}

#[test]
fn diagnostico_diferencia_erro_de_persona_e_verificar_ordena_erro_antes_da_dica() {
    let dir = DirTemporario::novo("diag-persona-ordem");
    let xdg = dir.path().join("xdg");
    let destino = dir.path().join("hermes");
    fs::create_dir_all(xdg.join("sexta-feira")).expect("cria xdg");
    fs::write(xdg.join("sexta-feira/persona.toml"), "nome = \"\"\n").expect("persona vazia");

    let diag = comando_cli()
        .env("XDG_CONFIG_HOME", &xdg)
        .args([
            "--json",
            "diagnostico",
            "--destino",
            &destino.display().to_string(),
        ])
        .assert()
        .success();
    let json: serde_json::Value = serde_json::from_slice(&diag.get_output().stdout).expect("JSON");
    let proximo = json["proximo_passo"].as_str().expect("proximo_passo");
    assert!(proximo.contains("persona.toml"));
    assert!(!proximo.contains("iniciar --sobrescrever"));

    let verif = comando_cli()
        .args([
            "verificar",
            "hermes",
            "--destino",
            &destino.display().to_string(),
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8(verif.get_output().stderr.clone()).expect("UTF-8");
    let pos_erro = stderr.find("Erro:").expect("Erro: presente");
    let pos_dica = stderr.find("Dica:").expect("Dica: presente");
    assert!(
        pos_erro < pos_dica,
        "Erro: deve vir antes de Dica:\n{stderr}"
    );
}
