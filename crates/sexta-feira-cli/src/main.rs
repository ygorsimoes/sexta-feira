//! CLI da Sexta-feira.

mod diagnostico;
mod iniciar;
mod verbose;

use std::io::Write;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use sexta_feira_adapters::{
    AdaptadorHermes, ConfiguracaoHermes, ErroDeHermes, Harness, OrigemResolucaoDestino,
};
use sexta_feira_core::{
    AdaptadorDeSetup, CamadaDeDefinicao, DefinicaoDoMordomo, ErroDeDefinicao, ErroDePlano,
    FontesDaDefinicao, OperacaoPlanejada, PlanoDeSetup, aplicar, planejar, verificar,
};

const AJUDA_RAIZ: &str = "\
Exemplos:
  sexta-feira iniciar
  sexta-feira diagnostico
  sexta-feira harnesses
  sexta-feira plano hermes
  sexta-feira aplicar hermes
  sexta-feira verificar hermes
  sexta-feira -vv plano hermes --json";

/// Sexta-feira: setup declarativo do seu mordomo virtual.
#[derive(Debug, Parser)]
#[command(name = "sexta-feira", version, about, after_help = AJUDA_RAIZ)]
struct Cli {
    /// Emite saída estruturada em JSON.
    #[arg(long, global = true)]
    json: bool,
    /// Aumenta o nível de contexto diagnóstico em `stderr` (`-v`, `-vv`, `-vvv`).
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true)]
    verbose: u8,
    #[command(subcommand)]
    comando: Comando,
}

#[derive(Debug, Subcommand)]
enum Comando {
    /// Guia a criação/atualização de `usuario.toml` e opcionalmente aplica o setup.
    #[command(after_help = "Exemplo: sexta-feira iniciar --tratamento \"senhor\" --nao-interativo")]
    Iniciar(iniciar::OpcoesIniciar),
    /// Lista os harnesses previstos.
    Harnesses,
    /// Inspeciona ambiente, camadas de configuração e harnesses.
    #[command(after_help = "Exemplo: sexta-feira diagnostico --json")]
    Diagnostico(diagnostico::OpcoesDiagnostico),
    /// Exibe o plano de mudanças sem alterar o disco.
    #[command(after_help = "Exemplo: sexta-feira plano hermes --perfil sexta-feira")]
    Plano(OpcoesAlvo),
    /// Aplica o setup declarativo no harness escolhido.
    #[command(after_help = "Exemplo: sexta-feira aplicar hermes")]
    Aplicar {
        #[command(flatten)]
        alvo: OpcoesAlvo,
        /// Substitui arquivos integrais em conflito gerando backup `.bak`.
        #[arg(long)]
        forcar: bool,
    },
    /// Verifica se o setup no harness está sincronizado.
    #[command(after_help = "Exemplo: sexta-feira verificar hermes")]
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

struct ContextoAlvo {
    definicao: DefinicaoDoMordomo,
    camadas: Vec<CamadaDeDefinicao>,
    fontes: FontesDaDefinicao,
    adaptador: Box<dyn AdaptadorDeSetup>,
    destino_base: PathBuf,
    origem_destino: OrigemResolucaoDestino,
    perfil_dir: PathBuf,
}

#[derive(Debug)]
struct ErroHarnessNaoImplementado {
    harness: Harness,
}

impl std::fmt::Display for ErroHarnessNaoImplementado {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "adaptador para `{}` ainda não implementado (status: em breve)",
            self.harness
        )
    }
}

impl std::error::Error for ErroHarnessNaoImplementado {}

#[derive(Debug)]
struct ErroVerificacaoPendente {
    harness: Harness,
}

impl std::fmt::Display for ErroVerificacaoPendente {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "setup de `{}` possui pendências ou divergências",
            self.harness
        )
    }
}

