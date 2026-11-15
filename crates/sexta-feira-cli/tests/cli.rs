//! Testes de ponta a ponta do binário `sexta-feira`.

#![allow(clippy::expect_used)]

use assert_cmd::Command;

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
