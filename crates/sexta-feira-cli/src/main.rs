//! CLI da Sexta-feira.

use std::io::Write;
use std::path::PathBuf;

use anyhow::bail;
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use sexta_feira_adapters::{AdaptadorHermes, ConfiguracaoHermes, Harness};
use sexta_feira_core::{
    AdaptadorDeSetup, DefinicaoDoMordomo, FontesDaDefinicao, OperacaoPlanejada, PlanoDeSetup,
    aplicar, planejar, verificar,
};
use tracing_subscriber::EnvFilter;

/// Sexta-feira: setup declarativo do seu mordomo virtual.
#[derive(Debug, Parser)]
#[command(name = "sexta-feira", version, about)]
struct Cli {
    /// Emite saída estruturada em JSON.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    comando: Comando,
}

#[derive(Debug, Subcommand)]
enum Comando {
    /// Lista os harnesses previstos.
    Harnesses,
    /// Exibe o plano de mudanças sem alterar o disco.
    Plano(OpcoesAlvo),
    /// Aplica o setup declarativo no harness escolhido.
    Aplicar {
        #[command(flatten)]
        alvo: OpcoesAlvo,
        /// Substitui arquivos integrais em conflito gerando backup `.bak`.
        #[arg(long)]
        forcar: bool,
    },
    /// Verifica se o setup no harness está sincronizado.
    Verificar(OpcoesAlvo),
}

#[derive(Debug, Clone, Args)]
struct OpcoesAlvo {
    /// Identificador do harness (ex.: `hermes`).
    harness: Harness,
    /// Caminho explícito para o diretório base de destino do harness.
    #[arg(long, alias = "hermes-home")]
    destino: Option<PathBuf>,
    /// Nome do perfil isolado no harness (padrão: `sexta-feira`).
    #[arg(long)]
    perfil: Option<String>,
    /// Arquivo TOML explícito com sobrescritas da persona.
    #[arg(long)]
    persona: Option<PathBuf>,
    /// Arquivo TOML explícito com sobrescritas do perfil do usuário.
    #[arg(long)]
    usuario: Option<PathBuf>,
}

#[derive(Serialize)]
struct HarnessJson {
    id: &'static str,
    disponivel: bool,
    status: &'static str,
}

#[derive(Serialize)]
struct AcaoJson<'a> {
    caminho: String,
    operacao: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    motivo: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<&'a str>,
}

#[derive(Default, Serialize)]
struct ResumoPlano {
    criar: usize,
    atualizar: usize,
    sem_alteracoes: usize,
    remover: usize,
    conflitos: usize,
}

#[derive(Serialize)]
struct PlanoJson<'a> {
    harness: &'static str,
    acoes: Vec<AcaoJson<'a>>,
    resumo: ResumoPlano,
}

#[derive(Serialize)]
struct AplicarJson {
    harness: &'static str,
    criados: usize,
    atualizados: usize,
    backups_criados: Vec<String>,
}

#[derive(Serialize)]
struct ItemVerificacaoJson<'a> {
    caminho: String,
    sincronizado: bool,
    detalhe: &'a str,
}

#[derive(Serialize)]
struct VerificarJson<'a> {
    harness: &'static str,
    sincronizado: bool,
    itens: Vec<ItemVerificacaoJson<'a>>,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("SEXTA_FEIRA_LOG").unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let json = cli.json;

    match cli.comando {
        Comando::Harnesses => {
            tracing::debug!(json, "executando subcomando harnesses");
            listar_harnesses(json)?;
        }
        Comando::Plano(alvo) => {
            tracing::debug!(harness = %alvo.harness, json, "executando subcomando plano");
            executar_plano(&alvo, json)?;
        }
        Comando::Aplicar { alvo, forcar } => {
            tracing::debug!(harness = %alvo.harness, forcar, json, "executando subcomando aplicar");
            executar_aplicar(&alvo, forcar, json)?;
        }
        Comando::Verificar(alvo) => {
            tracing::debug!(harness = %alvo.harness, json, "executando subcomando verificar");
            executar_verificar(&alvo, json)?;
        }
    }
    Ok(())
}