impl std::error::Error for ErroVerificacaoPendente {}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    verbose::inicializar_tracing(cli.verbose);

    if let Err(erro) = executar(cli) {
        let mut err = std::io::stderr().lock();
        let _ = writeln!(err, "Erro: {erro:#}");
        if let Some(dica) = sugerir_dica(&erro) {
            let _ = writeln!(err, "Dica: {dica}");
        }
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

fn executar(cli: Cli) -> anyhow::Result<()> {
    let json = cli.json;
    let nivel_verbose = cli.verbose;

    match cli.comando {
        Comando::Iniciar(opcoes) => {
            verbose::emitir_ambiente(nivel_verbose, "iniciar");
            tracing::debug!(json, "executando subcomando iniciar");
            iniciar::executar(&opcoes, json, nivel_verbose)?;
        }
        Comando::Harnesses => {
            verbose::emitir_ambiente(nivel_verbose, "harnesses");
            tracing::debug!(json, "executando subcomando harnesses");
            listar_harnesses(json)?;
        }
        Comando::Diagnostico(opcoes) => {
            verbose::emitir_ambiente(nivel_verbose, "diagnostico");
            tracing::debug!(harness = ?opcoes.harness, json, "executando subcomando diagnostico");
            diagnostico::executar(&opcoes, json, nivel_verbose)?;
        }
        Comando::Plano(alvo) => {
            verbose::emitir_ambiente(nivel_verbose, "plano");
            tracing::debug!(harness = %alvo.harness, json, "executando subcomando plano");
            executar_plano(&alvo, json, nivel_verbose)?;
        }
        Comando::Aplicar { alvo, forcar } => {
            verbose::emitir_ambiente(nivel_verbose, "aplicar");
            tracing::debug!(harness = %alvo.harness, forcar, json, "executando subcomando aplicar");
            executar_aplicar(&alvo, forcar, json, nivel_verbose)?;
        }
        Comando::Verificar(alvo) => {
            verbose::emitir_ambiente(nivel_verbose, "verificar");
            tracing::debug!(harness = %alvo.harness, json, "executando subcomando verificar");
            executar_verificar(&alvo, json, nivel_verbose)?;
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

fn resolver_adaptador(
    alvo: &OpcoesAlvo,
) -> anyhow::Result<(
    Box<dyn AdaptadorDeSetup>,
    PathBuf,
    OrigemResolucaoDestino,
    PathBuf,
)> {
    match alvo.harness {
        Harness::Hermes => {
            let env_hermes_home = std::env::var("HERMES_HOME").ok();
            let env_home = std::env::var("HOME").ok();
            let (config, origem) = ConfiguracaoHermes::resolver_com_origem(
                alvo.destino.as_deref(),
                env_hermes_home.as_deref(),
                env_home.as_deref(),
                alvo.perfil.as_deref(),
            )?;
            let base = config.hermes_home.clone();
            let perfil_dir = config.diretorio_do_perfil();
            Ok((
                Box::new(AdaptadorHermes::new(config)),
                base,
                origem,
                perfil_dir,
            ))
        }
        outro => Err(ErroHarnessNaoImplementado { harness: outro }.into()),
    }
}

fn carregar_contexto(alvo: &OpcoesAlvo, nivel_verbose: u8) -> anyhow::Result<ContextoAlvo> {
    let (adaptador, destino_base, origem_destino, perfil_dir) = resolver_adaptador(alvo)?;
    let mut fontes = FontesDaDefinicao::padrao_do_ambiente();
    if alvo.persona.is_some() {
        fontes.persona_explicita.clone_from(&alvo.persona);
    }
    if alvo.usuario.is_some() {
        fontes.usuario_explicito.clone_from(&alvo.usuario);
    }
    let (definicao, camadas) = DefinicaoDoMordomo::carregar_com_camadas(&fontes)?;
    verbose::emitir_contexto_definicao(nivel_verbose, &definicao, &camadas);
    verbose::emitir_contexto_destino(
        nivel_verbose,
        adaptador.id(),
        &destino_base,
        origem_destino,
        &perfil_dir,
        &adaptador.arquivo_estado(),
    );
    Ok(ContextoAlvo {
        definicao,
        camadas,
        fontes,
        adaptador,
        destino_base,
        origem_destino,
        perfil_dir,
    })
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

fn executar_plano(alvo: &OpcoesAlvo, json: bool, nivel_verbose: u8) -> anyhow::Result<()> {
    let ctx = carregar_contexto(alvo, nivel_verbose)?;
    let desejados = ctx.adaptador.planejar_arquivos(&ctx.definicao)?;
    let plano = planejar(desejados, ctx.adaptador.arquivo_estado())?;
    verbose::emitir_contexto_plano(nivel_verbose, &plano);
    let resumo = contabilizar_plano(&plano);
    let _ = (
        &ctx.camadas,
        &ctx.fontes,
        &ctx.destino_base,
        ctx.origem_destino,
    );

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
                harness: ctx.adaptador.id(),
                acoes,
                resumo,
            },
        )?;
        return Ok(());
    }

    writeln!(
        saida,
        "Plano para {} (destino: {}):",
        ctx.adaptador.id(),
        ctx.perfil_dir.display()
    )?;
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

    if resumo.conflitos > 0 {
        writeln!(
            saida,
            "Dica: há conflitos; revise o diff acima ou rode `sexta-feira aplicar {} --forcar` para sobrescrever gerando backup `.bak`.",
            ctx.adaptador.id()
        )?;
    } else if resumo.criar > 0 || resumo.atualizar > 0 || resumo.remover > 0 {
        writeln!(
            saida,
            "Dica: execute `sexta-feira aplicar {}` para aplicar as mudanças.",
            ctx.adaptador.id()
        )?;
    }
    Ok(())
}

fn executar_aplicar(
    alvo: &OpcoesAlvo,
    forcar: bool,
    json: bool,
    nivel_verbose: u8,
) -> anyhow::Result<()> {
    let ctx = carregar_contexto(alvo, nivel_verbose)?;
    let desejados = ctx.adaptador.planejar_arquivos(&ctx.definicao)?;
    let plano = planejar(desejados, ctx.adaptador.arquivo_estado())?;
    verbose::emitir_contexto_plano(nivel_verbose, &plano);
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
                harness: ctx.adaptador.id(),
                criados: resultado.criados,
                atualizados: resultado.atualizados,
                backups_criados,
            },
        )?;
        return Ok(());
    }

    writeln!(
        saida,
        "Setup aplicado para {} em {}: {} criado(s), {} atualizado(s), {} backup(s)",
        ctx.adaptador.id(),
        ctx.perfil_dir.display(),
        resultado.criados,
        resultado.atualizados,
        resultado.backups_criados.len()
    )?;
    for backup in &resultado.backups_criados {
        writeln!(saida, "Backup criado: {}", backup.display())?;
    }
    writeln!(
        saida,
        "Próximos passos:\n  1. Valide com: sexta-feira verificar {}",
        ctx.adaptador.id()
    )?;
    if let Some(dir_cfg) = ctx.fontes.diretorio_config.as_deref() {
        writeln!(
            saida,
            "  2. Personalize preferências em: {}",
            dir_cfg.join("usuario.toml").display()
        )?;
    }
    Ok(())
}

