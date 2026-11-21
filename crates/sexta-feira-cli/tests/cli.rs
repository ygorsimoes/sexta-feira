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
