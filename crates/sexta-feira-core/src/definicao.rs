//! Definição agnóstica do mordomo (persona e perfil do usuário).

use std::path::{Path, PathBuf};

use figment::Figment;
use figment::providers::{Format, Toml};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

const PERSONA_PADRAO_TOML: &str = include_str!("../../../recursos/mordomo/persona.toml");
const USUARIO_PADRAO_TOML: &str = include_str!("../../../recursos/mordomo/usuario.toml");

/// Identidade, tom e diretrizes comportamentais do mordomo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct PerfilDoUsuario {
    /// Como o mordomo deve se dirigir ao usuário (ex.: `"senhor"`).
    pub tratamento: String,
    /// Idioma preferido do usuário.
    pub idioma: String,
    /// Preferências explícitas de comunicação e trabalho.
    pub preferencias: Vec<String>,
}

/// Fontes opcionais para compor e sobrescrever a definição do mordomo em camadas.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FontesDaDefinicao {
    /// Diretório de configuração do usuário (ex.: `~/.config/sexta-feira`).
    pub diretorio_config: Option<PathBuf>,
    /// Arquivo TOML explícito com sobrescritas da persona.
    pub persona_explicita: Option<PathBuf>,
    /// Arquivo TOML explícito com sobrescritas do perfil do usuário.
    pub usuario_explicito: Option<PathBuf>,
}

impl FontesDaDefinicao {
    /// Resolve as fontes padrão a partir das variáveis de ambiente atuais.
    #[must_use]
    pub fn padrao_do_ambiente() -> Self {
        let xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let home = std::env::var("HOME").ok();
        Self {
            diretorio_config: Self::resolver_diretorio_config(xdg.as_deref(), home.as_deref()),
            persona_explicita: None,
            usuario_explicito: None,
        }
    }

    /// Resolve o diretório padrão de configuração (`$XDG_CONFIG_HOME/sexta-feira` ou `$HOME/.config/sexta-feira`).
    #[must_use]
    pub fn resolver_diretorio_config(
        xdg_config_home: Option<&str>,
        home: Option<&str>,
    ) -> Option<PathBuf> {
        if let Some(xdg) = xdg_config_home.map(str::trim).filter(|s| !s.is_empty()) {
            return Some(PathBuf::from(xdg).join("sexta-feira"));
        }
        home.map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|h| PathBuf::from(h).join(".config").join("sexta-feira"))
    }
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
    /// Arquivo explícito informado pelo usuário não foi encontrado.
    #[error("arquivo de definição não encontrado: `{caminho}`")]
    ArquivoNaoEncontrado {
        /// Caminho do arquivo ausente.
        caminho: PathBuf,
    },
    /// Falha ao carregar ou validar a configuração em camadas da persona.
    #[error("configuração da persona inválida: {detalhe}")]
    ConfiguracaoPersonaInvalida {
        /// Descrição detalhada do erro de configuração.
        detalhe: String,
    },
    /// Falha ao carregar ou validar a configuração em camadas do usuário.
    #[error("configuração do perfil do usuário inválida: {detalhe}")]
    ConfiguracaoUsuarioInvalida {
        /// Descrição detalhada do erro de configuração.
        detalhe: String,
    },
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

    /// Carrega a definição em camadas: padrão embutido, diretório de configuração
    /// do usuário (opcional) e arquivos explícitos (obrigatórios quando informados).
    ///
    /// # Errors
    ///
    /// Retorna erro se um arquivo explícito não existir, se houver erro de sintaxe ou
    /// chave desconhecida, ou se algum campo obrigatório falhar na validação.
    pub fn carregar(fontes: &FontesDaDefinicao) -> Result<Self, ErroDeDefinicao> {
        let dir_persona = fontes
            .diretorio_config
            .as_deref()
            .map(|dir| dir.join("persona.toml"));
        let dir_usuario = fontes
            .diretorio_config
            .as_deref()
            .map(|dir| dir.join("usuario.toml"));

        let persona: Persona = carregar_secao(
            PERSONA_PADRAO_TOML,
            dir_persona.as_deref(),
            fontes.persona_explicita.as_deref(),
        )
        .map_err(|err| mapear_erro_secao(err, true))?;

        let usuario: PerfilDoUsuario = carregar_secao(
            USUARIO_PADRAO_TOML,
            dir_usuario.as_deref(),
            fontes.usuario_explicito.as_deref(),
        )
        .map_err(|err| mapear_erro_secao(err, false))?;

        let definicao = Self { persona, usuario };
        definicao.validar()?;
        Ok(definicao)
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

enum ErroDeSecao {
    ArquivoAusente(PathBuf),
    Figment(Box<figment::Error>),
}

fn carregar_secao<T: DeserializeOwned>(
    base_toml: &str,
    arquivo_padrao: Option<&Path>,
    arquivo_explicito: Option<&Path>,
) -> Result<T, ErroDeSecao> {
    let mut figment = Figment::from(Toml::string(base_toml));

    if let Some(caminho) = arquivo_padrao
        && caminho.exists()
    {
        figment = figment.merge(Toml::file_exact(caminho));
    }

    if let Some(caminho) = arquivo_explicito {
        if !caminho.exists() {
            return Err(ErroDeSecao::ArquivoAusente(caminho.to_path_buf()));
        }
        figment = figment.merge(Toml::file_exact(caminho));
    }

    figment
        .extract()
        .map_err(|err| ErroDeSecao::Figment(Box::new(err)))
}

fn mapear_erro_secao(erro: ErroDeSecao, eh_persona: bool) -> ErroDeDefinicao {
    match erro {
        ErroDeSecao::ArquivoAusente(caminho) => ErroDeDefinicao::ArquivoNaoEncontrado { caminho },
        ErroDeSecao::Figment(err) if eh_persona => ErroDeDefinicao::ConfiguracaoPersonaInvalida {
            detalhe: err.to_string(),
        },
        ErroDeSecao::Figment(err) => ErroDeDefinicao::ConfiguracaoUsuarioInvalida {
            detalhe: err.to_string(),
        },
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static CONTADOR_TMP: AtomicU64 = AtomicU64::new(0);

    struct DirTemporario {
        caminho: PathBuf,
    }

    impl DirTemporario {
        fn novo(prefixo: &str) -> Self {
            let seq = CONTADOR_TMP.fetch_add(1, Ordering::Relaxed);
            let caminho = std::env::temp_dir().join(format!(
                "sexta-feira-def-{prefixo}-{}-{seq}",
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
}
