//! Catálogo dos harnesses suportados ou previstos pela Sexta-feira.

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sexta_feira_core::AdaptadorDeSetup;

use crate::hermes::{AdaptadorHermes, ConfiguracaoHermes};

/// Um harness de agente em que a Sexta-feira pode ser instalada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Harness {
    /// Hermes Agent, da Nous Research.
    Hermes,
    /// `OpenClaw`.
    Openclaw,
    /// Claude Code.
    ClaudeCode,
    /// Claude Desktop e claude.ai.
    ClaudeDesktop,
    /// `OpenCode`.
    Opencode,
}

impl Harness {
    /// Todos os harnesses previstos, na ordem em que serão suportados.
    pub const TODOS: [Self; 5] = [
        Self::Hermes,
        Self::Openclaw,
        Self::ClaudeCode,
        Self::ClaudeDesktop,
        Self::Opencode,
    ];

    /// Identificador usado na linha de comando e nos arquivos de configuração.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Hermes => "hermes",
            Self::Openclaw => "openclaw",
            Self::ClaudeCode => "claude-code",
            Self::ClaudeDesktop => "claude-desktop",
            Self::Opencode => "opencode",
        }
    }

    /// Indica se este harness já possui adaptador implementado.
    #[must_use]
    pub const fn disponivel(self) -> bool {
        matches!(self, Self::Hermes)
    }

    /// Rótulo curto de disponibilidade exibido pela CLI.
    #[must_use]
    pub const fn rotulo_status(self) -> &'static str {
        if self.disponivel() {
            "disponível"
        } else {
            "em breve"
        }
    }
}

impl fmt::Display for Harness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// Erro devolvido quando um identificador não corresponde a nenhum harness.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("harness desconhecido: `{0}`")]
pub struct HarnessDesconhecido(pub String);

impl FromStr for Harness {
    type Err = HarnessDesconhecido;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::TODOS
            .into_iter()
            .find(|h| h.id() == s)
            .ok_or_else(|| HarnessDesconhecido(s.to_owned()))
    }
}

/// Origem usada para resolver o diretório base de um harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrigemResolucaoDestino {
    /// Caminho passado explicitamente pela flag `--destino` (ou `--hermes-home`).
    Explicito,
    /// Variável de ambiente específica do harness (ex.: `HERMES_HOME`).
    VariavelAmbiente,
    /// Caminho padrão derivado de `HOME`.
    PadraoHome,
    /// Não foi possível resolver um caminho para este harness.
    Indisponivel,
}

/// Caminho inspecionado no sistema de arquivos com indicador de existência.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaminhoDiagnosticado {
    /// Caminho resolvido para inspeção.
    pub caminho: PathBuf,
    /// Indica se o caminho existe em disco no momento da inspeção.
    pub existe: bool,
}

impl CaminhoDiagnosticado {
    fn de_path(caminho: PathBuf) -> Self {
        let existe = caminho.exists();
        Self { caminho, existe }
    }
}

/// Diagnóstico ambiental consolidado para um harness do catálogo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticoHarness {
    /// Identificador do harness.
    pub harness: Harness,
    /// Indica se há adapter implementado na CLI.
    pub disponivel: bool,
    /// Rótulo curto de status (`"disponível"` ou `"em breve"`).
    pub status: &'static str,
    /// Origem da resolução do diretório base.
    pub origem_destino: OrigemResolucaoDestino,
    /// Diretório base resolvido para o harness, quando aplicável.
    pub destino_base: Option<CaminhoDiagnosticado>,
    /// Diretório do perfil alvo gerenciado pela Sexta-feira.
    pub diretorio_perfil: Option<CaminhoDiagnosticado>,
    /// Caminho esperado do arquivo de estado gerenciado.
    pub arquivo_estado: Option<CaminhoDiagnosticado>,
    /// Erro ocorrido ao tentar resolver o destino ou perfil deste harness, quando houver.
    pub erro_destino: Option<String>,
    /// Caminhos convencionais detectados em disco (para harnesses ainda sem adapter).
    pub caminhos_indicativos: Vec<PathBuf>,
}

/// Inspeciona os harnesses do catálogo (ou apenas `filtro`) frente ao ambiente informado.
#[must_use]
pub fn inspecionar_ambiente_harnesses(
    filtro: Option<Harness>,
    destino_explicito: Option<&Path>,
    perfil: Option<&str>,
    env_hermes_home: Option<&str>,
    env_home: Option<&str>,
) -> Vec<DiagnosticoHarness> {
    let alvos: Vec<Harness> = match filtro {
        Some(h) => vec![h],
        None => Harness::TODOS.to_vec(),
    };

    alvos
        .into_iter()
        .map(|harness| match harness {
            Harness::Hermes => {
                inspecionar_hermes(destino_explicito, perfil, env_hermes_home, env_home)
            }
            outro => inspecionar_em_breve(outro, env_home),
        })
        .collect()
}

