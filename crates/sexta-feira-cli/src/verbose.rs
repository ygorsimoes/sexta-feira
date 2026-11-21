//! Suporte a modo `--verbose` (`-v`, `-vv`, `-vvv`) e inicialização de `tracing` em `stderr`.

use std::io::Write as _;
use std::path::Path;

use sexta_feira_adapters::OrigemResolucaoDestino;
use sexta_feira_core::{
    CamadaDeDefinicao, DefinicaoDoMordomo, EstrategiaDeEscrita, FormatoEstruturado,
    OperacaoPlanejada, OrigemDaCamada, PermissaoDeArquivo, PlanoDeSetup, ResultadoDaVerificacao,
    StatusDaCamada,
};
use tracing_subscriber::EnvFilter;

/// Inicializa o assinante de logs em `stderr`, priorizando `SEXTA_FEIRA_LOG`.
pub fn inicializar_tracing(verbose: u8) {
    let filtro = std::env::var("SEXTA_FEIRA_LOG")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .map_or_else(
            || {
                let nivel = match verbose {
                    0 => "warn",
                    1 => "info",
                    2 => "debug",
                    _ => "trace",
                };
                EnvFilter::new(nivel)
            },
            EnvFilter::new,
        );

    tracing_subscriber::fmt()
        .with_env_filter(filtro)
        .with_writer(std::io::stderr)
        .init();
}

/// Emite informações básicas de execução (`-v`) e variáveis de ambiente (`-vv`).
pub fn emitir_ambiente(verbose: u8, comando: &str) {
    if verbose == 0 {
        return;
    }

    let mut err = std::io::stderr().lock();
    let _ = writeln!(
        err,
        "[verbose] comando={comando} versao={} plataforma={}/{}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    if verbose >= 2 {
        let cwd = std::env::current_dir()
            .map_or_else(|_| "<indisponível>".to_owned(), |p| p.display().to_string());
        let home = ler_env_opcional("HOME");
        let xdg = ler_env_opcional("XDG_CONFIG_HOME");
        let hermes_home = ler_env_opcional("HERMES_HOME");
        let log_env = ler_env_opcional("SEXTA_FEIRA_LOG");
        let _ = writeln!(
            err,
            "[verbose] ambiente: cwd={cwd} HOME={home} XDG_CONFIG_HOME={xdg} HERMES_HOME={hermes_home} SEXTA_FEIRA_LOG={log_env}"
        );
    }
}

/// Emite as camadas avaliadas e o resumo da definição carregada (`-v`).
pub fn emitir_contexto_definicao(
    verbose: u8,
    definicao: &DefinicaoDoMordomo,
    camadas: &[CamadaDeDefinicao],
) {
    if verbose == 0 {
        return;
    }

    let mut err = std::io::stderr().lock();
    let _ = writeln!(err, "[verbose] camadas de configuração:");
    for camada in camadas {
        let origem = match camada.origem {
            OrigemDaCamada::Embutida => "embutida",
            OrigemDaCamada::DiretorioConfig => "diretorio_config",
            OrigemDaCamada::ArquivoExplicito => "arquivo_explicito",
        };
        let status = match camada.status {
            StatusDaCamada::Aplicada => "aplicada",
            StatusDaCamada::Ausente => "ausente",
        };
        let caminho = camada
            .caminho
            .as_ref()
            .map_or_else(|| "<embutido>".to_owned(), |p| p.display().to_string());
        let _ = writeln!(
            err,
            "[verbose]   - secao={} origem={origem} status={status} caminho={caminho}",
            camada.secao
        );
    }

    let _ = writeln!(
        err,
        "[verbose] definicao: persona=\"{}\" ({}) tratamento=\"{}\" idioma={} tom={} diretrizes={} limites={} preferencias={}",
        definicao.persona.nome,
        definicao.persona.arquetipo,
        definicao.usuario.tratamento,
        definicao.usuario.idioma,
        definicao.persona.tom.len(),
        definicao.persona.diretrizes.len(),
        definicao.persona.limites.len(),
        definicao.usuario.preferencias.len()
    );
}

