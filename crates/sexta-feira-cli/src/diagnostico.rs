//! Subcomando `diagnostico`: inspeção completa de ambiente, configuração e harnesses.

use std::io::Write;
use std::path::PathBuf;

use clap::Args;
use serde::Serialize;
use sexta_feira_adapters::{
    AdaptadorHermes, CaminhoDiagnosticado, ConfiguracaoHermes, DiagnosticoHarness, Harness,
    OrigemResolucaoDestino, inspecionar_ambiente_harnesses,
};
use sexta_feira_core::{
    AdaptadorDeSetup, CamadaDeDefinicao, DefinicaoDoMordomo, FontesDaDefinicao, OrigemDaCamada,
    StatusDaCamada, verificar,
};

use crate::{escrever_json, verbose};

/// Opções do subcomando `diagnostico`.
#[derive(Debug, Clone, Args)]
pub struct OpcoesDiagnostico {
    /// Harness específico para detalhar (opcional; sem valor inspeciona todos).
    pub harness: Option<Harness>,
    /// Caminho explícito para o diretório base de destino do harness.
    #[arg(long, alias = "hermes-home")]
    pub destino: Option<PathBuf>,
    /// Nome do perfil isolado no harness (padrão: `sexta-feira`).
    #[arg(long)]
    pub perfil: Option<String>,
    /// Arquivo TOML explícito com sobrescritas da persona.
    #[arg(long)]
    pub persona: Option<PathBuf>,
    /// Arquivo TOML explícito com sobrescritas do perfil do usuário.
    #[arg(long)]
    pub usuario: Option<PathBuf>,
}

#[derive(Serialize)]
struct VariaveisAmbienteJson {
    home: Option<String>,
    xdg_config_home: Option<String>,
    hermes_home: Option<String>,
    sexta_feira_log: Option<String>,
}

#[derive(Serialize)]
struct ConfiguracaoDiagJson {
    diretorio: Option<String>,
    camadas: Vec<CamadaDeDefinicao>,
}

#[derive(Serialize)]
struct DefinicaoDiagJson {
    valida: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    persona_nome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tratamento: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    idioma: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    erro: Option<String>,
}

#[derive(Serialize)]
struct ItemSyncJson {
    caminho: String,
    sincronizado: bool,
    detalhe: String,
}

#[derive(Serialize)]
struct HarnessDiagJson {
    id: &'static str,
    disponivel: bool,
    status: &'static str,
    origem_destino: OrigemResolucaoDestino,
    #[serde(skip_serializing_if = "Option::is_none")]
    destino_base: Option<CaminhoDiagnosticado>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diretorio_perfil: Option<CaminhoDiagnosticado>,
    #[serde(skip_serializing_if = "Option::is_none")]
    arquivo_estado: Option<CaminhoDiagnosticado>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    caminhos_indicativos: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    erro: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sincronizado: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    itens: Vec<ItemSyncJson>,
}

#[derive(Serialize)]
struct DiagnosticoJson {
    versao: &'static str,
    plataforma: String,
    variaveis_ambiente: VariaveisAmbienteJson,
    configuracao: ConfiguracaoDiagJson,
    definicao: DefinicaoDiagJson,
    harnesses: Vec<HarnessDiagJson>,
    proximo_passo: String,
}

