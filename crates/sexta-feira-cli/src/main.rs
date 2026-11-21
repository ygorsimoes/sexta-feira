//! CLI da Sexta-feira.

use std::io::Write;
use std::path::PathBuf;

use anyhow::bail;
use clap::{Args, Parser, Subcommand};
use sexta_feira_adapters::{AdaptadorHermes, ConfiguracaoHermes, Harness};
use sexta_feira_core::{
    AdaptadorDeSetup, DefinicaoDoMordomo, OperacaoPlanejada, aplicar, planejar, verificar,
};
use tracing_subscriber::EnvFilter;

/// Sexta-feira: setup declarativo do seu mordomo virtual.
#[derive(Debug, Parser)]
#[command(name = "sexta-feira", version, about)]
struct Cli {
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
    /// Caminho explícito para o diretório base do Hermes (`HERMES_HOME`).
    #[arg(long)]
    hermes_home: Option<PathBuf>,
    /// Nome do perfil isolado no harness (padrão: `sexta-feira`).
    #[arg(long)]
    perfil: Option<String>,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("SEXTA_FEIRA_LOG").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    match Cli::parse().comando {
        Comando::Harnesses => listar_harnesses()?,
        Comando::Plano(alvo) => executar_plano(&alvo)?,
        Comando::Aplicar { alvo, forcar } => executar_aplicar(&alvo, forcar)?,
        Comando::Verificar(alvo) => executar_verificar(&alvo)?,
    }
    Ok(())
}

fn listar_harnesses() -> anyhow::Result<()> {
    let mut saida = std::io::stdout().lock();
    for harness in Harness::TODOS {
        writeln!(saida, "{harness}")?;
    }
    Ok(())
}

fn resolver_adaptador(alvo: &OpcoesAlvo) -> anyhow::Result<Box<dyn AdaptadorDeSetup>> {
    match alvo.harness {
        Harness::Hermes => {
            let config =
                ConfiguracaoHermes::resolver(alvo.hermes_home.as_deref(), alvo.perfil.as_deref())?;
            Ok(Box::new(AdaptadorHermes::new(config)))
        }
        outro => bail!("adaptador para `{outro}` ainda não implementado"),
    }
}

fn executar_plano(alvo: &OpcoesAlvo) -> anyhow::Result<()> {
    let definicao = DefinicaoDoMordomo::padrao()?;
    let adaptador = resolver_adaptador(alvo)?;
    let desejados = adaptador.planejar_arquivos(&definicao)?;
    let plano = planejar(desejados, adaptador.arquivo_estado())?;

    let mut criar = 0usize;
    let mut atualizar = 0usize;
    let mut sem_alteracoes = 0usize;
    let mut conflitos = 0usize;

    let mut saida = std::io::stdout().lock();
    writeln!(saida, "Plano para {}:", adaptador.id())?;

    for acao in &plano.acoes {
        let tag = match &acao.operacao {
            OperacaoPlanejada::Criar => {
                criar += 1;
                "CRIAR"
            }
            OperacaoPlanejada::Atualizar => {
                atualizar += 1;
                "ATUALIZAR"
            }
            OperacaoPlanejada::SemAlteracoes => {
                sem_alteracoes += 1;
                "OK"
            }
            OperacaoPlanejada::Remover => "REMOVER",
            OperacaoPlanejada::Conflito { .. } => {
                conflitos += 1;
                "CONFLITO"
            }
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

    writeln!(
        saida,
        "Resumo: {criar} criar, {atualizar} atualizar, {sem_alteracoes} sem alterações, {conflitos} conflitos"
    )?;
    Ok(())
}

fn executar_aplicar(alvo: &OpcoesAlvo, forcar: bool) -> anyhow::Result<()> {
    let definicao = DefinicaoDoMordomo::padrao()?;
    let adaptador = resolver_adaptador(alvo)?;
    let desejados = adaptador.planejar_arquivos(&definicao)?;
    let plano = planejar(desejados, adaptador.arquivo_estado())?;
    let resultado = aplicar(&plano, forcar)?;

    let mut saida = std::io::stdout().lock();
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

fn executar_verificar(alvo: &OpcoesAlvo) -> anyhow::Result<()> {
    let definicao = DefinicaoDoMordomo::padrao()?;
    let adaptador = resolver_adaptador(alvo)?;
    let desejados = adaptador.planejar_arquivos(&definicao)?;
    let resultado = verificar(&desejados, &adaptador.arquivo_estado())?;

    let mut saida = std::io::stdout().lock();
    for item in &resultado.itens {
        let tag = if item.sincronizado { "OK" } else { "PENDENTE" };
        writeln!(
            saida,
            "[{tag}] {} ({})",
            item.caminho.display(),
            item.detalhe
        )?;
    }

    if !resultado.sincronizado {
        bail!(
            "setup de `{}` possui pendências ou divergências",
            alvo.harness
        );
    }
    Ok(())
}