fn inspecionar_hermes(
    destino_explicito: Option<&Path>,
    perfil: Option<&str>,
    env_hermes_home: Option<&str>,
    env_home: Option<&str>,
) -> DiagnosticoHarness {
    match ConfiguracaoHermes::resolver_com_origem(
        destino_explicito,
        env_hermes_home,
        env_home,
        perfil,
    ) {
        Ok((config, origem_destino)) => {
            let destino_base = CaminhoDiagnosticado::de_path(config.hermes_home.clone());
            let diretorio_perfil = CaminhoDiagnosticado::de_path(config.diretorio_do_perfil());
            let arquivo_estado =
                CaminhoDiagnosticado::de_path(AdaptadorHermes::new(config).arquivo_estado());
            DiagnosticoHarness {
                harness: Harness::Hermes,
                disponivel: true,
                status: Harness::Hermes.rotulo_status(),
                origem_destino,
                destino_base: Some(destino_base),
                diretorio_perfil: Some(diretorio_perfil),
                arquivo_estado: Some(arquivo_estado),
                erro_destino: None,
                caminhos_indicativos: Vec::new(),
            }
        }
        Err(erro) => DiagnosticoHarness {
            harness: Harness::Hermes,
            disponivel: true,
            status: Harness::Hermes.rotulo_status(),
            origem_destino: OrigemResolucaoDestino::Indisponivel,
            destino_base: None,
            diretorio_perfil: None,
            arquivo_estado: None,
            erro_destino: Some(erro.to_string()),
            caminhos_indicativos: Vec::new(),
        },
    }
}

fn inspecionar_em_breve(harness: Harness, env_home: Option<&str>) -> DiagnosticoHarness {
    let mut caminhos_indicativos = Vec::new();
    if let Some(home) = env_home.map(str::trim).filter(|s| !s.is_empty()) {
        let base = PathBuf::from(home);
        let candidato = match harness {
            Harness::Openclaw => Some(base.join(".openclaw")),
            Harness::ClaudeCode => Some(base.join(".claude")),
            Harness::ClaudeDesktop => {
                if cfg!(target_os = "macos") {
                    Some(base.join("Library/Application Support/Claude"))
                } else {
                    Some(base.join(".config/Claude"))
                }
            }
            Harness::Opencode => Some(base.join(".config/opencode")),
            Harness::Hermes => None,
        };
        if let Some(caminho) = candidato
            && caminho.exists()
        {
            caminhos_indicativos.push(caminho);
        }
    }

    DiagnosticoHarness {
        harness,
        disponivel: false,
        status: harness.rotulo_status(),
        origem_destino: OrigemResolucaoDestino::Indisponivel,
        destino_base: None,
        diretorio_perfil: None,
        arquivo_estado: None,
        erro_destino: None,
        caminhos_indicativos,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn id_e_from_str_sao_inversos() {
        for harness in Harness::TODOS {
            assert_eq!(harness.id().parse::<Harness>(), Ok(harness));
        }
    }

    #[test]
    fn identificador_invalido_e_rejeitado() {
        assert_eq!(
            "jarvis".parse::<Harness>(),
            Err(HarnessDesconhecido("jarvis".to_owned()))
        );
    }

    #[test]
    fn hermes_esta_disponivel_e_demais_harnesses_em_breve() {
        assert!(Harness::Hermes.disponivel());
        assert_eq!(Harness::Hermes.rotulo_status(), "disponível");

        for outro in [
            Harness::Openclaw,
            Harness::ClaudeCode,
            Harness::ClaudeDesktop,
            Harness::Opencode,
        ] {
            assert!(!outro.disponivel());
            assert_eq!(outro.rotulo_status(), "em breve");
        }
    }

    #[test]
    fn inspecionar_ambiente_harnesses_detecta_hermes_e_caminhos_indicativos() {
        let base =
            std::env::temp_dir().join(format!("sexta-feira-adapters-diag-{}", std::process::id()));
        let hermes_perfil = base.join(".hermes/profiles/sexta-feira");
        let claude_dir = base.join(".claude");
        std::fs::create_dir_all(&hermes_perfil).expect("cria perfil hermes");
        std::fs::write(
            hermes_perfil.join(".sexta-feira-estado.toml"),
            "versao = 1\n",
        )
        .expect("cria estado");
        std::fs::create_dir_all(&claude_dir).expect("cria claude");

        let home_str = base.display().to_string();
        let diags = inspecionar_ambiente_harnesses(None, None, None, None, Some(&home_str));
        assert_eq!(diags.len(), Harness::TODOS.len());

        let hermes = &diags[0];
        assert_eq!(hermes.harness, Harness::Hermes);
        assert!(hermes.disponivel);
        assert_eq!(hermes.origem_destino, OrigemResolucaoDestino::PadraoHome);
        assert!(hermes.destino_base.as_ref().is_some_and(|c| c.existe));
        assert!(hermes.diretorio_perfil.as_ref().is_some_and(|c| c.existe));
        assert!(hermes.arquivo_estado.as_ref().is_some_and(|c| c.existe));

        let claude = diags
            .iter()
            .find(|d| d.harness == Harness::ClaudeCode)
            .expect("claude-code presente");
        assert!(!claude.disponivel);
        assert_eq!(claude.caminhos_indicativos, vec![claude_dir]);

        let _ = std::fs::remove_dir_all(&base);
    }
}
