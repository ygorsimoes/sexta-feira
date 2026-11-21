//! Planejamento, verificação e aplicação declarativa de arquivos de setup.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::estruturado;

/// Formato de arquivo estruturado suportado pela estratégia de mesclagem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatoEstruturado {
    /// Documento JSON com raiz objeto.
    Json,
    /// Documento TOML com raiz tabela.
    Toml,
}

/// Permissão aplicada ao arquivo no sistema de arquivos de destino.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissaoDeArquivo {
    /// Permissão padrão do sistema para arquivos comuns.
    #[default]
    Padrao,
    /// Marca o arquivo como executável (`+x`) em plataformas Unix.
    Executavel,
}

/// Estratégia de gerenciamento de um arquivo no destino.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstrategiaDeEscrita {
    /// A Sexta-feira gerencia todo o conteúdo do arquivo.
    ArquivoIntegral,
    /// A Sexta-feira gerencia apenas o trecho entre os marcadores informados.
    BlocoDelimitado {
        /// Marcador de início do bloco gerenciado.
        inicio: &'static str,
        /// Marcador de fim do bloco gerenciado.
        fim: &'static str,
    },
    /// A Sexta-feira mescla apenas as chaves gerenciadas mantendo as do usuário.
    MesclagemEstruturada {
        /// Formato do documento estruturado.
        formato: FormatoEstruturado,
    },
}

/// Descrição declarativa de um arquivo esperado após a aplicação do setup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArquivoDesejado {
    /// Caminho completo do arquivo no sistema de arquivos do usuário.
    pub caminho: PathBuf,
    /// Conteúdo desejado (integral ou interno ao bloco delimitado).
    pub conteudo: String,
    /// Estratégia adotada para criação, atualização e detecção de conflitos.
    pub estrategia: EstrategiaDeEscrita,
    /// Permissão desejada para o arquivo.
    pub permissao: PermissaoDeArquivo,
}

impl ArquivoDesejado {
    /// Cria um arquivo desejado com permissão padrão (`PermissaoDeArquivo::Padrao`).
    #[must_use]
    pub const fn new(caminho: PathBuf, conteudo: String, estrategia: EstrategiaDeEscrita) -> Self {
        Self {
            caminho,
            conteudo,
            estrategia,
            permissao: PermissaoDeArquivo::Padrao,
        }
    }

    /// Define a permissão desejada para o arquivo.
    #[must_use]
    pub const fn com_permissao(mut self, permissao: PermissaoDeArquivo) -> Self {
        self.permissao = permissao;
        self
    }
}

/// Classificação da operação necessária para sincronizar um arquivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperacaoPlanejada {
    /// O arquivo não existe e será criado.
    Criar,
    /// O arquivo existe e precisa ser atualizado sem perda de dados fora do escopo.
    Atualizar,
    /// O arquivo já está idêntico ao estado desejado.
    SemAlteracoes,
    /// O arquivo anteriormente gerenciado tornou-se obsoleto e será removido.
    Remover,
    /// O arquivo integral divergiu fora do controle do setup e exige `--forcar`.
    Conflito {
        /// Explicação legível da divergência detectada.
        motivo: String,
    },
}

/// Item planejado para um único arquivo de destino.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcaoPlanejada {
    /// Caminho do arquivo alvo.
    pub caminho: PathBuf,
    /// Operação prevista para este arquivo.
    pub operacao: OperacaoPlanejada,
    /// Conteúdo final completo que ficará gravado em disco após a aplicação.
    pub conteudo_final: String,
    /// Estratégia associada ao arquivo.
    pub estrategia: EstrategiaDeEscrita,
    /// Permissão associada ao arquivo.
    pub permissao: PermissaoDeArquivo,
    /// Resumo textual legível das mudanças previstas, quando houver.
    pub resumo_diff: Option<String>,
    remocao: bool,
    checksum_estruturado: Option<String>,
}

/// Plano completo de sincronização calculado antes de qualquer escrita.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanoDeSetup {
    /// Caminho do arquivo de estado onde os checksums aplicados são registrados.
    pub arquivo_estado: PathBuf,
    /// Ações ordenadas para cada arquivo desejado.
    pub acoes: Vec<AcaoPlanejada>,
}

/// Resumo das operações realizadas ao aplicar um plano.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultadoDaAplicacao {
    /// Quantidade de arquivos criados.
    pub criados: usize,
    /// Quantidade de arquivos atualizados (incluindo sobrescritas forçadas).
    pub atualizados: usize,
    /// Caminhos dos backups `.bak` gerados durante sobrescritas forçadas.
    pub backups_criados: Vec<PathBuf>,
}

/// Diagnóstico individual de um arquivo durante a verificação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemDeVerificacao {
    /// Caminho do arquivo verificado.
    pub caminho: PathBuf,
    /// Indica se o arquivo existe e está sincronizado com o estado desejado.
    pub sincronizado: bool,
    /// Descrição objetiva do estado encontrado.
    pub detalhe: String,
}

/// Resultado consolidado da verificação de um setup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultadoDaVerificacao {
    /// Verdadeiro somente quando todos os arquivos estão sincronizados.
    pub sincronizado: bool,
    /// Lista de verificações por arquivo.
    pub itens: Vec<ItemDeVerificacao>,
}

