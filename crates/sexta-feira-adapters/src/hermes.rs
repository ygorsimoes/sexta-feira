//! Adaptador declarativo para o perfil do Hermes Agent.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use sexta_feira_core::{
    AdaptadorDeSetup, ArquivoDesejado, DefinicaoDoMordomo, ErroDeAdaptador, EstrategiaDeEscrita,
};

/// Nome padrão do perfil isolado criado no Hermes.
pub const PERFIL_PADRAO: &str = "sexta-feira";

/// Limite máximo de linhas recomendado para `SOUL.md` no Hermes.
pub const LIMITE_LINHAS_SOUL: usize = 80;

const MARCADOR_USER_INICIO: &str = "<!-- sexta-feira:inicio -->";
const MARCADOR_USER_FIM: &str = "<!-- sexta-feira:fim -->";

/// Configuração resolvida para um perfil alvo do Hermes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfiguracaoHermes {
    /// Diretório raiz do Hermes (`$HERMES_HOME` ou `~/.hermes`).
    pub hermes_home: PathBuf,
    /// Nome do perfil isolado dentro de `<hermes_home>/profiles/`.
    pub perfil: String,
}

/// Erros de resolução da configuração do Hermes.
#[derive(Debug, thiserror::Error)]
pub enum ErroDeHermes {
    /// Não foi possível determinar o diretório base do Hermes.
    #[error(
        "não foi possível determinar o diretório base do Hermes; informe `--hermes-home` ou defina `HERMES_HOME`/`HOME`"
    )]
    DiretorioBaseIndisponivel,
    /// O caminho informado via `--hermes-home` é vazio.
    #[error("o caminho informado em `--hermes-home` não pode ser vazio")]
    HermesHomeVazio,
    /// Nome de perfil inválido ou inseguro para compor caminhos.
    #[error(
        "nome de perfil inválido (`{perfil}`): use apenas letras, números, `-` ou `_`, sem espaços ou separadores"
    )]
    PerfilInvalido {
        /// Valor rejeitado.
        perfil: String,
    },
}

impl ConfiguracaoHermes {
    /// Resolve o diretório base do Hermes e o nome do perfil usando o ambiente atual.
    ///
    /// # Errors
    ///
    /// Retorna erro se o caminho base não puder ser determinado ou se o perfil for inválido.
    pub fn resolver(
        hermes_home_explicito: Option<&Path>,
        perfil: Option<&str>,
    ) -> Result<Self, ErroDeHermes> {
        let env_hermes_home = std::env::var("HERMES_HOME").ok();
        let env_home = std::env::var("HOME").ok();
        Self::resolver_com_env(
            hermes_home_explicito,
            env_hermes_home.as_deref(),
            env_home.as_deref(),
            perfil,
        )
    }

    /// Resolve a configuração do Hermes a partir de valores explícitos e variáveis injetadas.
    ///
    /// Ordem de precedência para o diretório base:
    /// 1. `hermes_home_explicito`;
    /// 2. `HERMES_HOME` (quando não vazia);
    /// 3. `<HOME>/.hermes`.
    ///
    /// # Errors
    ///
    /// Retorna erro se o caminho base estiver indisponível ou se o perfil for inválido.
    pub fn resolver_com_env(
        hermes_home_explicito: Option<&Path>,
        env_hermes_home: Option<&str>,
        env_home: Option<&str>,
        perfil: Option<&str>,
    ) -> Result<Self, ErroDeHermes> {
        let hermes_home = if let Some(caminho) = hermes_home_explicito {
            if caminho.as_os_str().is_empty() {
                return Err(ErroDeHermes::HermesHomeVazio);
            }
            caminho.to_path_buf()
        } else if let Some(caminho) = env_hermes_home.map(str::trim).filter(|s| !s.is_empty()) {
            PathBuf::from(caminho)
        } else if let Some(home) = env_home.map(str::trim).filter(|s| !s.is_empty()) {
            PathBuf::from(home).join(".hermes")
        } else {
            return Err(ErroDeHermes::DiretorioBaseIndisponivel);
        };

        let nome_perfil = validar_perfil(perfil.unwrap_or(PERFIL_PADRAO))?;
        Ok(Self {
            hermes_home,
            perfil: nome_perfil,
        })
    }

    /// Diretório raiz do perfil isolado (`<hermes_home>/profiles/<perfil>`).
    #[must_use]
    pub fn diretorio_do_perfil(&self) -> PathBuf {
        self.hermes_home.join("profiles").join(&self.perfil)
    }
}

/// Adaptador que traduz a definição do mordomo para o perfil do Hermes Agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptadorHermes {
    config: ConfiguracaoHermes,
}

impl AdaptadorHermes {
    /// Cria um novo adaptador para a configuração informada.
    #[must_use]
    pub const fn new(config: ConfiguracaoHermes) -> Self {
        Self { config }
    }