fn executar_verificar(alvo: &OpcoesAlvo, json: bool, nivel_verbose: u8) -> anyhow::Result<()> {
    let ctx = carregar_contexto(alvo, nivel_verbose)?;
    let desejados = ctx.adaptador.planejar_arquivos(&ctx.definicao)?;
    let resultado = verificar(&desejados, &ctx.adaptador.arquivo_estado())?;
    verbose::emitir_contexto_verificacao(nivel_verbose, &resultado);

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
                harness: ctx.adaptador.id(),
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
        let sincronizados = resultado.itens.iter().filter(|i| i.sincronizado).count();
        writeln!(
            saida,
            "Resumo: {sincronizados}/{} item(ns) sincronizado(s)",
            resultado.itens.len()
        )?;
    }
    drop(saida);

    if !resultado.sincronizado {
        return Err(ErroVerificacaoPendente {
            harness: alvo.harness,
        }
        .into());
    }
    Ok(())
}

fn sugerir_dica(erro: &anyhow::Error) -> Option<String> {
    if erro.downcast_ref::<ErroHarnessNaoImplementado>().is_some() {
        return Some(
            "rode `sexta-feira harnesses` para ver o status atual ou peça ao seu agente para seguir a instalação assistida em `recursos/skills/instalar-sexta-feira/SKILL.md`.".to_owned(),
        );
    }
    if let Some(verif) = erro.downcast_ref::<ErroVerificacaoPendente>() {
        return Some(format!(
            "execute `sexta-feira aplicar {}` para sincronizar.",
            verif.harness
        ));
    }
    if let Some(ErroDePlano::ConflitoPendente { .. }) = erro.downcast_ref::<ErroDePlano>() {
        return Some(
            "rode `sexta-feira plano <harness>` para inspecionar o diff ou use `sexta-feira aplicar <harness> --forcar` para sobrescrever gerando backup `.bak`.".to_owned(),
        );
    }
    if let Some(err_def) = erro.downcast_ref::<ErroDeDefinicao>() {
        return match err_def {
            ErroDeDefinicao::ArquivoNaoEncontrado { .. } => Some(
                "confira o caminho informado em `--persona`/`--usuario` ou crie a configuração padrão no diretório `${XDG_CONFIG_HOME:-~/.config}/sexta-feira/`.".to_owned(),
            ),
            ErroDeDefinicao::ConfiguracaoUsuarioInvalida { .. }
            | ErroDeDefinicao::ConfiguracaoPersonaInvalida { .. }
            | ErroDeDefinicao::TomlUsuarioInvalido(_)
            | ErroDeDefinicao::TomlPersonaInvalido(_)
            | ErroDeDefinicao::CampoObrigatorioVazio { .. }
            | ErroDeDefinicao::ListaObrigatoriaVazia { .. } => Some(
                "chaves válidas de `usuario.toml` (`tratamento`, `idioma`, `preferencias`) e `persona.toml` (`nome`, `arquetipo`, `idioma`, `resumo`, `tom`, `diretrizes`, `limites`) não aceitam campos extras ou vazios. Use `-vv` ou `diagnostico` para inspecionar as camadas.".to_owned(),
            ),
        };
    }
    if let Some(ErroDeHermes::DiretorioBaseIndisponivel) = erro.downcast_ref::<ErroDeHermes>() {
        return Some(
            "informe `--destino <dir>` (ou `--hermes-home <dir>`) ou defina `HERMES_HOME`/`HOME`."
                .to_owned(),
        );
    }
    None
}

pub(crate) fn escrever_json<T: Serialize + ?Sized>(
    saida: &mut impl Write,
    valor: &T,
) -> anyhow::Result<()> {
    serde_json::to_writer_pretty(&mut *saida, valor)?;
    writeln!(saida)?;
    Ok(())
}