/// Erros ocorridos ao planejar, verificar ou aplicar um setup.
#[derive(Debug, thiserror::Error)]
pub enum ErroDePlano {
    /// Erro de leitura ou escrita em disco.
    #[error("erro de E/S em `{caminho}`: {fonte}")]
    Io {
        /// Caminho envolvido na falha.
        caminho: PathBuf,
        /// Causa original do sistema operacional.
        #[source]
        fonte: io::Error,
    },
    /// Arquivo de estado TOML inválido ou corrompido.
    #[error("não foi possível ler o estado do setup em `{caminho}`: {fonte}")]
    EstadoInvalido {
        /// Caminho do arquivo de estado.
        caminho: PathBuf,
        /// Erro de deserialização TOML.
        #[source]
        fonte: toml::de::Error,
    },
    /// Arquivo de estado com versão de esquema desconhecida ou incompatível.
    #[error("versão de estado não suportada ({versao}) em `{caminho}`")]
    VersaoDeEstadoNaoSuportada {
        /// Caminho do arquivo de estado.
        caminho: PathBuf,
        /// Versão encontrada no arquivo.
        versao: u32,
    },
    /// Falha ao serializar o arquivo de estado para TOML.
    #[error("não foi possível serializar o estado do setup: {0}")]
    SerializacaoDeEstado(#[source] toml::ser::Error),
    /// Marcadores de bloco delimitado ausentes parcialmente, duplicados ou fora de ordem.
    #[error("delimitadores inválidos em `{caminho}`: verifique os marcadores `{inicio}` e `{fim}`")]
    DelimitadoresInvalidos {
        /// Caminho do arquivo com marcadores inconsistentes.
        caminho: PathBuf,
        /// Marcador inicial esperado.
        inicio: &'static str,
        /// Marcador final esperado.
        fim: &'static str,
    },
    /// Tentativa de aplicar um plano contendo conflitos sem `--forcar`.
    #[error("conflito em `{caminho}`: {motivo}. Use `--forcar` para substituir gerando backup")]
    ConflitoPendente {
        /// Caminho do arquivo em conflito.
        caminho: PathBuf,
        /// Motivo do conflito detectado.
        motivo: String,
    },
    /// Conteúdo desejado inválido para mesclagem estruturada.
    #[error("conteúdo estruturado inválido para `{caminho}`: {detalhe}")]
    ConteudoEstruturadoInvalido {
        /// Caminho do arquivo alvo.
        caminho: PathBuf,
        /// Descrição do erro de sintaxe ou estrutura.
        detalhe: String,
    },
}

const VERSAO_ESTADO_ATUAL: u32 = 1;
const LIMITE_CELULAS_LCS: usize = 250_000;
const DIFF_PERMISSAO_EXECUTAVEL: &str = "~ ajuste de permissão para executável (+x)";

const fn versao_estado_atual() -> u32 {
    VERSAO_ESTADO_ATUAL
}

#[derive(Debug, Serialize, Deserialize)]
struct EstadoAplicado {
    #[serde(default = "versao_estado_atual")]
    versao: u32,
    #[serde(default)]
    arquivos_integrais: BTreeMap<String, String>,
    #[serde(default)]
    arquivos_estruturados: BTreeMap<String, String>,
}

impl Default for EstadoAplicado {
    fn default() -> Self {
        Self {
            versao: VERSAO_ESTADO_ATUAL,
            arquivos_integrais: BTreeMap::new(),
            arquivos_estruturados: BTreeMap::new(),
        }
    }
}

/// Calcula o plano declarativo sem modificar o sistema de arquivos.
///
/// # Errors
///
/// Retorna erro em falhas de leitura, estado corrompido ou delimitadores inválidos.
pub fn planejar(
    arquivos: Vec<ArquivoDesejado>,
    arquivo_estado: PathBuf,
) -> Result<PlanoDeSetup, ErroDePlano> {
    let estado = carregar_estado(&arquivo_estado)?;
    let raiz_estado = arquivo_estado.parent();
    let mut acoes = Vec::with_capacity(arquivos.len());
    let mut chaves_integrais_desejadas = BTreeSet::new();

    for arquivo in arquivos {
        let atual = ler_opcional(&arquivo.caminho)?;
        let acao = match arquivo.estrategia {
            EstrategiaDeEscrita::ArquivoIntegral => {
                chaves_integrais_desejadas.insert(chave_estado(&arquivo.caminho, raiz_estado));
                planejar_integral(&arquivo, atual.as_deref(), &estado, raiz_estado)?
            }
            EstrategiaDeEscrita::BlocoDelimitado { inicio, fim } => {
                planejar_bloco_delimitado(&arquivo, atual.as_deref(), inicio, fim)?
            }
            EstrategiaDeEscrita::MesclagemEstruturada { formato } => {
                planejar_estruturado(&arquivo, atual.as_deref(), formato, &estado, raiz_estado)?
            }
        };
        acoes.push(acao);
    }

    for (chave, checksum_registrado) in &estado.arquivos_integrais {
        if chaves_integrais_desejadas.contains(chave) {
            continue;
        }
        let caminho = resolver_caminho_estado(chave, raiz_estado);
        let Some(conteudo_atual) = ler_opcional(&caminho)? else {
            continue;
        };
        let atual_normalizado = normalizar_quebras(&conteudo_atual);
        let operacao = if checksum_conteudo(&atual_normalizado) == *checksum_registrado {
            OperacaoPlanejada::Remover
        } else {
            OperacaoPlanejada::Conflito {
                motivo: "o arquivo obsoleto foi modificado manualmente desde a última aplicação"
                    .to_owned(),
            }
        };
        acoes.push(AcaoPlanejada {
            caminho,
            operacao,
            conteudo_final: String::new(),
            estrategia: EstrategiaDeEscrita::ArquivoIntegral,
            permissao: PermissaoDeArquivo::Padrao,
            resumo_diff: Some(gerar_resumo_diff(&atual_normalizado, "")),
            remocao: true,
            checksum_estruturado: None,
        });
    }

    Ok(PlanoDeSetup {
        arquivo_estado,
        acoes,
    })
}

/// Aplica um plano previamente calculado ao sistema de arquivos.
///
/// Sem `forcar`, aborta antes de qualquer escrita caso exista ao menos um conflito.
/// Com `forcar`, cria um backup `.bak` antes de sobrescrever arquivos em conflito.
///
/// # Errors
///
/// Retorna erro se houver conflito pendente sem `forcar` ou em falhas de E/S.
pub fn aplicar(plano: &PlanoDeSetup, forcar: bool) -> Result<ResultadoDaAplicacao, ErroDePlano> {
    if !forcar && let Some(conflito) = plano.acoes.iter().find_map(extrair_conflito) {
        return Err(conflito);
    }

    let mut estado = carregar_estado(&plano.arquivo_estado)?;
    let raiz_estado = plano.arquivo_estado.parent();
    let mut criados = 0;
    let mut atualizados = 0;
    let mut backups_criados = Vec::new();
    let mut alterou_estado = false;
    let mut chaves_integrais_ativas = BTreeSet::new();
    let mut chaves_estruturadas_ativas = BTreeSet::new();

    for acao in &plano.acoes {
        let resultado_acao =
            aplicar_acao(acao, &mut criados, &mut atualizados, &mut backups_criados);
        if let Err(erro) = resultado_acao {
            if alterou_estado {
                let _ = salvar_estado(&plano.arquivo_estado, &estado);
            }
            return Err(erro);
        }

        let chave = chave_estado(&acao.caminho, raiz_estado);
        match acao.estrategia {
            EstrategiaDeEscrita::ArquivoIntegral => {
                if acao.remocao {
                    if estado.arquivos_integrais.remove(&chave).is_some() {
                        alterou_estado = true;
                    }
                } else {
                    chaves_integrais_ativas.insert(chave.clone());
                    let checksum = checksum_conteudo(&acao.conteudo_final);
                    if estado.arquivos_integrais.get(&chave) != Some(&checksum) {
                        estado.arquivos_integrais.insert(chave, checksum);
                        alterou_estado = true;
                    }
                }
            }
            EstrategiaDeEscrita::MesclagemEstruturada { .. } => {
                chaves_estruturadas_ativas.insert(chave.clone());
                if let Some(checksum) = &acao.checksum_estruturado
                    && estado.arquivos_estruturados.get(&chave) != Some(checksum)
                {
                    estado.arquivos_estruturados.insert(chave, checksum.clone());
                    alterou_estado = true;
                }
            }
            EstrategiaDeEscrita::BlocoDelimitado { .. } => {}
        }
    }

    let antes_poda_integrais = estado.arquivos_integrais.len();
    estado
        .arquivos_integrais
        .retain(|chave, _| chaves_integrais_ativas.contains(chave));
    if estado.arquivos_integrais.len() != antes_poda_integrais {
        alterou_estado = true;
    }

    let antes_poda_estruturados = estado.arquivos_estruturados.len();
    estado
        .arquivos_estruturados
        .retain(|chave, _| chaves_estruturadas_ativas.contains(chave));
    if estado.arquivos_estruturados.len() != antes_poda_estruturados {
        alterou_estado = true;
    }

    if alterou_estado {
        salvar_estado(&plano.arquivo_estado, &estado)?;
    }

    Ok(ResultadoDaAplicacao {
        criados,
        atualizados,
        backups_criados,
    })
}

fn aplicar_acao(
    acao: &AcaoPlanejada,
    criados: &mut usize,
    atualizados: &mut usize,
    backups_criados: &mut Vec<PathBuf>,
) -> Result<(), ErroDePlano> {
    match &acao.operacao {
        OperacaoPlanejada::SemAlteracoes => Ok(()),
        OperacaoPlanejada::Criar => {
            gravar_arquivo(&acao.caminho, &acao.conteudo_final)?;
            garantir_permissao(&acao.caminho, acao.permissao)?;
            *criados += 1;
            Ok(())
        }
        OperacaoPlanejada::Atualizar => {
            if acao.resumo_diff.as_deref() != Some(DIFF_PERMISSAO_EXECUTAVEL) {
                gravar_arquivo(&acao.caminho, &acao.conteudo_final)?;
            }
            garantir_permissao(&acao.caminho, acao.permissao)?;
            *atualizados += 1;
            Ok(())
        }
        OperacaoPlanejada::Remover => remover_arquivo_se_existir(&acao.caminho),
        OperacaoPlanejada::Conflito { .. } => {
            if acao.caminho.exists() {
                let backup = caminho_backup(&acao.caminho);
                fs::copy(&acao.caminho, &backup).map_err(|fonte| ErroDePlano::Io {
                    caminho: backup.clone(),
                    fonte,
                })?;
                backups_criados.push(backup);
            }
            if acao.remocao {
                return remover_arquivo_se_existir(&acao.caminho);
            }
            gravar_arquivo(&acao.caminho, &acao.conteudo_final)?;
            garantir_permissao(&acao.caminho, acao.permissao)?;
            *atualizados += 1;
            Ok(())
        }
    }
}

fn remover_arquivo_se_existir(caminho: &Path) -> Result<(), ErroDePlano> {
    match fs::remove_file(caminho) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(fonte) => Err(ErroDePlano::Io {
            caminho: caminho.to_path_buf(),
            fonte,
        }),
    }
}