fn listar_harnesses(json: bool) -> anyhow::Result<()> {
    let mut saida = std::io::stdout().lock();
    if json {
        let itens: Vec<HarnessJson> = Harness::TODOS
            .into_iter()
            .map(|h| HarnessJson {
                id: h.id(),
                disponivel: h.disponivel(),
                status: h.rotulo_status(),
            })
            .collect();
        escrever_json(&mut saida, &itens)?;
        return Ok(());
    }

    for harness in Harness::TODOS {
        writeln!(saida, "{:<14}  {}", harness.id(), harness.rotulo_status())?;
    }
    Ok(())
}

fn resolver_adaptador(alvo: &OpcoesAlvo) -> anyhow::Result<Box<dyn AdaptadorDeSetup>> {
    match alvo.harness {
        Harness::Hermes => {
            let config =
                ConfiguracaoHermes::resolver(alvo.destino.as_deref(), alvo.perfil.as_deref())?;
            Ok(Box::new(AdaptadorHermes::new(config)))
        }
        outro => bail!("adaptador para `{outro}` ainda não implementado"),
    }
}

fn carregar_contexto(
    alvo: &OpcoesAlvo,
) -> anyhow::Result<(DefinicaoDoMordomo, Box<dyn AdaptadorDeSetup>)> {
    let mut fontes = FontesDaDefinicao::padrao_do_ambiente();
    if alvo.persona.is_some() {
        fontes.persona_explicita.clone_from(&alvo.persona);
    }
    if alvo.usuario.is_some() {
        fontes.usuario_explicito.clone_from(&alvo.usuario);
    }
    let definicao = DefinicaoDoMordomo::carregar(&fontes)?;
    let adaptador = resolver_adaptador(alvo)?;
    Ok((definicao, adaptador))
}

fn contabilizar_plano(plano: &PlanoDeSetup) -> ResumoPlano {
    let mut resumo = ResumoPlano::default();
    for acao in &plano.acoes {
        match &acao.operacao {
            OperacaoPlanejada::Criar => resumo.criar += 1,
            OperacaoPlanejada::Atualizar => resumo.atualizar += 1,
            OperacaoPlanejada::SemAlteracoes => resumo.sem_alteracoes += 1,
            OperacaoPlanejada::Remover => resumo.remover += 1,
            OperacaoPlanejada::Conflito { .. } => resumo.conflitos += 1,
        }
    }
    resumo
}

fn executar_plano(alvo: &OpcoesAlvo, json: bool) -> anyhow::Result<()> {
    let (definicao, adaptador) = carregar_contexto(alvo)?;
    let desejados = adaptador.planejar_arquivos(&definicao)?;
    let plano = planejar(desejados, adaptador.arquivo_estado())?;
    let resumo = contabilizar_plano(&plano);

    let mut saida = std::io::stdout().lock();
    if json {
        let acoes = plano
            .acoes
            .iter()
            .map(|acao| {
                let (operacao, motivo) = match &acao.operacao {
                    OperacaoPlanejada::Criar => ("criar", None),
                    OperacaoPlanejada::Atualizar => ("atualizar", None),
                    OperacaoPlanejada::SemAlteracoes => ("sem_alteracoes", None),
                    OperacaoPlanejada::Remover => ("remover", None),
                    OperacaoPlanejada::Conflito { motivo } => ("conflito", Some(motivo.as_str())),
                };
                AcaoJson {
                    caminho: acao.caminho.display().to_string(),
                    operacao,
                    motivo,
                    diff: acao.resumo_diff.as_deref(),
                }
            })
            .collect();

        escrever_json(
            &mut saida,
            &PlanoJson {
                harness: adaptador.id(),
                acoes,
                resumo,
            },
        )?;
        return Ok(());
    }

    writeln!(saida, "Plano para {}:", adaptador.id())?;
    for acao in &plano.acoes {
        let tag = match &acao.operacao {
            OperacaoPlanejada::Criar => "CRIAR",
            OperacaoPlanejada::Atualizar => "ATUALIZAR",
            OperacaoPlanejada::SemAlteracoes => "OK",
            OperacaoPlanejada::Remover => "REMOVER",
            OperacaoPlanejada::Conflito { .. } => "CONFLITO",
        };

        writeln!(saida, "[{tag}] {}", acao.caminho.display())?;
        if let OperacaoPlanejada::Conflito { motivo } = &acao.operacao {
            writeln!(saida, "  motivo: {motivo}")?;
        }
        if let Some(diff) = &acao.resumo_diff {
            for linha in diff.lines() {
                writeln!(saida, "  {linha}")?;
            }
        }
    }

    if resumo.remover > 0 {
        writeln!(
            saida,
            "Resumo: {} criar, {} atualizar, {} sem alterações, {} remover, {} conflitos",
            resumo.criar, resumo.atualizar, resumo.sem_alteracoes, resumo.remover, resumo.conflitos
        )?;
    } else {
        writeln!(
            saida,
            "Resumo: {} criar, {} atualizar, {} sem alterações, {} conflitos",
            resumo.criar, resumo.atualizar, resumo.sem_alteracoes, resumo.conflitos
        )?;
    }
    Ok(())
}

