//! Subcomando `iniciar`: configuração guiada ou scriptável do `usuario.toml`.

use std::fs::{self, OpenOptions};
use std::io::{BufRead as _, IsTerminal as _, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context as _, bail};
use clap::Args;
use serde::{Deserialize, Serialize};
use sexta_feira_adapters::{AdaptadorHermes, ConfiguracaoHermes, Harness};
use sexta_feira_core::{
    AdaptadorDeSetup, DefinicaoDoMordomo, FontesDaDefinicao, OperacaoPlanejada, aplicar, planejar,
    verificar,
};

use crate::{ErroHarnessNaoImplementado, escrever_json, verbose};

static CONTADOR_VALIDACAO: AtomicU64 = AtomicU64::new(0);

/// Opções do subcomando `iniciar`.
#[derive(Debug, Clone, Args)]
#[allow(clippy::struct_excessive_bools)]
pub struct OpcoesIniciar {
    /// Como a Sexta-feira deve se dirigir a você (ex.: `"senhor"`, `"comandante"`).
    #[arg(long)]
    pub tratamento: Option<String>,
    /// Idioma preferido (padrão: `pt-BR`).
    #[arg(long)]
    pub idioma: Option<String>,
    /// Preferência de comunicação/trabalho (pode ser repetida; substitui a lista padrão).
    #[arg(long = "preferencia")]
    pub preferencias: Vec<String>,
    /// Diretório onde gravar `usuario.toml` (padrão: `${XDG_CONFIG_HOME:-~/.config}/sexta-feira`).
    #[arg(long)]
    pub config_dir: Option<PathBuf>,
    /// Harness opcional para pré-visualizar ou aplicar após configurar.
    #[arg(long)]
    pub harness: Option<Harness>,
    /// Caminho explícito para o diretório base de destino do harness.
    #[arg(long, alias = "hermes-home")]
    pub destino: Option<PathBuf>,
    /// Nome do perfil isolado no harness (padrão: `sexta-feira`).
    #[arg(long)]
    pub perfil: Option<String>,
    /// Aplica imediatamente o setup no harness escolhido (requer `--harness`).
    #[arg(long, requires = "harness")]
    pub aplicar: bool,
    /// Substitui arquivos em conflito no harness gerando backup `.bak`.
    #[arg(long, requires = "aplicar")]
    pub forcar: bool,
    /// Autoriza atualizar `usuario.toml` existente (gerando backup `.bak`).
    #[arg(long)]
    pub sobrescrever: bool,
    /// Executa sem prompts no terminal (ideal para scripts e agentes).
    #[arg(long)]
    pub nao_interativo: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct UsuarioParcial {
    #[serde(skip_serializing_if = "Option::is_none")]
    tratamento: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    idioma: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preferencias: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum AcaoConfig {
    Criado,
    Atualizado,
    Mantido,
}

#[derive(Serialize)]
struct AplicadoIniciarJson {
    harness: &'static str,
    criados: usize,
    atualizados: usize,
    backups_criados: Vec<String>,
    sincronizado: bool,
}

#[derive(Serialize)]
struct PlanoIniciarJson {
    harness: &'static str,
    criar: usize,
    atualizar: usize,
    conflitos: usize,
}

#[derive(Serialize)]
struct IniciarJson {
    caminho_config: String,
    acao: AcaoConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    backup_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plano: Option<PlanoIniciarJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aplicado: Option<AplicadoIniciarJson>,
}

struct TempValidacao {
    caminho: PathBuf,
}

impl TempValidacao {
    fn gravar(conteudo: &str) -> anyhow::Result<Self> {
        let seq = CONTADOR_VALIDACAO.fetch_add(1, Ordering::Relaxed);
        let caminho = std::env::temp_dir().join(format!(
            "sexta-feira-validacao-{}-{seq}.toml",
            std::process::id()
        ));
        fs::write(&caminho, conteudo).with_context(|| {
            format!(
                "não foi possível gravar arquivo temporário de validação em `{}`",
                caminho.display()
            )
        })?;
        Ok(Self { caminho })
    }
}

impl Drop for TempValidacao {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.caminho);
    }
}