/// Executa o subcomando `diagnostico`, sempre retornando sucesso após inspecionar o ambiente.
pub fn executar(opcoes: &OpcoesDiagnostico, json: bool, nivel_verbose: u8) -> anyhow::Result<()> {
    let env_home = ler_env("HOME");
    let env_xdg = ler_env("XDG_CONFIG_HOME");
    let env_hermes = ler_env("HERMES_HOME");
    let env_log = ler_env("SEXTA_FEIRA_LOG");

    let mut fontes = FontesDaDefinicao::padrao_do_ambiente();
    if opcoes.persona.is_some() {
        fontes.persona_explicita.clone_from(&opcoes.persona);
    }
    if opcoes.usuario.is_some() {
        fontes.usuario_explicito.clone_from(&opcoes.usuario);
    }

    let camadas = fontes.inspecionar_camadas();
    let resultado_def = DefinicaoDoMordomo::carregar_com_camadas(&fontes);
    if let Ok((ref def, ref cam)) = resultado_def {
        verbose::emitir_contexto_definicao(nivel_verbose, def, cam);
    }

    let inspecoes = inspecionar_ambiente_harnesses(
        opcoes.harness,
        opcoes.destino.as_deref(),
        opcoes.perfil.as_deref(),
        env_hermes.as_deref(),
        env_home.as_deref(),
    );

    let definicao_ok = resultado_def.as_ref().ok().map(|(d, _)| d);
    let harnesses_diag: Vec<HarnessDiagJson> = inspecoes
        .into_iter()
        .map(|item| enriquecer_harness(item, definicao_ok, opcoes))
        .collect();

    let proximo_passo = definir_proximo_passo(
        resultado_def.is_ok(),
        opcoes.harness,
        &harnesses_diag,
        fontes.diretorio_config.as_deref(),
    );

    let relatorio = DiagnosticoJson {
        versao: env!("CARGO_PKG_VERSION"),
        plataforma: format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
        variaveis_ambiente: VariaveisAmbienteJson {
            home: env_home,
            xdg_config_home: env_xdg,
            hermes_home: env_hermes,
            sexta_feira_log: env_log,
        },
        configuracao: ConfiguracaoDiagJson {
            diretorio: fontes
                .diretorio_config
                .as_ref()
                .map(|p| p.display().to_string()),
            camadas,
        },
        definicao: match resultado_def {
            Ok((def, _)) => DefinicaoDiagJson {
                valida: true,
                persona_nome: Some(def.persona.nome),
                tratamento: Some(def.usuario.tratamento),
                idioma: Some(def.usuario.idioma),
                erro: None,
            },
            Err(err) => DefinicaoDiagJson {
                valida: false,
                persona_nome: None,
                tratamento: None,
                idioma: None,
                erro: Some(err.to_string()),
            },
        },
        harnesses: harnesses_diag,
        proximo_passo,
    };

    let mut saida = std::io::stdout().lock();
    if json {
        escrever_json(&mut saida, &relatorio)?;
        return Ok(());
    }

    imprimir_texto(&mut saida, &relatorio)
}

fn enriquecer_harness(
    item: DiagnosticoHarness,
    definicao: Option<&DefinicaoDoMordomo>,
    opcoes: &OpcoesDiagnostico,
) -> HarnessDiagJson {
    let mut sincronizado = None;
    let mut itens = Vec::new();
    let mut erro = item.erro_destino;

    if item.harness == Harness::Hermes
        && erro.is_none()
        && let Some(def) = definicao
    {
        match ConfiguracaoHermes::resolver(opcoes.destino.as_deref(), opcoes.perfil.as_deref()) {
            Ok(config) => {
                let adaptador = AdaptadorHermes::new(config);
                match adaptador.planejar_arquivos(def) {
                    Ok(desejados) => match verificar(&desejados, &adaptador.arquivo_estado()) {
                        Ok(verif) => {
                            sincronizado = Some(verif.sincronizado);
                            itens = verif
                                .itens
                                .into_iter()
                                .map(|it| ItemSyncJson {
                                    caminho: it.caminho.display().to_string(),
                                    sincronizado: it.sincronizado,
                                    detalhe: it.detalhe,
                                })
                                .collect();
                        }
                        Err(e) => erro = Some(e.to_string()),
                    },
                    Err(e) => erro = Some(e.to_string()),
                }
            }
            Err(e) => erro = Some(e.to_string()),
        }
    }

    HarnessDiagJson {
        id: item.harness.id(),
        disponivel: item.disponivel,
        status: item.status,
        origem_destino: item.origem_destino,
        destino_base: item.destino_base,
        diretorio_perfil: item.diretorio_perfil,
        arquivo_estado: item.arquivo_estado,
        caminhos_indicativos: item
            .caminhos_indicativos
            .into_iter()
            .map(|p| p.display().to_string())
            .collect(),
        erro,
        sincronizado,
        itens,
    }
}

fn definir_proximo_passo(
    definicao_valida: bool,
    filtro: Option<Harness>,
    harnesses: &[HarnessDiagJson],
    diretorio_config: Option<&std::path::Path>,
) -> String {
    if !definicao_valida {
        let alvo = diretorio_config.map_or_else(
            || "${XDG_CONFIG_HOME:-~/.config}/sexta-feira/usuario.toml".to_owned(),
            |d| d.join("usuario.toml").display().to_string(),
        );
        return format!(
            "Corrija `{alvo}` ou execute `sexta-feira iniciar --sobrescrever` para regenerar com backup `.bak`."
        );
    }

    if let Some(hermes) = harnesses.iter().find(|h| h.id == "hermes") {
        if let Some(err) = hermes.erro.as_deref() {
            return format!(
                "Corrija o erro na inspeção do `hermes` ({err}) e rode `sexta-feira diagnostico` novamente."
            );
        }
        return match hermes.sincronizado {
            Some(true) => "Setup do `hermes` já está sincronizado.".to_owned(),
            Some(false) => {
                "Execute `sexta-feira plano hermes` para revisar e `sexta-feira aplicar hermes` para aplicar."
                    .to_owned()
            }
            None => "Informe `--destino <dir>` ou defina `HERMES_HOME`/`HOME` para inspecionar o `hermes`."
                .to_owned(),
        };
    }

    let id = filtro.map_or("este harness", Harness::id);
    format!(
        "`{id}` ainda não possui adapter oficial na CLI; use a skill em `recursos/skills/instalar-sexta-feira/SKILL.md` para instalação assistida."
    )
}

