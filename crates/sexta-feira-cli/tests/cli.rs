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

fn saida(args: &[&str]) -> String {
    let saida = Command::cargo_bin("sexta-feira")
        .expect("o binário deve existir")
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
    hermes
    openclaw
    claude-code
    claude-desktop
    opencode
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

    let falha = Command::cargo_bin("sexta-feira")
        .expect("o binário deve existir")
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

    Command::cargo_bin("sexta-feira")
        .expect("o binário deve existir")
        .args(["verificar", "hermes", "--hermes-home", &hermes_home])
        .assert()
        .failure();

    let _ = saida(&["aplicar", "hermes", "--hermes-home", &hermes_home]);
    fs::write(
        dir.path().join("profiles/sexta-feira/SOUL.md"),
        "# Alterado fora do setup\n",
    )
    .expect("altera SOUL.md");

    Command::cargo_bin("sexta-feira")
        .expect("o binário deve existir")
        .args(["verificar", "hermes", "--hermes-home", &hermes_home])
        .assert()
        .failure();
}

#[test]
fn harness_nao_implementado_retorna_erro_claro() {
    let falha = Command::cargo_bin("sexta-feira")
        .expect("o binário deve existir")
        .args(["plano", "openclaw"])
        .assert()
        .failure();
    let stderr = String::from_utf8(falha.get_output().stderr.clone()).expect("UTF-8");
    assert!(stderr.contains("openclaw"));
    assert!(stderr.contains("ainda não implementado"));
}
