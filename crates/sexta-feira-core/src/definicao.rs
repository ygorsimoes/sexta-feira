//! Definição agnóstica do mordomo (persona e perfil do usuário).

use serde::{Deserialize, Serialize};

const PERSONA_PADRAO_TOML: &str = include_str!("../../../recursos/mordomo/persona.toml");
const USUARIO_PADRAO_TOML: &str = include_str!("../../../recursos/mordomo/usuario.toml");

/// Identidade, tom e diretrizes comportamentais do mordomo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Persona {
    /// Nome do assistente.
    pub nome: String,
    /// Arquétipo ou referência de personalidade.
    pub arquetipo: String,
    /// Idioma principal das respostas.
    pub idioma: String,
    /// Resumo curto da identidade do mordomo.
    pub resumo: String,
    /// Traços de voz e postura comunicativa.
    pub tom: Vec<String>,
    /// Regras operacionais de atuação.
    pub diretrizes: Vec<String>,
    /// Limites e restrições obrigatórias.
    pub limites: Vec<String>,
}

/// Preferências e forma de tratamento do usuário.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerfilDoUsuario {
    /// Como o mordomo deve se dirigir ao usuário (ex.: `"senhor"`).
    pub tratamento: String,
    /// Idioma preferido do usuário.
    pub idioma: String,
    /// Preferências explícitas de comunicação e trabalho.
    pub preferencias: Vec<String>,
}

/// Especificação agnóstica completa do mordomo e das preferências do usuário.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinicaoDoMordomo {
    /// Identidade e regras da persona.
    pub persona: Persona,
    /// Dados e preferências do usuário.
    pub usuario: PerfilDoUsuario,
}

/// Erros de carregamento ou validação da definição do mordomo.
#[derive(Debug, thiserror::Error)]
pub enum ErroDeDefinicao {
    /// Falha ao interpretar o arquivo TOML da persona.
    #[error("não foi possível interpretar a definição da persona: {0}")]
    TomlPersonaInvalido(#[source] toml::de::Error),
    /// Falha ao interpretar o arquivo TOML do perfil do usuário.
    #[error("não foi possível interpretar o perfil do usuário: {0}")]
    TomlUsuarioInvalido(#[source] toml::de::Error),
    /// Campo textual obrigatório vazio ou composto apenas por espaços.
    #[error("campo obrigatório `{campo}` não pode ser vazio")]
    CampoObrigatorioVazio {
        /// Nome qualificado do campo inválido.
        campo: &'static str,
    },
    /// Lista obrigatória sem itens ou contendo item em branco.
    #[error("lista obrigatória `{campo}` deve conter ao menos um item válido")]
    ListaObrigatoriaVazia {
        /// Nome qualificado da lista inválida.
        campo: &'static str,
    },
}

impl DefinicaoDoMordomo {
    /// Carrega e valida a definição padrão embutida no binário.
    ///
    /// # Errors
    ///
    /// Retorna erro se os recursos embutidos estiverem inválidos.
    pub fn padrao() -> Result<Self, ErroDeDefinicao> {
        Self::de_toml(PERSONA_PADRAO_TOML, USUARIO_PADRAO_TOML)
    }

    /// Carrega e valida uma definição a partir de conteúdos TOML.
    ///
    /// # Errors
    ///
    /// Retorna erro se o TOML for inválido ou algum campo obrigatório falhar na validação.
    pub fn de_toml(persona_toml: &str, usuario_toml: &str) -> Result<Self, ErroDeDefinicao> {
        let persona: Persona =
            toml::from_str(persona_toml).map_err(ErroDeDefinicao::TomlPersonaInvalido)?;
        let usuario: PerfilDoUsuario =
            toml::from_str(usuario_toml).map_err(ErroDeDefinicao::TomlUsuarioInvalido)?;

        let definicao = Self { persona, usuario };
        definicao.validar()?;
        Ok(definicao)
    }

    fn validar(&self) -> Result<(), ErroDeDefinicao> {
        exigir_texto(&self.persona.nome, "persona.nome")?;
        exigir_texto(&self.persona.arquetipo, "persona.arquetipo")?;
        exigir_texto(&self.persona.idioma, "persona.idioma")?;
        exigir_texto(&self.persona.resumo, "persona.resumo")?;
        exigir_lista(&self.persona.tom, "persona.tom")?;
        exigir_lista(&self.persona.diretrizes, "persona.diretrizes")?;
        exigir_lista(&self.persona.limites, "persona.limites")?;

        exigir_texto(&self.usuario.tratamento, "usuario.tratamento")?;
        exigir_texto(&self.usuario.idioma, "usuario.idioma")?;
        exigir_lista(&self.usuario.preferencias, "usuario.preferencias")?;
        Ok(())
    }
}

fn exigir_texto(valor: &str, campo: &'static str) -> Result<(), ErroDeDefinicao> {
    if valor.trim().is_empty() {
        return Err(ErroDeDefinicao::CampoObrigatorioVazio { campo });
    }
    Ok(())
}

fn exigir_lista(itens: &[String], campo: &'static str) -> Result<(), ErroDeDefinicao> {
    if itens.is_empty() || itens.iter().any(|item| item.trim().is_empty()) {
        return Err(ErroDeDefinicao::ListaObrigatoriaVazia { campo });
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn definicao_padrao_carrega_com_valores_esperados() {
        let def = DefinicaoDoMordomo::padrao().expect("definição padrão deve ser válida");
        assert_eq!(def.persona.nome, "Sexta-feira");
        assert_eq!(def.persona.arquetipo, "J.A.R.V.I.S.");
        assert_eq!(def.persona.idioma, "pt-BR");
        assert_eq!(def.usuario.tratamento, "senhor");
        assert_ne!(def.persona.diretrizes, Vec::<String>::new());
        assert_ne!(def.persona.limites, Vec::<String>::new());
    }

    #[test]
    fn rejeita_tratamento_ou_nome_vazio() {
        let persona = r#"
            nome = "   "
            arquetipo = "J.A.R.V.I.S."
            idioma = "pt-BR"
            resumo = "Mordomo pessoal."
            tom = ["formal"]
            diretrizes = ["ser clara"]
            limites = ["confirmar ações externas"]
        "#;
        let usuario = r#"
            tratamento = "senhor"
            idioma = "pt-BR"
            preferencias = ["objetividade"]
        "#;
        assert!(matches!(
            DefinicaoDoMordomo::de_toml(persona, usuario),
            Err(ErroDeDefinicao::CampoObrigatorioVazio {
                campo: "persona.nome"
            })
        ));
    }

    #[test]
    fn rejeita_lista_obrigatoria_vazia_ou_item_em_branco() {
        let persona = r#"
            nome = "Sexta-feira"
            arquetipo = "J.A.R.V.I.S."
            idioma = "pt-BR"
            resumo = "Mordomo pessoal."
            tom = []
            diretrizes = ["ser clara"]
            limites = ["confirmar ações externas"]
        "#;
        let usuario = r#"
            tratamento = "senhor"
            idioma = "pt-BR"
            preferencias = ["objetividade"]
        "#;
        assert!(matches!(
            DefinicaoDoMordomo::de_toml(persona, usuario),
            Err(ErroDeDefinicao::ListaObrigatoriaVazia {
                campo: "persona.tom"
            })
        ));
    }
}