fn imprimir_texto(saida: &mut impl Write, rel: &DiagnosticoJson) -> anyhow::Result<()> {
    writeln!(
        saida,
        "Diagnóstico Sexta-feira v{} ({})",
        rel.versao, rel.plataforma
    )?;
    writeln!(
        saida,
        "Ambiente: HOME={} | XDG_CONFIG_HOME={} | HERMES_HOME={} | SEXTA_FEIRA_LOG={}",
        rel.variaveis_ambiente
            .home
            .as_deref()
            .unwrap_or("<ausente>"),
        rel.variaveis_ambiente
            .xdg_config_home
            .as_deref()
            .unwrap_or("<ausente>"),
        rel.variaveis_ambiente
            .hermes_home
            .as_deref()
            .unwrap_or("<ausente>"),
        rel.variaveis_ambiente
            .sexta_feira_log
            .as_deref()
            .unwrap_or("<ausente>"),
    )?;
    writeln!(
        saida,
        "Configuração: {}",
        rel.configuracao
            .diretorio
            .as_deref()
            .unwrap_or("<indisponível>")
    )?;
    for camada in &rel.configuracao.camadas {
        let origem = match camada.origem {
            OrigemDaCamada::Embutida => "embutida",
            OrigemDaCamada::DiretorioConfig => "config",
            OrigemDaCamada::ArquivoExplicito => "explícita",
        };
        let status = match camada.status {
            StatusDaCamada::Aplicada => "aplicada",
            StatusDaCamada::Ausente => "ausente",
        };
        let caminho = camada
            .caminho
            .as_ref()
            .map_or_else(|| "<embutido>".to_owned(), |p| p.display().to_string());
        writeln!(
            saida,
            "  - [{}] {origem} ({status}): {caminho}",
            camada.secao
        )?;
    }

    if rel.definicao.valida {
        writeln!(
            saida,
            "Definição: válida (persona: {}, tratamento: \"{}\", idioma: {})",
            rel.definicao.persona_nome.as_deref().unwrap_or("-"),
            rel.definicao.tratamento.as_deref().unwrap_or("-"),
            rel.definicao.idioma.as_deref().unwrap_or("-"),
        )?;
    } else {
        writeln!(
            saida,
            "Definição: inválida ({})",
            rel.definicao.erro.as_deref().unwrap_or("erro desconhecido")
        )?;
    }

    imprimir_harnesses_texto(saida, &rel.harnesses, rel.definicao.valida)?;

    writeln!(saida, "Próximo passo: {}", rel.proximo_passo)?;
    Ok(())
}

fn imprimir_harnesses_texto(
    saida: &mut impl Write,
    harnesses: &[HarnessDiagJson],
    definicao_valida: bool,
) -> anyhow::Result<()> {
    writeln!(saida, "Harnesses:")?;
    for h in harnesses {
        let sync = match (
            h.sincronizado,
            h.erro.as_deref(),
            h.disponivel,
            definicao_valida,
        ) {
            (Some(true), _, _, _) => "sincronizado",
            (Some(false), _, _, _) => "pendente/divergente",
            (None, Some(_), _, _) => "erro na inspeção",
            (None, None, true, false) => "não verificado (definição inválida)",
            (None, None, true, true) => "destino indisponível",
            (None, None, false, _) => "sem adapter",
        };
        writeln!(saida, "  - {} [{} | {sync}]", h.id, h.status)?;
        if let Some(err) = h.erro.as_deref() {
            writeln!(saida, "      erro: {err}")?;
        }
        if let Some(perfil) = &h.diretorio_perfil {
            writeln!(
                saida,
                "      perfil: {} (existe: {})",
                perfil.caminho.display(),
                if perfil.existe { "sim" } else { "não" }
            )?;
        }
        if !h.caminhos_indicativos.is_empty() {
            writeln!(
                saida,
                "      indícios locais: {}",
                h.caminhos_indicativos.join(", ")
            )?;
        }
    }
    Ok(())
}

fn ler_env(chave: &str) -> Option<String> {
    std::env::var(chave).ok().filter(|s| !s.trim().is_empty())
}