/// Executa o subcomando `iniciar`.
pub fn executar(opcoes: &OpcoesIniciar, json: bool, nivel_verbose: u8) -> anyhow::Result<()> {
    let interativo = !opcoes.nao_interativo;
    if interativo && !std::io::stdin().is_terminal() {
        bail!(
            "modo interativo requer um terminal (TTY); passe `--nao-interativo` (ex.: `sexta-feira iniciar --tratamento \"senhor\" --nao-interativo`)"
        );
    }

    let config_dir = resolver_diretorio_alvo(opcoes)?;
    let usuario_path = config_dir.join("usuario.toml");

    let (acao, backup_config) =
        processar_usuario_toml(opcoes, &config_dir, &usuario_path, interativo)?;

    let (harness_escolhido, aplicar_agora) = if interativo && opcoes.harness.is_none() && !json {
        perguntar_harness_interativo()?
    } else {
        (opcoes.harness, opcoes.aplicar)
    };

    let mut plano_json = None;
    let mut aplicado_json = None;

    if let Some(harness) = harness_escolhido {
        let (p_json, a_json) =
            executar_etapa_harness(harness, &config_dir, opcoes, aplicar_agora, nivel_verbose)?;
        plano_json = p_json;
        aplicado_json = a_json;
    }

    let mut saida = std::io::stdout().lock();
    if json {
        escrever_json(
            &mut saida,
            &IniciarJson {
                caminho_config: usuario_path.display().to_string(),
                acao,
                backup_config: backup_config.map(|p| p.display().to_string()),
                plano: plano_json,
                aplicado: aplicado_json,
            },
        )?;
        return Ok(());
    }

    imprimir_resumo_texto(
        &mut saida,
        &usuario_path,
        acao,
        backup_config.as_deref(),
        harness_escolhido,
        plano_json.as_ref(),
        aplicado_json.as_ref(),
    )
}

fn resolver_diretorio_alvo(opcoes: &OpcoesIniciar) -> anyhow::Result<PathBuf> {
    if let Some(dir) = &opcoes.config_dir {
        if dir.as_os_str().is_empty() {
            bail!("o caminho informado em `--config-dir` não pode ser vazio");
        }
        return Ok(dir.clone());
    }
    FontesDaDefinicao::padrao_do_ambiente()
        .diretorio_config
        .context("não foi possível determinar o diretório de configuração; defina `XDG_CONFIG_HOME`/`HOME` ou informe `--config-dir`")
}