/// Verifica se todos os arquivos desejados existem e estão sincronizados.
///
/// # Errors
///
/// Retorna erro em falhas de leitura, estado corrompido ou delimitadores inválidos.
pub fn verificar(
    arquivos: &[ArquivoDesejado],
    arquivo_estado: &Path,
) -> Result<ResultadoDaVerificacao, ErroDePlano> {
    let plano = planejar(arquivos.to_vec(), arquivo_estado.to_path_buf())?;
    let mut itens = Vec::with_capacity(plano.acoes.len());
    let mut sincronizado = true;

    for acao in plano.acoes {
        let (item_ok, detalhe) = match acao.operacao {
            OperacaoPlanejada::SemAlteracoes => (true, "sincronizado".to_owned()),
            OperacaoPlanejada::Criar => (false, "arquivo ausente".to_owned()),
            OperacaoPlanejada::Atualizar
                if acao.resumo_diff.as_deref() == Some(DIFF_PERMISSAO_EXECUTAVEL) =>
            {
                (false, "permissão executável ausente".to_owned())
            }
            OperacaoPlanejada::Atualizar => (false, "atualização pendente".to_owned()),
            OperacaoPlanejada::Remover => {
                (false, "arquivo obsoleto pendente de remoção".to_owned())
            }
            OperacaoPlanejada::Conflito { motivo } => (false, format!("conflito: {motivo}")),
        };
        if !item_ok {
            sincronizado = false;
        }
        itens.push(ItemDeVerificacao {
            caminho: acao.caminho,
            sincronizado: item_ok,
            detalhe,
        });
    }

    Ok(ResultadoDaVerificacao {
        sincronizado,
        itens,
    })
}

fn planejar_integral(
    arquivo: &ArquivoDesejado,
    atual: Option<&str>,
    estado: &EstadoAplicado,
    raiz_estado: Option<&Path>,
) -> Result<AcaoPlanejada, ErroDePlano> {
    let desejado = normalizar_quebras(&arquivo.conteudo);
    let Some(conteudo_atual) = atual else {
        let resumo_diff = Some(gerar_resumo_diff("", &desejado));
        return Ok(AcaoPlanejada {
            caminho: arquivo.caminho.clone(),
            operacao: OperacaoPlanejada::Criar,
            conteudo_final: desejado,
            estrategia: arquivo.estrategia,
            permissao: arquivo.permissao,
            resumo_diff,
            remocao: false,
            checksum_estruturado: None,
        });
    };

    let atual_normalizado = normalizar_quebras(conteudo_atual);
    if atual_normalizado == desejado {
        if permissao_pendente(&arquivo.caminho, arquivo.permissao)? {
            return Ok(AcaoPlanejada {
                caminho: arquivo.caminho.clone(),
                operacao: OperacaoPlanejada::Atualizar,
                conteudo_final: desejado,
                estrategia: arquivo.estrategia,
                permissao: arquivo.permissao,
                resumo_diff: Some(DIFF_PERMISSAO_EXECUTAVEL.to_owned()),
                remocao: false,
                checksum_estruturado: None,
            });
        }
        return Ok(AcaoPlanejada {
            caminho: arquivo.caminho.clone(),
            operacao: OperacaoPlanejada::SemAlteracoes,
            conteudo_final: desejado,
            estrategia: arquivo.estrategia,
            permissao: arquivo.permissao,
            resumo_diff: None,
            remocao: false,
            checksum_estruturado: None,
        });
    }

    let chave = chave_estado(&arquivo.caminho, raiz_estado);
    let checksum_atual = checksum_conteudo(&atual_normalizado);
    let operacao = match estado.arquivos_integrais.get(&chave) {
        Some(registrado) if *registrado == checksum_atual => OperacaoPlanejada::Atualizar,
        Some(_) => OperacaoPlanejada::Conflito {
            motivo: "o arquivo foi modificado manualmente desde a última aplicação".to_owned(),
        },
        None => OperacaoPlanejada::Conflito {
            motivo: "o arquivo já existia antes de ser gerenciado pela Sexta-feira".to_owned(),
        },
    };

    let resumo_diff = Some(gerar_resumo_diff(&atual_normalizado, &desejado));
    Ok(AcaoPlanejada {
        caminho: arquivo.caminho.clone(),
        operacao,
        conteudo_final: desejado,
        estrategia: arquivo.estrategia,
        permissao: arquivo.permissao,
        resumo_diff,
        remocao: false,
        checksum_estruturado: None,
    })
}

fn planejar_bloco_delimitado(
    arquivo: &ArquivoDesejado,
    atual: Option<&str>,
    inicio: &'static str,
    fim: &'static str,
) -> Result<AcaoPlanejada, ErroDePlano> {
    let atual_normalizado = atual.map(normalizar_quebras);
    let conteudo_final = mesclar_bloco(
        atual_normalizado.as_deref(),
        &arquivo.conteudo,
        inicio,
        fim,
        &arquivo.caminho,
    )?;

    let (operacao, resumo_diff) = match atual_normalizado {
        None => (
            OperacaoPlanejada::Criar,
            Some(gerar_resumo_diff("", &conteudo_final)),
        ),
        Some(ref anterior) if *anterior == conteudo_final => {
            if permissao_pendente(&arquivo.caminho, arquivo.permissao)? {
                (
                    OperacaoPlanejada::Atualizar,
                    Some(DIFF_PERMISSAO_EXECUTAVEL.to_owned()),
                )
            } else {
                (OperacaoPlanejada::SemAlteracoes, None)
            }
        }
        Some(ref anterior) => (
            OperacaoPlanejada::Atualizar,
            Some(gerar_resumo_diff(anterior, &conteudo_final)),
        ),
    };

    Ok(AcaoPlanejada {
        caminho: arquivo.caminho.clone(),
        operacao,
        conteudo_final,
        estrategia: arquivo.estrategia,
        permissao: arquivo.permissao,
        resumo_diff,
        remocao: false,
        checksum_estruturado: None,
    })
}

fn planejar_estruturado(
    arquivo: &ArquivoDesejado,
    atual: Option<&str>,
    formato: FormatoEstruturado,
    estado: &EstadoAplicado,
    raiz_estado: Option<&Path>,
) -> Result<AcaoPlanejada, ErroDePlano> {
    let chave = chave_estado(&arquivo.caminho, raiz_estado);
    let checksum_registrado = estado.arquivos_estruturados.get(&chave).map(String::as_str);
    let res = estruturado::planejar_documento(
        formato,
        &arquivo.caminho,
        &arquivo.conteudo,
        atual,
        checksum_registrado,
    )?;

    let (operacao, resumo_diff) = match res.operacao {
        OperacaoPlanejada::SemAlteracoes => {
            if permissao_pendente(&arquivo.caminho, arquivo.permissao)? {
                (
                    OperacaoPlanejada::Atualizar,
                    Some(DIFF_PERMISSAO_EXECUTAVEL.to_owned()),
                )
            } else {
                (OperacaoPlanejada::SemAlteracoes, None)
            }
        }
        OperacaoPlanejada::Criar => (
            OperacaoPlanejada::Criar,
            Some(gerar_resumo_diff("", &res.conteudo_final)),
        ),
        outro => {
            let antes = res.texto_anterior.as_deref().unwrap_or("");
            (outro, Some(gerar_resumo_diff(antes, &res.conteudo_final)))
        }
    };

    Ok(AcaoPlanejada {
        caminho: arquivo.caminho.clone(),
        operacao,
        conteudo_final: res.conteudo_final,
        estrategia: arquivo.estrategia,
        permissao: arquivo.permissao,
        resumo_diff,
        remocao: false,
        checksum_estruturado: Some(res.checksum_patch),
    })
}

fn resolver_caminho_estado(chave: &str, raiz_estado: Option<&Path>) -> PathBuf {
    let caminho = Path::new(chave);
    if caminho.is_relative()
        && let Some(raiz) = raiz_estado
        && !raiz.as_os_str().is_empty()
    {
        return raiz.join(caminho);
    }
    caminho.to_path_buf()
}

#[cfg(unix)]
fn permissao_pendente(caminho: &Path, permissao: PermissaoDeArquivo) -> Result<bool, ErroDePlano> {
    use std::os::unix::fs::PermissionsExt;

    if permissao != PermissaoDeArquivo::Executavel {
        return Ok(false);
    }
    let meta = fs::metadata(caminho).map_err(|fonte| ErroDePlano::Io {
        caminho: caminho.to_path_buf(),
        fonte,
    })?;
    Ok(meta.permissions().mode() & 0o111 == 0)
}

#[cfg(not(unix))]
const fn permissao_pendente(
    _caminho: &Path,
    _permissao: PermissaoDeArquivo,
) -> Result<bool, ErroDePlano> {
    Ok(false)
}