    /// Referência à configuração do Hermes associada a este adaptador.
    #[must_use]
    pub const fn configuracao(&self) -> &ConfiguracaoHermes {
        &self.config
    }
}

impl AdaptadorDeSetup for AdaptadorHermes {
    fn id(&self) -> &'static str {
        "hermes"
    }

    fn arquivo_estado(&self) -> PathBuf {
        self.config
            .diretorio_do_perfil()
            .join(".sexta-feira-estado.toml")
    }

    fn planejar_arquivos(
        &self,
        definicao: &DefinicaoDoMordomo,
    ) -> Result<Vec<ArquivoDesejado>, ErroDeAdaptador> {
        let perfil_dir = self.config.diretorio_do_perfil();
        let soul = renderizar_soul(definicao)?;
        let user = renderizar_user(definicao);

        Ok(vec![
            ArquivoDesejado {
                caminho: perfil_dir.join("SOUL.md"),
                conteudo: soul,
                estrategia: EstrategiaDeEscrita::ArquivoIntegral,
            },
            ArquivoDesejado {
                caminho: perfil_dir.join("memories").join("USER.md"),
                conteudo: user,
                estrategia: EstrategiaDeEscrita::BlocoDelimitado {
                    inicio: MARCADOR_USER_INICIO,
                    fim: MARCADOR_USER_FIM,
                },
            },
        ])
    }
}

fn validar_perfil(perfil: &str) -> Result<String, ErroDeHermes> {
    let valido = !perfil.is_empty()
        && perfil
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valido {
        return Err(ErroDeHermes::PerfilInvalido {
            perfil: perfil.to_owned(),
        });
    }
    Ok(perfil.to_owned())
}

fn renderizar_soul(definicao: &DefinicaoDoMordomo) -> Result<String, ErroDeAdaptador> {
    let persona = &definicao.persona;
    let usuario = &definicao.usuario;
    let mut saida = String::new();

    let _ = writeln!(saida, "# {}", persona.nome);
    let _ = writeln!(saida);
    let _ = writeln!(
        saida,
        "Você é **{}**, assistente pessoal inspirada em **{}** ({}).",
        persona.nome, persona.arquetipo, persona.idioma
    );
    let _ = writeln!(saida, "{}", persona.resumo);
    let _ = writeln!(
        saida,
        "Dirija-se sempre ao usuário como **\"{}\"**.",
        usuario.tratamento
    );

    escrever_secao(&mut saida, "Tom e Postura", &persona.tom);
    escrever_secao(&mut saida, "Diretrizes de Atuação", &persona.diretrizes);
    escrever_secao(&mut saida, "Limites", &persona.limites);

    let total_linhas = saida.lines().count();
    if total_linhas > LIMITE_LINHAS_SOUL {
        return Err(ErroDeAdaptador::Mensagem(format!(
            "SOUL.md excedeu o limite de {LIMITE_LINHAS_SOUL} linhas ({total_linhas} linhas)"
        )));
    }

    Ok(saida)
}

fn renderizar_user(definicao: &DefinicaoDoMordomo) -> String {
    let usuario = &definicao.usuario;
    let mut saida = String::new();

    let _ = writeln!(saida, "## Preferências gerenciadas pela Sexta-feira");
    let _ = writeln!(saida);
    let _ = writeln!(saida, "- **Tratamento**: {}", usuario.tratamento);
    let _ = writeln!(saida, "- **Idioma**: {}", usuario.idioma);
    for preferencia in &usuario.preferencias {
        let _ = writeln!(saida, "- {preferencia}");
    }

    saida
}