fn processar_usuario_toml(
    opcoes: &OpcoesIniciar,
    config_dir: &Path,
    usuario_path: &Path,
    interativo: bool,
) -> anyhow::Result<(AcaoConfig, Option<PathBuf>)> {
    if !usuario_path.exists() {
        let candidato = if interativo {
            coletar_interativo(opcoes, None)?
        } else {
            montar_novo_nao_interativo(opcoes)
        };
        let toml_novo = serializar_e_validar(&candidato)?;
        fs::create_dir_all(config_dir).with_context(|| {
            format!(
                "não foi possível criar o diretório `{}`",
                config_dir.display()
            )
        })?;
        fs::write(usuario_path, toml_novo)
            .with_context(|| format!("não foi possível gravar `{}`", usuario_path.display()))?;
        return Ok((AcaoConfig::Criado, None));
    }

    let conteudo_atual = fs::read_to_string(usuario_path)
        .with_context(|| format!("não foi possível ler `{}`", usuario_path.display()))?;

    match toml::from_str::<UsuarioParcial>(&conteudo_atual) {
        Ok(existente) => {
            let candidato = if interativo {
                coletar_interativo(opcoes, Some(&existente))?
            } else {
                aplicar_patch(&existente, opcoes)
            };

            let toml_novo = serializar_e_validar(&candidato)?;
            if candidato == existente {
                return Ok((AcaoConfig::Mantido, None));
            }

            if !autorizar_sobrescrita(opcoes, usuario_path, interativo)? {
                return Ok((AcaoConfig::Mantido, None));
            }

            let backup = criar_backup_numerado(usuario_path)?;
            fs::write(usuario_path, toml_novo).with_context(|| {
                format!("não foi possível atualizar `{}`", usuario_path.display())
            })?;
            Ok((AcaoConfig::Atualizado, Some(backup)))
        }
        Err(err_parse) => {
            let candidato = if interativo {
                eprintln!(
                    "Aviso: `{}` atual contém erro ({err_parse}) e será regenerado se você confirmar.",
                    usuario_path.display()
                );
                coletar_interativo(opcoes, None)?
            } else {
                montar_novo_nao_interativo(opcoes)
            };
            let toml_novo = serializar_e_validar(&candidato)?;

            if !opcoes.sobrescrever {
                if interativo {
                    let confirmou = confirmar_tty(&format!(
                        "Sobrescrever `{}` inválido gerando backup `.bak`? [s/N]: ",
                        usuario_path.display()
                    ))?;
                    if !confirmou {
                        bail!(
                            "`{}` está inválido ({err_parse}); corrija o arquivo ou autorize `--sobrescrever`",
                            usuario_path.display()
                        );
                    }
                } else {
                    bail!(
                        "`{}` existente está inválido ({err_parse}); passe `--sobrescrever` para regenerar gerando backup `.bak`",
                        usuario_path.display()
                    );
                }
            }

            let backup = criar_backup_numerado(usuario_path)?;
            fs::write(usuario_path, toml_novo)
                .with_context(|| format!("não foi possível gravar `{}`", usuario_path.display()))?;
            Ok((AcaoConfig::Atualizado, Some(backup)))
        }
    }
}

fn montar_novo_nao_interativo(opcoes: &OpcoesIniciar) -> UsuarioParcial {
    UsuarioParcial {
        tratamento: Some(
            opcoes
                .tratamento
                .clone()
                .unwrap_or_else(|| "senhor".to_owned()),
        ),
        idioma: Some(opcoes.idioma.clone().unwrap_or_else(|| "pt-BR".to_owned())),
        preferencias: if opcoes.preferencias.is_empty() {
            None
        } else {
            Some(opcoes.preferencias.clone())
        },
    }
}

fn aplicar_patch(existente: &UsuarioParcial, opcoes: &OpcoesIniciar) -> UsuarioParcial {
    UsuarioParcial {
        tratamento: opcoes
            .tratamento
            .clone()
            .or_else(|| existente.tratamento.clone()),
        idioma: opcoes.idioma.clone().or_else(|| existente.idioma.clone()),
        preferencias: if opcoes.preferencias.is_empty() {
            existente.preferencias.clone()
        } else {
            Some(opcoes.preferencias.clone())
        },
    }
}

fn coletar_interativo(
    opcoes: &OpcoesIniciar,
    existente: Option<&UsuarioParcial>,
) -> anyhow::Result<UsuarioParcial> {
    let mut err = std::io::stderr().lock();
    let _ = writeln!(err, "Configuração inicial da Sexta-feira");

    let tratamento_padrao = opcoes
        .tratamento
        .as_deref()
        .or_else(|| existente.and_then(|e| e.tratamento.as_deref()))
        .unwrap_or("senhor");
    let tratamento = if let Some(t) = &opcoes.tratamento {
        t.clone()
    } else {
        drop(err);
        let resp = ler_linha_tty(&format!(
            "Como deseja ser tratado? [Enter = \"{tratamento_padrao}\"]: "
        ))?;
        if resp.trim().is_empty() {
            tratamento_padrao.to_owned()
        } else {
            resp.trim().to_owned()
        }
    };

    let idioma_padrao = opcoes
        .idioma
        .as_deref()
        .or_else(|| existente.and_then(|e| e.idioma.as_deref()))
        .unwrap_or("pt-BR");
    let idioma = if let Some(i) = &opcoes.idioma {
        i.clone()
    } else {
        let resp = ler_linha_tty(&format!("Idioma preferido [Enter = \"{idioma_padrao}\"]: "))?;
        if resp.trim().is_empty() {
            idioma_padrao.to_owned()
        } else {
            resp.trim().to_owned()
        }
    };

    let preferencias = if opcoes.preferencias.is_empty() {
        let hint = if existente.and_then(|e| e.preferencias.as_ref()).is_some() {
            "Enter = manter atuais"
        } else {
            "Enter = herdar padrão embutido"
        };
        let resp = ler_linha_tty(&format!(
            "Preferências separadas por `;` (atenção: substitui a lista padrão) [{hint}]: "
        ))?;
        if resp.trim().is_empty() {
            existente.and_then(|e| e.preferencias.clone())
        } else {
            Some(resp.split(';').map(|s| s.trim().to_owned()).collect())
        }
    } else {
        Some(opcoes.preferencias.clone())
    };

    Ok(UsuarioParcial {
        tratamento: Some(tratamento),
        idioma: Some(idioma),
        preferencias,
    })
}