fn executar_aplicar(alvo: &OpcoesAlvo, forcar: bool, json: bool) -> anyhow::Result<()> {
    let (definicao, adaptador) = carregar_contexto(alvo)?;
    let desejados = adaptador.planejar_arquivos(&definicao)?;
    let plano = planejar(desejados, adaptador.arquivo_estado())?;
    let resultado = aplicar(&plano, forcar)?;

    let mut saida = std::io::stdout().lock();
    if json {
        let backups_criados = resultado
            .backups_criados
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        escrever_json(
            &mut saida,
            &AplicarJson {
                harness: adaptador.id(),
                criados: resultado.criados,
                atualizados: resultado.atualizados,
                backups_criados,
            },
        )?;
        return Ok(());
    }

    writeln!(
        saida,
        "Setup aplicado para {}: {} criado(s), {} atualizado(s), {} backup(s)",
        adaptador.id(),
        resultado.criados,
        resultado.atualizados,
        resultado.backups_criados.len()
    )?;
    for backup in &resultado.backups_criados {
        writeln!(saida, "Backup criado: {}", backup.display())?;
    }
    Ok(())
}

fn executar_verificar(alvo: &OpcoesAlvo, json: bool) -> anyhow::Result<()> {
    let (definicao, adaptador) = carregar_contexto(alvo)?;
    let desejados = adaptador.planejar_arquivos(&definicao)?;
    let resultado = verificar(&desejados, &adaptador.arquivo_estado())?;

    let mut saida = std::io::stdout().lock();
    if json {
        let itens = resultado
            .itens
            .iter()
            .map(|item| ItemVerificacaoJson {
                caminho: item.caminho.display().to_string(),
                sincronizado: item.sincronizado,
                detalhe: item.detalhe.as_str(),
            })
            .collect();
        escrever_json(
            &mut saida,
            &VerificarJson {
                harness: adaptador.id(),
                sincronizado: resultado.sincronizado,
                itens,
            },
        )?;
    } else {
        for item in &resultado.itens {
            let tag = if item.sincronizado { "OK" } else { "PENDENTE" };
            writeln!(
                saida,
                "[{tag}] {} ({})",
                item.caminho.display(),
                item.detalhe
            )?;
        }
    }
    drop(saida);

    if !resultado.sincronizado {
        if !json {
            let _ = writeln!(
                std::io::stderr(),
                "Dica: execute `sexta-feira aplicar {}` para sincronizar.",
                alvo.harness
            );
        }
        bail!(
            "setup de `{}` possui pendências ou divergências",
            alvo.harness
        );
    }
    Ok(())
}

fn escrever_json<T: Serialize + ?Sized>(saida: &mut impl Write, valor: &T) -> anyhow::Result<()> {
    serde_json::to_writer_pretty(&mut *saida, valor)?;
    writeln!(saida)?;
    Ok(())
}
