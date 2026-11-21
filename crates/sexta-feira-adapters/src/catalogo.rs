//! Catálogo dos harnesses suportados ou previstos pela Sexta-feira.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

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

#[cfg(test)]
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
}