#[cfg(unix)]
fn garantir_permissao(caminho: &Path, permissao: PermissaoDeArquivo) -> Result<(), ErroDePlano> {
    use std::os::unix::fs::PermissionsExt;

    if permissao != PermissaoDeArquivo::Executavel {
        return Ok(());
    }
    let meta = fs::metadata(caminho).map_err(|fonte| ErroDePlano::Io {
        caminho: caminho.to_path_buf(),
        fonte,
    })?;
    let mut perms = meta.permissions();
    let atual = perms.mode();
    if atual & 0o111 == 0 {
        perms.set_mode(atual | 0o111);
        fs::set_permissions(caminho, perms).map_err(|fonte| ErroDePlano::Io {
            caminho: caminho.to_path_buf(),
            fonte,
        })?;
    }
    Ok(())
}

#[cfg(not(unix))]
const fn garantir_permissao(
    _caminho: &Path,
    _permissao: PermissaoDeArquivo,
) -> Result<(), ErroDePlano> {
    Ok(())
}

fn mesclar_bloco(
    existente: Option<&str>,
    bloco_interno: &str,
    inicio: &'static str,
    fim: &'static str,
    caminho: &Path,
) -> Result<String, ErroDePlano> {
    let bloco_interno_norm = normalizar_quebras(bloco_interno);
    let corpo = bloco_interno_norm.trim_end_matches('\n');
    let bloco_completo = format!("{inicio}\n{corpo}\n{fim}\n");

    let Some(texto) = existente else {
        return Ok(bloco_completo);
    };

    let ocorrencias_inicio = localizar_linhas_marcador(texto, inicio);
    let ocorrencias_fim = localizar_linhas_marcador(texto, fim);

    match (ocorrencias_inicio.as_slice(), ocorrencias_fim.as_slice()) {
        ([], []) => {
            if texto.trim().is_empty() {
                Ok(bloco_completo)
            } else {
                Ok(format!(
                    "{bloco_completo}\n{}",
                    texto.trim_start_matches('\n')
                ))
            }
        }
        ([pos_inicio], [pos_fim]) if pos_inicio.inicio_byte < pos_fim.inicio_byte => {
            let antes = &texto[..pos_inicio.inicio_byte];
            let depois = &texto[pos_fim.fim_byte..];
            Ok(format!("{antes}{bloco_completo}{depois}"))
        }
        _ => Err(ErroDePlano::DelimitadoresInvalidos {
            caminho: caminho.to_path_buf(),
            inicio,
            fim,
        }),
    }
}

struct OcorrenciaMarcador {
    inicio_byte: usize,
    fim_byte: usize,
}

fn localizar_linhas_marcador(texto: &str, marcador: &str) -> Vec<OcorrenciaMarcador> {
    let mut ocorrencias = Vec::new();
    let mut offset = 0usize;

    for linha in texto.split_inclusive('\n') {
        let fim_linha = offset + linha.len();
        let conteudo_linha = linha.strip_suffix('\n').unwrap_or(linha);
        if conteudo_linha.trim_end_matches([' ', '\t']) == marcador {
            ocorrencias.push(OcorrenciaMarcador {
                inicio_byte: offset,
                fim_byte: fim_linha,
            });
        }
        offset = fim_linha;
    }

    ocorrencias
}

fn extrair_conflito(acao: &AcaoPlanejada) -> Option<ErroDePlano> {
    match &acao.operacao {
        OperacaoPlanejada::Conflito { motivo } => Some(ErroDePlano::ConflitoPendente {
            caminho: acao.caminho.clone(),
            motivo: motivo.clone(),
        }),
        _ => None,
    }
}

fn ler_opcional(caminho: &Path) -> Result<Option<String>, ErroDePlano> {
    match fs::read_to_string(caminho) {
        Ok(conteudo) => Ok(Some(conteudo)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(fonte) => Err(ErroDePlano::Io {
            caminho: caminho.to_path_buf(),
            fonte,
        }),
    }
}

static CONTADOR_TEMP_ESCRITA: AtomicU64 = AtomicU64::new(0);

fn gravar_arquivo(caminho: &Path, conteudo: &str) -> Result<(), ErroDePlano> {
    let dir_pai = match caminho.parent() {
        Some(pai) if !pai.as_os_str().is_empty() => {
            fs::create_dir_all(pai).map_err(|fonte| ErroDePlano::Io {
                caminho: pai.to_path_buf(),
                fonte,
            })?;
            pai
        }
        _ => Path::new("."),
    };

    let (caminho_temp, mut arquivo) = criar_temporario_exclusivo(dir_pai, caminho)?;

    if let Err(fonte) = arquivo.write_all(conteudo.as_bytes()) {
        drop(arquivo);
        let _ = fs::remove_file(&caminho_temp);
        return Err(ErroDePlano::Io {
            caminho: caminho.to_path_buf(),
            fonte,
        });
    }
    drop(arquivo);

    if let Err(fonte) = fs::rename(&caminho_temp, caminho) {
        let _ = fs::remove_file(&caminho_temp);
        return Err(ErroDePlano::Io {
            caminho: caminho.to_path_buf(),
            fonte,
        });
    }

    Ok(())
}

fn criar_temporario_exclusivo(
    dir_pai: &Path,
    destino: &Path,
) -> Result<(PathBuf, fs::File), ErroDePlano> {
    loop {
        let seq = CONTADOR_TEMP_ESCRITA.fetch_add(1, Ordering::Relaxed);
        let nome_temp = format!(".sexta-feira-tmp-{}-{seq}", std::process::id());
        let caminho_temp = dir_pai.join(nome_temp);

        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&caminho_temp)
        {
            Ok(arquivo) => return Ok((caminho_temp, arquivo)),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
            Err(fonte) => {
                return Err(ErroDePlano::Io {
                    caminho: destino.to_path_buf(),
                    fonte,
                });
            }
        }
    }
}

fn carregar_estado(caminho: &Path) -> Result<EstadoAplicado, ErroDePlano> {
    let Some(conteudo) = ler_opcional(caminho)? else {
        return Ok(EstadoAplicado::default());
    };
    let estado: EstadoAplicado =
        toml::from_str(&conteudo).map_err(|fonte| ErroDePlano::EstadoInvalido {
            caminho: caminho.to_path_buf(),
            fonte,
        })?;
    if estado.versao != VERSAO_ESTADO_ATUAL {
        return Err(ErroDePlano::VersaoDeEstadoNaoSuportada {
            caminho: caminho.to_path_buf(),
            versao: estado.versao,
        });
    }
    Ok(estado)
}

fn salvar_estado(caminho: &Path, estado: &EstadoAplicado) -> Result<(), ErroDePlano> {
    let serializado = toml::to_string_pretty(estado).map_err(ErroDePlano::SerializacaoDeEstado)?;
    gravar_arquivo(caminho, &serializado)
}

fn chave_estado(caminho: &Path, raiz: Option<&Path>) -> String {
    let caminho_norm = normalizar_caminho(caminho);
    if let Some(base) = raiz {
        let base_norm = normalizar_caminho(base);
        if let Ok(relativo) = caminho_norm.strip_prefix(&base_norm) {
            return relativo.to_string_lossy().into_owned();
        }
    }
    caminho_norm.to_string_lossy().into_owned()
}

fn normalizar_caminho(caminho: &Path) -> PathBuf {
    let absoluto = if caminho.is_relative()
        && let Ok(cwd) = std::env::current_dir()
    {
        cwd.join(caminho)
    } else {
        caminho.to_path_buf()
    };

    let mut normalizado = PathBuf::new();
    for componente in absoluto.components() {
        match componente {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(
                    normalizado.components().next_back(),
                    Some(Component::Normal(_))
                ) {
                    normalizado.pop();
                } else if normalizado.is_relative() {
                    normalizado.push(componente.as_os_str());
                }
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalizado.push(componente.as_os_str());
            }
        }
    }
    normalizado
}

fn caminho_backup(caminho: &Path) -> PathBuf {
    let mut base = caminho.as_os_str().to_os_string();
    base.push(".bak");
    let inicial = PathBuf::from(&base);
    if !caminho_ocupado(&inicial) {
        return inicial;
    }

    for indice in 1usize.. {
        let mut candidato_os = OsString::from(&base);
        candidato_os.push(format!(".{indice}"));
        let candidato = PathBuf::from(candidato_os);
        if !caminho_ocupado(&candidato) {
            return candidato;
        }
    }

    inicial
}

fn caminho_ocupado(caminho: &Path) -> bool {
    !matches!(
        fs::symlink_metadata(caminho),
        Err(err) if err.kind() == io::ErrorKind::NotFound
    )
}

fn normalizar_quebras(texto: &str) -> String {
    texto.replace("\r\n", "\n")
}

pub(crate) fn checksum_conteudo(conteudo: &str) -> String {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;

    let mut hash = OFFSET_BASIS;
    for byte in conteudo.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("fnv1a64:{hash:016x}:{}", conteudo.len())
}