fn escrever_secao(saida: &mut String, titulo: &str, itens: &[String]) {
    let _ = writeln!(saida);
    let _ = writeln!(saida, "## {titulo}");
    for item in itens {
        let _ = writeln!(saida, "- {item}");
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use sexta_feira_core::{
        AdaptadorDeSetup, DefinicaoDoMordomo, EstrategiaDeEscrita, aplicar, planejar, verificar,
    };

    use super::*;

    static CONTADOR_TMP: AtomicU64 = AtomicU64::new(0);

    struct DirTemporario {
        caminho: PathBuf,
    }

    impl DirTemporario {
        fn novo(prefixo: &str) -> Self {
            let seq = CONTADOR_TMP.fetch_add(1, Ordering::Relaxed);
            let caminho = std::env::temp_dir().join(format!(
                "sexta-feira-hermes-{prefixo}-{}-{seq}",
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
    fn resolve_hermes_home_por_prioridade() {
        let explicito = Path::new("/tmp/hermes-custom");
        let cfg = ConfiguracaoHermes::resolver_com_env(
            Some(explicito),
            Some("/tmp/env-hermes"),
            Some("/home/usuario"),
            None,
        )
        .expect("caminho explícito deve prevalecer");
        assert_eq!(cfg.hermes_home, PathBuf::from("/tmp/hermes-custom"));
        assert_eq!(cfg.perfil, PERFIL_PADRAO);
        assert_eq!(
            cfg.diretorio_do_perfil(),
            PathBuf::from("/tmp/hermes-custom/profiles/sexta-feira")
        );

        let via_env = ConfiguracaoHermes::resolver_com_env(
            None,
            Some("/tmp/env-hermes"),
            Some("/home/usuario"),
            Some("pessoal"),
        )
        .expect("HERMES_HOME deve prevalecer sobre HOME");
        assert_eq!(via_env.hermes_home, PathBuf::from("/tmp/env-hermes"));
        assert_eq!(via_env.perfil, "pessoal");

        let via_home =
            ConfiguracaoHermes::resolver_com_env(None, Some("   "), Some("/home/usuario"), None)
                .expect("HERMES_HOME vazio deve cair para HOME/.hermes");
        assert_eq!(via_home.hermes_home, PathBuf::from("/home/usuario/.hermes"));

        let erro = ConfiguracaoHermes::resolver_com_env(None, None, None, None)
            .expect_err("ausência de origem deve falhar");
        assert!(matches!(erro, ErroDeHermes::DiretorioBaseIndisponivel));
    }

    #[test]
    fn rejeita_perfil_invalido() {
        for invalido in ["", "   ", "../fora", "a/b", "a\\b", ".", "..", "meu perfil"] {
            let err = ConfiguracaoHermes::resolver_com_env(
                Some(Path::new("/tmp/hermes")),
                None,
                None,
                Some(invalido),
            )
            .expect_err("perfil inválido deve ser rejeitado");
            assert!(
                matches!(err, ErroDeHermes::PerfilInvalido { .. }),
                "esperava PerfilInvalido para {invalido:?}, obteve {err:?}"
            );
        }
    }

    #[test]
    fn gera_soul_enxuto_com_personalidade_jarvis_e_user_com_senhor() {
        let def = DefinicaoDoMordomo::padrao().expect("definição padrão");
        let cfg =
            ConfiguracaoHermes::resolver_com_env(Some(Path::new("/tmp/hermes")), None, None, None)
                .expect("configuração válida");
        let adaptador = AdaptadorHermes::new(cfg);

        let arquivos = adaptador
            .planejar_arquivos(&def)
            .expect("deve gerar arquivos do Hermes");
        assert_eq!(arquivos.len(), 2);

        let soul = &arquivos[0];
        assert!(soul.caminho.ends_with("profiles/sexta-feira/SOUL.md"));
        assert_eq!(soul.estrategia, EstrategiaDeEscrita::ArquivoIntegral);
        assert!(soul.conteudo.lines().count() <= LIMITE_LINHAS_SOUL);
        assert!(soul.conteudo.contains("Sexta-feira"));
        assert!(soul.conteudo.contains("J.A.R.V.I.S."));
        assert!(soul.conteudo.contains("pt-BR"));
        assert!(soul.conteudo.contains("senhor"));

        let user = &arquivos[1];
        assert!(
            user.caminho
                .ends_with("profiles/sexta-feira/memories/USER.md")
        );
        assert!(matches!(
            user.estrategia,
            EstrategiaDeEscrita::BlocoDelimitado { .. }
        ));
        assert!(user.conteudo.contains("senhor"));
    }

    #[test]
    fn ciclo_completo_com_motor_declarativo_do_core() {
        let dir = DirTemporario::novo("ciclo");
        let def = DefinicaoDoMordomo::padrao().expect("definição padrão");
        let cfg = ConfiguracaoHermes::resolver_com_env(Some(dir.path()), None, None, None)
            .expect("configuração válida");
        let adaptador = AdaptadorHermes::new(cfg.clone());

        let user_md = cfg.diretorio_do_perfil().join("memories/USER.md");
        fs::create_dir_all(user_md.parent().expect("pai")).expect("cria memories");
        fs::write(&user_md, "- Gosta de café sem açúcar.\n").expect("grava memória prévia");

        let desejados = adaptador
            .planejar_arquivos(&def)
            .expect("planejar arquivos");
        let plano = planejar(desejados.clone(), adaptador.arquivo_estado()).expect("planejar");
        aplicar(&plano, false).expect("aplicar");

        let verif = verificar(&desejados, &adaptador.arquivo_estado()).expect("verificar");
        assert!(verif.sincronizado);

        let conteudo_user = fs::read_to_string(&user_md).expect("lê USER.md");
        assert!(conteudo_user.contains("senhor"));
        assert!(conteudo_user.contains("- Gosta de café sem açúcar."));
    }
}