fn autorizar_sobrescrita(
    opcoes: &OpcoesIniciar,
    usuario_path: &Path,
    interativo: bool,
) -> anyhow::Result<bool> {
    if opcoes.sobrescrever {
        return Ok(true);
    }
    if !interativo {
        bail!(
            "`{}` já existe e há alterações pendentes; passe `--sobrescrever` para atualizar gerando backup `.bak`",
            usuario_path.display()
        );
    }
    confirmar_tty(&format!(
        "`{}` já existe. Atualizar gerando backup `.bak`? [s/N]: ",
        usuario_path.display()
    ))
}

fn serializar_e_validar(candidato: &UsuarioParcial) -> anyhow::Result<String> {
    let mut conteudo =
        toml::to_string_pretty(candidato).context("não foi possível serializar `usuario.toml`")?;
    if !conteudo.ends_with('\n') {
        conteudo.push('\n');
    }
    let tmp = TempValidacao::gravar(&conteudo)?;
    let fontes = FontesDaDefinicao {
        diretorio_config: None,
        persona_explicita: None,
        usuario_explicito: Some(tmp.caminho.clone()),
    };
    DefinicaoDoMordomo::carregar(&fontes)?;
    Ok(conteudo)
}

fn criar_backup_numerado(origem: &Path) -> anyhow::Result<PathBuf> {
    let mut indice = 0usize;
    loop {
        let sufixo = if indice == 0 {
            ".bak".to_owned()
        } else {
            format!(".bak.{indice}")
        };
        let mut candidato = origem.as_os_str().to_os_string();
        candidato.push(&sufixo);
        let destino = PathBuf::from(candidato);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destino)
        {
            Ok(_) => {
                fs::copy(origem, &destino).with_context(|| {
                    format!(
                        "não foi possível copiar backup para `{}`",
                        destino.display()
                    )
                })?;
                return Ok(destino);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                indice += 1;
            }
            Err(e) => {
                return Err(e).with_context(|| {
                    format!("não foi possível criar backup em `{}`", destino.display())
                });
            }
        }
    }
}