fn gerar_resumo_diff(antes: &str, depois: &str) -> String {
    let linhas_antes: Vec<&str> = antes.lines().collect();
    let linhas_depois: Vec<&str> = depois.lines().collect();

    let prefixo = linhas_antes
        .iter()
        .zip(&linhas_depois)
        .take_while(|(a, b)| a == b)
        .count();
    let restante_antes = &linhas_antes[prefixo..];
    let restante_depois = &linhas_depois[prefixo..];
    let sufixo = restante_antes
        .iter()
        .rev()
        .zip(restante_depois.iter().rev())
        .take_while(|(a, b)| a == b)
        .count();

    let miolo_antes = &restante_antes[..restante_antes.len() - sufixo];
    let miolo_depois = &restante_depois[..restante_depois.len() - sufixo];

    let mut saida = String::new();
    let celulas = (miolo_antes.len() + 1).saturating_mul(miolo_depois.len() + 1);
    if celulas > LIMITE_CELULAS_LCS {
        for linha in miolo_antes {
            let _ = writeln!(saida, "- {linha}");
        }
        for linha in miolo_depois {
            let _ = writeln!(saida, "+ {linha}");
        }
    } else {
        let cols = miolo_depois.len() + 1;
        let dp = tabela_lcs(miolo_antes, miolo_depois);
        let mut i = 0;
        let mut j = 0;

        while i < miolo_antes.len() && j < miolo_depois.len() {
            if miolo_antes[i] == miolo_depois[j] {
                i += 1;
                j += 1;
            } else if dp[(i + 1) * cols + j] >= dp[i * cols + (j + 1)] {
                let _ = writeln!(saida, "- {}", miolo_antes[i]);
                i += 1;
            } else {
                let _ = writeln!(saida, "+ {}", miolo_depois[j]);
                j += 1;
            }
        }

        for linha in &miolo_antes[i..] {
            let _ = writeln!(saida, "- {linha}");
        }
        for linha in &miolo_depois[j..] {
            let _ = writeln!(saida, "+ {linha}");
        }
    }

    if saida.is_empty() {
        let _ = writeln!(saida, "~ alteração de formatação ou quebras de linha");
    }
    saida.trim_end_matches('\n').to_owned()
}