/// Emite detalhes do destino resolvido para o adapter atual (`-v`).
pub fn emitir_contexto_destino(
    verbose: u8,
    harness: &str,
    destino_base: &Path,
    origem: OrigemResolucaoDestino,
    perfil_dir: &Path,
    arquivo_estado: &Path,
) {
    if verbose == 0 {
        return;
    }

    let origem_rotulo = match origem {
        OrigemResolucaoDestino::Explicito => "explicito",
        OrigemResolucaoDestino::VariavelAmbiente => "variavel_ambiente",
        OrigemResolucaoDestino::PadraoHome => "padrao_home",
        OrigemResolucaoDestino::Indisponivel => "indisponivel",
    };
    let mut err = std::io::stderr().lock();
    let _ = writeln!(
        err,
        "[verbose] destino: harness={harness} origem={origem_rotulo} base={} perfil_dir={} estado={} (existe={})",
        destino_base.display(),
        perfil_dir.display(),
        arquivo_estado.display(),
        arquivo_estado.exists()
    );
}

/// Emite detalhes das estratégias, permissões e métricas dos arquivos planejados (`-v` / `-vv`).
pub fn emitir_contexto_plano(verbose: u8, plano: &PlanoDeSetup) {
    if verbose == 0 {
        return;
    }

    let mut err = std::io::stderr().lock();
    let _ = writeln!(
        err,
        "[verbose] plano: {} ação(ões) (estado={})",
        plano.acoes.len(),
        plano.arquivo_estado.display()
    );
    for acao in &plano.acoes {
        let estrategia = formatar_estrategia(acao.estrategia);
        let permissao = match acao.permissao {
            PermissaoDeArquivo::Padrao => "padrao",
            PermissaoDeArquivo::Executavel => "executavel(+x)",
        };
        let operacao = match &acao.operacao {
            OperacaoPlanejada::Criar => "criar".to_owned(),
            OperacaoPlanejada::Atualizar => "atualizar".to_owned(),
            OperacaoPlanejada::SemAlteracoes => "sem_alteracoes".to_owned(),
            OperacaoPlanejada::Remover => "remover".to_owned(),
            OperacaoPlanejada::Conflito { motivo } => format!("conflito({motivo})"),
        };
        if verbose >= 2 {
            let linhas = acao.conteudo_final.lines().count();
            let bytes = acao.conteudo_final.len();
            let _ = writeln!(
                err,
                "[verbose]   - {} operacao={operacao} estrategia={estrategia} permissao={permissao} tamanho={bytes}B/{linhas} linhas",
                acao.caminho.display()
            );
        } else {
            let _ = writeln!(
                err,
                "[verbose]   - {} operacao={operacao} estrategia={estrategia} permissao={permissao}",
                acao.caminho.display()
            );
        }
    }
}

/// Emite detalhes de verificação em `stderr` (`-v`).
pub fn emitir_contexto_verificacao(verbose: u8, resultado: &ResultadoDaVerificacao) {
    if verbose == 0 {
        return;
    }

    let mut err = std::io::stderr().lock();
    let _ = writeln!(
        err,
        "[verbose] verificacao: sincronizado={} itens={}",
        resultado.sincronizado,
        resultado.itens.len()
    );
    for item in &resultado.itens {
        let _ = writeln!(
            err,
            "[verbose]   - {} sincronizado={} detalhe=\"{}\"",
            item.caminho.display(),
            item.sincronizado,
            item.detalhe
        );
    }
}

fn formatar_estrategia(estrategia: EstrategiaDeEscrita) -> &'static str {
    match estrategia {
        EstrategiaDeEscrita::ArquivoIntegral => "ArquivoIntegral",
        EstrategiaDeEscrita::BlocoDelimitado { .. } => "BlocoDelimitado",
        EstrategiaDeEscrita::MesclagemEstruturada {
            formato: FormatoEstruturado::Json,
        } => "MesclagemEstruturada(Json)",
        EstrategiaDeEscrita::MesclagemEstruturada {
            formato: FormatoEstruturado::Toml,
        } => "MesclagemEstruturada(Toml)",
    }
}

fn ler_env_opcional(chave: &str) -> String {
    std::env::var(chave)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "<não definida>".to_owned())
}