fn executar_etapa_harness(
    harness: Harness,
    config_dir: &Path,
    opcoes: &OpcoesIniciar,
    aplicar_agora: bool,
    nivel_verbose: u8,
) -> anyhow::Result<(Option<PlanoIniciarJson>, Option<AplicadoIniciarJson>)> {
    let adaptador = match harness {
        Harness::Hermes => {
            let config =
                ConfiguracaoHermes::resolver(opcoes.destino.as_deref(), opcoes.perfil.as_deref())?;
            AdaptadorHermes::new(config)
        }
        outro => return Err(ErroHarnessNaoImplementado { harness: outro }.into()),
    };

    let fontes = FontesDaDefinicao {
        diretorio_config: Some(config_dir.to_path_buf()),
        persona_explicita: None,
        usuario_explicito: None,
    };
    let (definicao, camadas) = DefinicaoDoMordomo::carregar_com_camadas(&fontes)?;
    verbose::emitir_contexto_definicao(nivel_verbose, &definicao, &camadas);

    let desejados = adaptador.planejar_arquivos(&definicao)?;
    let plano = planejar(desejados, adaptador.arquivo_estado())?;
    verbose::emitir_contexto_plano(nivel_verbose, &plano);

    let (mut criar, mut atualizar, mut conflitos) = (0usize, 0usize, 0usize);
    for acao in &plano.acoes {
        match acao.operacao {
            OperacaoPlanejada::Criar => criar += 1,
            OperacaoPlanejada::Atualizar => atualizar += 1,
            OperacaoPlanejada::Conflito { .. } => conflitos += 1,
            OperacaoPlanejada::SemAlteracoes | OperacaoPlanejada::Remover => {}
        }
    }

    if !aplicar_agora {
        return Ok((
            Some(PlanoIniciarJson {
                harness: adaptador.id(),
                criar,
                atualizar,
                conflitos,
            }),
            None,
        ));
    }

    let resultado = aplicar(&plano, opcoes.forcar)?;
    let desejados_pos = adaptador.planejar_arquivos(&definicao)?;
    let verif = verificar(&desejados_pos, &adaptador.arquivo_estado())?;
    verbose::emitir_contexto_verificacao(nivel_verbose, &verif);

    Ok((
        None,
        Some(AplicadoIniciarJson {
            harness: adaptador.id(),
            criados: resultado.criados,
            atualizados: resultado.atualizados,
            backups_criados: resultado
                .backups_criados
                .into_iter()
                .map(|p| p.display().to_string())
                .collect(),
            sincronizado: verif.sincronizado,
        }),
    ))
}

fn perguntar_harness_interativo() -> anyhow::Result<(Option<Harness>, bool)> {
    let resp = ler_linha_tty("Deseja aplicar agora no harness `hermes`? [s/N]: ")?;
    let sim = matches!(
        resp.trim().to_lowercase().as_str(),
        "s" | "sim" | "y" | "yes"
    );
    if sim {
        Ok((Some(Harness::Hermes), true))
    } else {
        Ok((None, false))
    }
}

fn ler_linha_tty(prompt: &str) -> anyhow::Result<String> {
    let mut err = std::io::stderr().lock();
    write!(err, "{prompt}")?;
    err.flush()?;
    drop(err);

    let mut linha = String::new();
    std::io::stdin().lock().read_line(&mut linha)?;
    Ok(linha)
}

fn confirmar_tty(prompt: &str) -> anyhow::Result<bool> {
    let resp = ler_linha_tty(prompt)?;
    Ok(matches!(
        resp.trim().to_lowercase().as_str(),
        "s" | "sim" | "y" | "yes"
    ))
}

fn imprimir_resumo_texto(
    saida: &mut impl Write,
    usuario_path: &Path,
    acao: AcaoConfig,
    backup: Option<&Path>,
    harness: Option<Harness>,
    plano: Option<&PlanoIniciarJson>,
    aplicado: Option<&AplicadoIniciarJson>,
) -> anyhow::Result<()> {
    let rotulo = match acao {
        AcaoConfig::Criado => "criado",
        AcaoConfig::Atualizado => "atualizado",
        AcaoConfig::Mantido => "mantido",
    };
    writeln!(
        saida,
        "Arquivo de perfil {rotulo}: {}",
        usuario_path.display()
    )?;
    if let Some(bak) = backup {
        writeln!(saida, "Backup criado: {}", bak.display())?;
    }

    if let Some(ap) = aplicado {
        writeln!(
            saida,
            "Setup aplicado em `{}`: {} criado(s), {} atualizado(s), sincronizado={}",
            ap.harness, ap.criados, ap.atualizados, ap.sincronizado
        )?;
    } else if let Some(pl) = plano {
        writeln!(
            saida,
            "Plano para `{}`: {} criar, {} atualizar, {} conflitos",
            pl.harness, pl.criar, pl.atualizar, pl.conflitos
        )?;
        writeln!(
            saida,
            "Próximo passo: execute `sexta-feira aplicar {}`.",
            pl.harness
        )?;
    } else if harness.is_none() {
        writeln!(
            saida,
            "Próximos passos:\n  1. Revise o plano com: sexta-feira plano hermes\n  2. Aplique com: sexta-feira aplicar hermes"
        )?;
    }
    Ok(())
}