fn tabela_lcs(antes: &[&str], depois: &[&str]) -> Vec<usize> {
    let n = antes.len();
    let m = depois.len();
    let cols = m + 1;
    let mut dp = vec![0usize; (n + 1) * cols];

    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i * cols + j] = if antes[i] == depois[j] {
                dp[(i + 1) * cols + (j + 1)] + 1
            } else {
                dp[(i + 1) * cols + j].max(dp[i * cols + (j + 1)])
            };
        }
    }
    dp
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
                "sexta-feira-core-{prefixo}-{}-{seq}",
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
    fn planejar_e_aplicar_criam_arquivos_e_segunda_execucao_e_idempotente() {
        let dir = DirTemporario::novo("idempotente");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let soul = dir.path().join("SOUL.md");
        let user = dir.path().join("memories/USER.md");

        let desejados = vec![
            ArquivoDesejado::new(
                soul.clone(),
                "# Sexta-feira\n".to_owned(),
                EstrategiaDeEscrita::ArquivoIntegral,
            ),
            ArquivoDesejado::new(
                user.clone(),
                "Tratamento: senhor\n".to_owned(),
                EstrategiaDeEscrita::BlocoDelimitado {
                    inicio: "<!-- inicio -->",
                    fim: "<!-- fim -->",
                },
            ),
        ];

        let plano = planejar(desejados.clone(), estado.clone()).expect("planejamento inicial");
        assert!(matches!(plano.acoes[0].operacao, OperacaoPlanejada::Criar));
        assert!(matches!(plano.acoes[1].operacao, OperacaoPlanejada::Criar));
        assert!(!soul.exists());

        let aplicacao = aplicar(&plano, false).expect("aplicação inicial");
        assert_eq!(aplicacao.criados, 2);
        assert_eq!(aplicacao.atualizados, 0);
        assert!(estado.exists());

        let verificacao = verificar(&desejados, &estado).expect("verificação após aplicar");
        assert!(verificacao.sincronizado);

        let segundo_plano = planejar(desejados, estado).expect("replanejamento");
        assert!(matches!(
            segundo_plano.acoes[0].operacao,
            OperacaoPlanejada::SemAlteracoes
        ));
        assert!(matches!(
            segundo_plano.acoes[1].operacao,
            OperacaoPlanejada::SemAlteracoes
        ));
    }

    #[test]
    fn detecta_conflito_quando_arquivo_integral_preexistente_nao_tem_estado_e_aborta_sem_forcar() {
        let dir = DirTemporario::novo("conflito-preexistente");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let novo = dir.path().join("memories/USER.md");
        let existente = dir.path().join("SOUL.md");
        fs::write(&existente, "# Persona manual do usuário\n").expect("grava arquivo prévio");

        let desejados = vec![
            ArquivoDesejado::new(
                novo.clone(),
                "Tratamento: senhor\n".to_owned(),
                EstrategiaDeEscrita::BlocoDelimitado {
                    inicio: "<!-- inicio -->",
                    fim: "<!-- fim -->",
                },
            ),
            ArquivoDesejado::new(
                existente,
                "# Sexta-feira\n".to_owned(),
                EstrategiaDeEscrita::ArquivoIntegral,
            ),
        ];

        let plano = planejar(desejados, estado).expect("planejamento com conflito");
        assert!(matches!(plano.acoes[0].operacao, OperacaoPlanejada::Criar));
        assert!(matches!(
            plano.acoes[1].operacao,
            OperacaoPlanejada::Conflito { .. }
        ));

        let err = aplicar(&plano, false).expect_err("sem --forcar deve abortar");
        assert!(matches!(err, ErroDePlano::ConflitoPendente { .. }));
        assert!(!novo.exists(), "pré-checagem deve impedir criação parcial");
    }

    #[test]
    fn aplicar_com_forcar_gera_backup_e_atualiza_estado() {
        let dir = DirTemporario::novo("forcar-backup");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let soul = dir.path().join("SOUL.md");

        let desejados = vec![ArquivoDesejado::new(
            soul.clone(),
            "# Versão gerenciada\n".to_owned(),
            EstrategiaDeEscrita::ArquivoIntegral,
        )];

        let plano_inicial = planejar(desejados.clone(), estado.clone()).expect("plano inicial");
        aplicar(&plano_inicial, false).expect("aplica inicial");

        fs::write(&soul, "# Editado manualmente\n").expect("simula edição manual");

        let plano_conflito = planejar(desejados.clone(), estado.clone()).expect("plano conflito");
        assert!(matches!(
            plano_conflito.acoes[0].operacao,
            OperacaoPlanejada::Conflito { .. }
        ));

        let res = aplicar(&plano_conflito, true).expect("aplicação forçada");
        assert_eq!(res.atualizados, 1);
        assert_eq!(res.backups_criados.len(), 1);
        assert_eq!(
            fs::read_to_string(&res.backups_criados[0]).expect("lê backup"),
            "# Editado manualmente\n"
        );
        assert_eq!(
            fs::read_to_string(&soul).expect("lê soul atualizado"),
            "# Versão gerenciada\n"
        );
        assert!(
            verificar(&desejados, &estado)
                .expect("verifica")
                .sincronizado
        );
    }

    #[test]
    fn bloco_delimitado_preserva_conteudo_existente_com_ou_sem_marcadores() {
        let dir = DirTemporario::novo("bloco-delimitado");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let user = dir.path().join("USER.md");
        let inicio = "<!-- sexta-feira:inicio -->";
        let fim = "<!-- sexta-feira:fim -->";

        fs::write(&user, "# Notas aprendidas\n- Prefere café sem açúcar.\n")
            .expect("grava memória prévia");

        let v1 = vec![ArquivoDesejado::new(
            user.clone(),
            "Tratamento: senhor\n".to_owned(),
            EstrategiaDeEscrita::BlocoDelimitado { inicio, fim },
        )];
        let p1 = planejar(v1, estado.clone()).expect("plano sem marcadores");
        assert!(matches!(p1.acoes[0].operacao, OperacaoPlanejada::Atualizar));
        aplicar(&p1, false).expect("aplica bloco no topo");

        let apos_v1 = fs::read_to_string(&user).expect("lê após v1");
        assert!(apos_v1.contains("Tratamento: senhor"));
        assert!(apos_v1.contains("- Prefere café sem açúcar."));

        let com_prefixo = format!("Cabeçalho do usuário\n\n{apos_v1}\nRodapé do usuário\n");
        fs::write(&user, com_prefixo).expect("adiciona antes e depois");

        let v2 = vec![ArquivoDesejado::new(
            user.clone(),
            "Tratamento: senhor\nIdioma: pt-BR\n".to_owned(),
            EstrategiaDeEscrita::BlocoDelimitado { inicio, fim },
        )];
        let p2 = planejar(v2, estado.clone()).expect("plano com marcadores");
        aplicar(&p2, false).expect("atualiza só o bloco interno");

        let apos_v2 = fs::read_to_string(&user).expect("lê após v2");
        assert!(apos_v2.contains("Cabeçalho do usuário"));
        assert!(apos_v2.contains("Idioma: pt-BR"));
        assert!(apos_v2.contains("- Prefere café sem açúcar."));
        assert!(apos_v2.contains("Rodapé do usuário"));

        fs::write(&user, format!("{inicio}\nSem marcador de fim\n")).expect("escreve inválido");
        let v3 = vec![ArquivoDesejado::new(
            user,
            "Tratamento: senhor\n".to_owned(),
            EstrategiaDeEscrita::BlocoDelimitado { inicio, fim },
        )];
        let err = planejar(v3, estado).expect_err("marcador único deve falhar");
        assert!(matches!(err, ErroDePlano::DelimitadoresInvalidos { .. }));
    }

    #[test]
    fn aplicar_com_forcar_nao_sobrescreve_backup_anterior() {
        let dir = DirTemporario::novo("backup-numerado");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let soul = dir.path().join("SOUL.md");
        let desejados = vec![ArquivoDesejado::new(
            soul.clone(),
            "# Gerenciado\n".to_owned(),
            EstrategiaDeEscrita::ArquivoIntegral,
        )];

        let p0 = planejar(desejados.clone(), estado.clone()).expect("plano inicial");
        aplicar(&p0, false).expect("aplica inicial");

        fs::write(&soul, "# Manual 1\n").expect("edição manual 1");
        let p1 = planejar(desejados.clone(), estado.clone()).expect("plano conflito 1");
        let r1 = aplicar(&p1, true).expect("forçar 1");
        assert_eq!(r1.backups_criados, vec![dir.path().join("SOUL.md.bak")]);

        fs::write(&soul, "# Manual 2\n").expect("edição manual 2");
        let p2 = planejar(desejados, estado).expect("plano conflito 2");
        let r2 = aplicar(&p2, true).expect("forçar 2");
        assert_eq!(r2.backups_criados, vec![dir.path().join("SOUL.md.bak.1")]);
        assert_eq!(
            fs::read_to_string(dir.path().join("SOUL.md.bak")).expect("lê backup 1"),
            "# Manual 1\n"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("SOUL.md.bak.1")).expect("lê backup 2"),
            "# Manual 2\n"
        );
    }

    #[test]
    fn aplicar_recupera_estado_quando_arquivo_integral_ja_bate_mas_estado_foi_perdido() {
        let dir = DirTemporario::novo("recupera-estado");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let soul = dir.path().join("SOUL.md");
        let v1 = vec![ArquivoDesejado::new(
            soul.clone(),
            "# V1\n".to_owned(),
            EstrategiaDeEscrita::ArquivoIntegral,
        )];

        let p1 = planejar(v1.clone(), estado.clone()).expect("plano v1");
        aplicar(&p1, false).expect("aplica v1");
        fs::remove_file(&estado).expect("remove estado");

        let p_rec = planejar(v1, estado.clone()).expect("plano recuperação");
        assert!(matches!(
            p_rec.acoes[0].operacao,
            OperacaoPlanejada::SemAlteracoes
        ));
        aplicar(&p_rec, false).expect("aplica recuperação");
        assert!(estado.exists(), "estado deve ser recriado");

        let v2 = vec![ArquivoDesejado::new(
            soul,
            "# V2\n".to_owned(),
            EstrategiaDeEscrita::ArquivoIntegral,
        )];
        let p2 = planejar(v2, estado).expect("plano v2");
        assert!(matches!(p2.acoes[0].operacao, OperacaoPlanejada::Atualizar));
    }

    #[test]
    fn aplicar_persiste_estado_parcial_quando_falha_no_meio() {
        let dir = DirTemporario::novo("estado-parcial");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let ok_path = dir.path().join("sub");
        let bloqueado = ok_path.join("b.md");

        let desejados = vec![
            ArquivoDesejado::new(
                ok_path.clone(),
                "# Ok\n".to_owned(),
                EstrategiaDeEscrita::ArquivoIntegral,
            ),
            ArquivoDesejado::new(
                bloqueado,
                "# Falha\n".to_owned(),
                EstrategiaDeEscrita::ArquivoIntegral,
            ),
        ];

        let plano = planejar(desejados, estado.clone()).expect("planeja");
        let err = aplicar(&plano, false).expect_err("escrita sob arquivo regular deve falhar");
        assert!(matches!(err, ErroDePlano::Io { .. }));
        assert!(ok_path.is_file(), "primeiro arquivo foi gravado");
        assert!(
            estado.exists(),
            "estado parcial deve registrar primeiro arquivo gravado"
        );
        let estado_toml = fs::read_to_string(&estado).expect("lê estado parcial");
        assert!(estado_toml.contains("sub"));
    }

    #[test]
    fn chave_estado_e_estavel_entre_caminho_relativo_e_absoluto() {
        let dir = DirTemporario::novo("chave-estavel");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let soul_redundante = dir.path().join("sub/../SOUL.md");
        let soul_limpo = dir.path().join("SOUL.md");

        assert_eq!(chave_estado(&soul_redundante, Some(dir.path())), "SOUL.md");
        let cwd = std::env::current_dir().expect("cwd");
        assert_eq!(
            chave_estado(Path::new("./sub/../SOUL.md"), Some(&cwd)),
            "SOUL.md"
        );

        let v1 = vec![ArquivoDesejado::new(
            soul_redundante,
            "# V1\n".to_owned(),
            EstrategiaDeEscrita::ArquivoIntegral,
        )];
        let p1 = planejar(v1, estado.clone()).expect("plano v1");
        aplicar(&p1, false).expect("aplica v1");

        let v2 = vec![ArquivoDesejado::new(
            soul_limpo,
            "# V2\n".to_owned(),
            EstrategiaDeEscrita::ArquivoIntegral,
        )];
        let p2 = planejar(v2, estado).expect("plano v2");
        assert!(matches!(p2.acoes[0].operacao, OperacaoPlanejada::Atualizar));
    }

    #[test]
    fn bloco_delimitado_rejeita_marcadores_duplicados() {
        let dir = DirTemporario::novo("marcadores-duplicados");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let user = dir.path().join("USER.md");
        let inicio = "<!-- sexta-feira:inicio -->";
        let fim = "<!-- sexta-feira:fim -->";

        fs::write(
            &user,
            format!("{inicio}\nbloco 1\n{fim}\n\n{inicio}\nbloco 2\n{fim}\n"),
        )
        .expect("grava arquivo com blocos duplicados");

        let desejados = vec![ArquivoDesejado::new(
            user,
            "Tratamento: senhor\n".to_owned(),
            EstrategiaDeEscrita::BlocoDelimitado { inicio, fim },
        )];

        let err = planejar(desejados, estado).expect_err("marcadores duplicados devem falhar");
        assert!(matches!(err, ErroDePlano::DelimitadoresInvalidos { .. }));
    }

    #[test]
    fn bloco_delimitado_nao_casa_marcador_com_prefixo_na_mesma_linha() {
        let dir = DirTemporario::novo("prefixo-sem-quebra");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let user = dir.path().join("USER.md");
        let inicio = "<!-- sexta-feira:inicio -->";
        let fim = "<!-- sexta-feira:fim -->";

        fs::write(
            &user,
            format!("Prefixo sem quebra{inicio}\nantigo\n{fim}\n"),
        )
        .expect("grava prefixo sem quebra");

        let desejados = vec![ArquivoDesejado::new(
            user.clone(),
            "novo\n".to_owned(),
            EstrategiaDeEscrita::BlocoDelimitado { inicio, fim },
        )];

        let err = planejar(desejados.clone(), estado.clone())
            .expect_err("marcador de inicio com prefixo na mesma linha deixa fim órfão");
        assert!(matches!(err, ErroDePlano::DelimitadoresInvalidos { .. }));

        fs::write(&user, format!("Prefixo {inicio} e {fim} na frase\n")).expect("grava frase");
        let plano = planejar(desejados, estado).expect("marcadores inline não interferem");
        aplicar(&plano, false).expect("aplica");

        let resultado = fs::read_to_string(&user).expect("lê resultado");
        assert_eq!(
            resultado,
            format!("{inicio}\nnovo\n{fim}\n\nPrefixo {inicio} e {fim} na frase\n")
        );
    }

    #[test]
    fn resumo_diff_detecta_remocao_de_linha_duplicada() {
        let diff = gerar_resumo_diff("a\nb\na\n", "a\nb\n");
        assert!(
            diff.lines().any(|linha| linha == "- a"),
            "esperava remoção da segunda ocorrência de `a`, obteve: {diff:?}"
        );
    }

    #[test]
    fn resumo_diff_detecta_reordenacao_de_linhas() {
        let diff = gerar_resumo_diff("linha 1\nlinha 2\n", "linha 2\nlinha 1\n");
        assert!(diff.contains("- "), "esperava remoção no diff: {diff:?}");
        assert!(diff.contains("+ "), "esperava adição no diff: {diff:?}");
        assert!(!diff.contains("formatação"));
    }

    #[test]
    fn resumo_diff_preserva_espaco_de_linha_vazia_removida() {
        let diff = gerar_resumo_diff("a\n\n", "a\n");
        assert_eq!(diff, "- ");
    }

    #[test]
    fn resumo_diff_com_prefixo_e_sufixo_comuns_grandes() {
        let mut antes = String::new();
        let mut depois = String::new();
        for idx in 0..3000 {
            let _ = writeln!(antes, "linha comum {idx}");
            let _ = writeln!(depois, "linha comum {idx}");
        }
        let _ = writeln!(antes, "valor antigo");
        let _ = writeln!(depois, "valor novo");
        for idx in 3000..6000 {
            let _ = writeln!(antes, "linha comum {idx}");
            let _ = writeln!(depois, "linha comum {idx}");
        }

        let diff = gerar_resumo_diff(&antes, &depois);
        assert_eq!(diff, "- valor antigo\n+ valor novo");
    }

    #[test]
    fn bloco_delimitado_rejeita_marcadores_assimetricos_e_invertidos() {
        let dir = DirTemporario::novo("marcadores-variacoes");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let user = dir.path().join("USER.md");
        let inicio = "<!-- sexta-feira:inicio -->";
        let fim = "<!-- sexta-feira:fim -->";

        for conteudo in [
            format!("{inicio}\nbloco\n{inicio}\n{fim}\n"),
            format!("{inicio}\nbloco\n{fim}\n{fim}\n"),
            format!("{fim}\ninvertido\n{inicio}\n"),
        ] {
            fs::write(&user, conteudo).expect("grava variação inválida");
            let desejados = vec![ArquivoDesejado::new(
                user.clone(),
                "ok\n".to_owned(),
                EstrategiaDeEscrita::BlocoDelimitado { inicio, fim },
            )];
            let err = planejar(desejados, estado.clone()).expect_err("deve rejeitar variação");
            assert!(matches!(err, ErroDePlano::DelimitadoresInvalidos { .. }));
        }
    }

    #[cfg(unix)]
    #[test]
    fn caminho_backup_nao_sobrescreve_symlink_quebrado() {
        let dir = DirTemporario::novo("backup-symlink");
        let soul = dir.path().join("SOUL.md");
        let bak = dir.path().join("SOUL.md.bak");
        std::os::unix::fs::symlink(dir.path().join("alvo-inexistente"), &bak)
            .expect("cria symlink quebrado em .bak");

        assert_eq!(caminho_backup(&soul), dir.path().join("SOUL.md.bak.1"));
    }

    #[test]
    fn carregar_estado_aceita_formato_legado_sem_versao_e_rejeita_versao_nao_suportada() {
        let dir = DirTemporario::novo("versao-estado");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let soul = dir.path().join("SOUL.md");
        fs::write(
            &estado,
            format!(
                "[arquivos_integrais]\n\"SOUL.md\" = \"{}\"\n",
                checksum_conteudo("# V1\n")
            ),
        )
        .expect("grava estado legado");
        fs::write(&soul, "# V1\n").expect("grava soul");

        let plano = planejar(
            vec![ArquivoDesejado::new(
                soul.clone(),
                "# V2\n".to_owned(),
                EstrategiaDeEscrita::ArquivoIntegral,
            )],
            estado.clone(),
        )
        .expect("estado legado sem versao deve ser aceito");
        assert!(matches!(
            plano.acoes[0].operacao,
            OperacaoPlanejada::Atualizar
        ));
        aplicar(&plano, false).expect("grava estado versionado");
        let estado_salvo = fs::read_to_string(&estado).expect("le estado");
        assert!(estado_salvo.contains("versao = 1"));

        fs::write(&estado, "versao = 99\n").expect("grava versao futura");
        let err = planejar(
            vec![ArquivoDesejado::new(
                soul,
                "# V2\n".to_owned(),
                EstrategiaDeEscrita::ArquivoIntegral,
            )],
            estado,
        )
        .expect_err("versao futura deve falhar");
        assert!(matches!(
            err,
            ErroDePlano::VersaoDeEstadoNaoSuportada { versao: 99, .. }
        ));
    }

    #[test]
    fn bloco_delimitado_ignora_marcador_inline_e_espacos_a_direita_no_marcador() {
        let dir = DirTemporario::novo("marcador-por-linha");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let user = dir.path().join("USER.md");
        let inicio = "<!-- sexta-feira:inicio -->";
        let fim = "<!-- sexta-feira:fim -->";

        fs::write(
            &user,
            format!("{inicio}  \nantigo\n{fim}\t \nNota: usar `{inicio}` no exemplo.\n"),
        )
        .expect("grava arquivo");

        let desejados = vec![ArquivoDesejado::new(
            user.clone(),
            "novo\n".to_owned(),
            EstrategiaDeEscrita::BlocoDelimitado { inicio, fim },
        )];
        let plano = planejar(desejados, estado).expect("nao deve confundir marcador inline");
        aplicar(&plano, false).expect("aplica");

        let final_txt = fs::read_to_string(&user).expect("le final");
        assert_eq!(
            final_txt,
            format!("{inicio}\nnovo\n{fim}\nNota: usar `{inicio}` no exemplo.\n")
        );
    }

    #[test]
    fn resumo_diff_usa_fallback_linear_acima_do_limite_lcs() {
        let mut antes = String::new();
        let mut depois = String::new();
        for idx in 0..600 {
            let _ = writeln!(antes, "antiga {idx}");
            let _ = writeln!(depois, "nova {idx}");
        }
        let diff = gerar_resumo_diff(&antes, &depois);
        assert!(diff.contains("- antiga 0"));
        assert!(diff.contains("+ nova 599"));
    }

    #[cfg(unix)]
    #[test]
    fn arquivo_executavel_aplica_e_verifica_permissao_unix() {
        use std::os::unix::fs::PermissionsExt;

        let dir = DirTemporario::novo("permissao-exec");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let script = dir.path().join("bin/rotina.sh");

        let desejados = vec![
            ArquivoDesejado::new(
                script.clone(),
                "#!/bin/sh\necho ok\n".to_owned(),
                EstrategiaDeEscrita::ArquivoIntegral,
            )
            .com_permissao(PermissaoDeArquivo::Executavel),
        ];

        let p1 = planejar(desejados.clone(), estado.clone()).expect("plano criacao");
        aplicar(&p1, false).expect("aplica script");
        let mode = fs::metadata(&script).expect("meta").permissions().mode();
        assert_ne!(mode & 0o111, 0, "script criado deve ter bit +x");
        assert!(
            verificar(&desejados, &estado)
                .expect("verifica")
                .sincronizado
        );

        let mut perms = fs::metadata(&script).expect("meta").permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&script, perms).expect("remove +x");

        let verif_sem_x = verificar(&desejados, &estado).expect("verifica sem +x");
        assert!(!verif_sem_x.sincronizado);
        assert!(verif_sem_x.itens[0].detalhe.contains("permissão"));

        let p2 = planejar(desejados.clone(), estado.clone()).expect("plano ajuste +x");
        assert!(matches!(p2.acoes[0].operacao, OperacaoPlanejada::Atualizar));
        let res = aplicar(&p2, false).expect("reaplica +x");
        assert_eq!(res.atualizados, 1);
        assert!(
            verificar(&desejados, &estado)
                .expect("verifica final")
                .sincronizado
        );
    }

    #[test]
    fn poda_arquivo_integral_orfao_com_seguranca_e_respeita_conflito() {
        let dir = DirTemporario::novo("poda-orfao");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let a = dir.path().join("A.md");
        let b = dir.path().join("B.md");

        let v1 = vec![
            ArquivoDesejado::new(
                a.clone(),
                "# A\n".to_owned(),
                EstrategiaDeEscrita::ArquivoIntegral,
            ),
            ArquivoDesejado::new(
                b.clone(),
                "# B\n".to_owned(),
                EstrategiaDeEscrita::ArquivoIntegral,
            ),
        ];
        let p1 = planejar(v1, estado.clone()).expect("plano inicial");
        aplicar(&p1, false).expect("aplica inicial");

        let v2 = vec![ArquivoDesejado::new(
            a.clone(),
            "# A\n".to_owned(),
            EstrategiaDeEscrita::ArquivoIntegral,
        )];
        let p_rem = planejar(v2.clone(), estado.clone()).expect("plano remocao");
        assert!(
            p_rem
                .acoes
                .iter()
                .any(|ac| ac.caminho == b && matches!(ac.operacao, OperacaoPlanejada::Remover))
        );
        assert!(
            !verificar(&v2, &estado)
                .expect("verifica antes")
                .sincronizado
        );

        aplicar(&p_rem, false).expect("aplica remocao");
        assert!(!b.exists(), "arquivo orfao deve ser removido");
        assert!(
            verificar(&v2, &estado)
                .expect("verifica depois")
                .sincronizado
        );

        // Agora testa conflito quando orfao foi editado manualmente antes da poda
        let v3 = vec![
            ArquivoDesejado::new(
                a.clone(),
                "# A\n".to_owned(),
                EstrategiaDeEscrita::ArquivoIntegral,
            ),
            ArquivoDesejado::new(
                b.clone(),
                "# B\n".to_owned(),
                EstrategiaDeEscrita::ArquivoIntegral,
            ),
        ];
        aplicar(&planejar(v3, estado.clone()).expect("recria B"), false).expect("aplica B");
        fs::write(&b, "# B editado manualmente\n").expect("edita B");

        let p_conflito = planejar(v2.clone(), estado.clone()).expect("plano conflito orfao");
        assert!(p_conflito.acoes.iter().any(|ac| {
            ac.caminho == b && matches!(ac.operacao, OperacaoPlanejada::Conflito { .. })
        }));
        assert!(matches!(
            aplicar(&p_conflito, false),
            Err(ErroDePlano::ConflitoPendente { .. })
        ));
        assert!(b.exists(), "sem --forcar nao pode apagar arquivo editado");

        let res_forcar = aplicar(&p_conflito, true).expect("forcar remocao com backup");
        assert!(!b.exists());
        assert_eq!(
            res_forcar.backups_criados,
            vec![dir.path().join("B.md.bak")]
        );
        assert!(
            verificar(&v2, &estado)
                .expect("verifica final")
                .sincronizado
        );
    }

    #[test]
    fn mesclagem_estruturada_json_preserva_chaves_usuario_e_detecta_conflito_em_chaves_gerenciadas()
    {
        let dir = DirTemporario::novo("merge-json");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let cfg = dir.path().join("settings.json");

        fs::write(
            &cfg,
            "{\n    \"tema\": \"escuro\",\n    \"mcpServers\": {\n        \"outro\": { \"cmd\": \"x\" }\n    }\n}\n",
        )
        .expect("grava json previo");

        let desejados_v1 = vec![ArquivoDesejado::new(
            cfg.clone(),
            r#"{"mcpServers":{"sexta-feira":{"command":"sexta-feira"}}}"#.to_owned(),
            EstrategiaDeEscrita::MesclagemEstruturada {
                formato: FormatoEstruturado::Json,
            },
        )];

        let p1 = planejar(desejados_v1.clone(), estado.clone()).expect("plano v1");
        assert!(matches!(p1.acoes[0].operacao, OperacaoPlanejada::Atualizar));
        aplicar(&p1, false).expect("aplica v1");

        let lido: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&cfg).expect("le cfg")).expect("json valido");
        assert_eq!(lido["tema"], "escuro");
        assert_eq!(lido["mcpServers"]["outro"]["cmd"], "x");
        assert_eq!(lido["mcpServers"]["sexta-feira"]["command"], "sexta-feira");
        assert!(
            verificar(&desejados_v1, &estado)
                .expect("verifica")
                .sincronizado
        );

        // Usuário altera chave própria mantendo formatação customizada: deve continuar SemAlteracoes
        let custom = fs::read_to_string(&cfg)
            .expect("le")
            .replace("\"escuro\"", "\"claro\"");
        fs::write(&cfg, custom).expect("altera chave do usuario");
        let p_user =
            planejar(desejados_v1.clone(), estado.clone()).expect("plano apos chave propria");
        assert!(matches!(
            p_user.acoes[0].operacao,
            OperacaoPlanejada::SemAlteracoes
        ));

        // Usuário altera chave gerenciada: deve gerar Conflito e exigir --forcar
        let adulterado = fs::read_to_string(&cfg)
            .expect("le")
            .replace("\"command\": \"sexta-feira\"", "\"command\": \"manual\"");
        fs::write(&cfg, adulterado).expect("adultera chave gerenciada");
        let p_conflito = planejar(desejados_v1, estado.clone()).expect("plano conflito");
        assert!(matches!(
            p_conflito.acoes[0].operacao,
            OperacaoPlanejada::Conflito { .. }
        ));
        assert!(matches!(
            aplicar(&p_conflito, false),
            Err(ErroDePlano::ConflitoPendente { .. })
        ));

        let res_forcar = aplicar(&p_conflito, true).expect("forcar merge");
        assert_eq!(res_forcar.backups_criados.len(), 1);
        let final_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&cfg).expect("le final"))
                .expect("json valido");
        assert_eq!(final_json["tema"], "claro");
        assert_eq!(
            final_json["mcpServers"]["sexta-feira"]["command"],
            "sexta-feira"
        );
    }

    #[test]
    fn mesclagem_estruturada_rejeita_colisao_preexistente_sem_estado_e_suporta_toml() {
        let dir = DirTemporario::novo("merge-toml");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let cfg = dir.path().join("config.toml");

        fs::write(
            &cfg,
            "[usuario]\ntratamento = \"manual\"\n[extra]\nmanter = true\n",
        )
        .expect("toml previo");
        let desejados = vec![ArquivoDesejado::new(
            cfg.clone(),
            "[usuario]\ntratamento = \"senhor\"\n".to_owned(),
            EstrategiaDeEscrita::MesclagemEstruturada {
                formato: FormatoEstruturado::Toml,
            },
        )];

        let p_colisao = planejar(desejados.clone(), estado.clone()).expect("plano colisao");
        assert!(matches!(
            p_colisao.acoes[0].operacao,
            OperacaoPlanejada::Conflito { .. }
        ));
        aplicar(&p_colisao, true).expect("forca sobre colisao");

        let conteudo = fs::read_to_string(&cfg).expect("le toml");
        assert!(conteudo.contains("manter = true"));
        assert!(conteudo.contains("tratamento = \"senhor\""));
        assert!(
            verificar(&desejados, &estado)
                .expect("verifica")
                .sincronizado
        );

        // JSON preexistente com colisão e JSON inválido em disco também devem gerar Conflito
        let json_cfg = dir.path().join("colisao.json");
        fs::write(
            &json_cfg,
            r#"{"tema":"escuro","mcpServers":{"sexta":"antigo"}}"#,
        )
        .expect("json previo");
        let json_desejado = vec![ArquivoDesejado::new(
            json_cfg.clone(),
            r#"{"mcpServers":{"sexta":"novo"}}"#.to_owned(),
            EstrategiaDeEscrita::MesclagemEstruturada {
                formato: FormatoEstruturado::Json,
            },
        )];
        let p_json_col =
            planejar(json_desejado.clone(), estado.clone()).expect("plano colisao json");
        assert!(matches!(
            p_json_col.acoes[0].operacao,
            OperacaoPlanejada::Conflito { .. }
        ));
        aplicar(&p_json_col, true).expect("forca sobre colisao json");
        let json_final: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&json_cfg).expect("le json")).expect("valido");
        assert_eq!(json_final["tema"], "escuro");
        assert_eq!(json_final["mcpServers"]["sexta"], "novo");

        fs::write(&json_cfg, "{ quebrado").expect("json corrompido");
        let p_quebrado = planejar(json_desejado, estado).expect("plano json corrompido");
        assert!(matches!(
            p_quebrado.acoes[0].operacao,
            OperacaoPlanejada::Conflito { .. }
        ));
    }
}
