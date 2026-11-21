//! Planejamento, verificação e aplicação declarativa de arquivos de setup.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

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
    /// Resumo textual legível das mudanças previstas, quando houver.
    pub resumo_diff: Option<String>,
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
    /// Falha ao serializar o arquivo de estado para TOML.
    #[error("não foi possível serializar o estado do setup: {0}")]
    SerializacaoDeEstado(#[source] toml::ser::Error),
    /// Marcadores de bloco delimitado ausentes parcialmente ou fora de ordem.
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
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct EstadoAplicado {
    #[serde(default)]
    arquivos_integrais: BTreeMap<String, String>,
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

    for arquivo in arquivos {
        let atual = ler_opcional(&arquivo.caminho)?;
        let acao = match arquivo.estrategia {
            EstrategiaDeEscrita::ArquivoIntegral => {
                planejar_integral(&arquivo, atual.as_deref(), &estado, raiz_estado)
            }
            EstrategiaDeEscrita::BlocoDelimitado { inicio, fim } => {
                planejar_bloco_delimitado(&arquivo, atual.as_deref(), inicio, fim)?
            }
        };
        acoes.push(acao);
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

    for acao in &plano.acoes {
        let resultado_acao =
            aplicar_acao(acao, &mut criados, &mut atualizados, &mut backups_criados);
        if let Err(erro) = resultado_acao {
            if alterou_estado {
                let _ = salvar_estado(&plano.arquivo_estado, &estado);
            }
            return Err(erro);
        }

        if acao.estrategia == EstrategiaDeEscrita::ArquivoIntegral {
            let chave = chave_estado(&acao.caminho, raiz_estado);
            let checksum = checksum_conteudo(&acao.conteudo_final);
            if estado.arquivos_integrais.get(&chave) != Some(&checksum) {
                estado.arquivos_integrais.insert(chave, checksum);
                alterou_estado = true;
            }
        }
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
            *criados += 1;
            Ok(())
        }
        OperacaoPlanejada::Atualizar => {
            gravar_arquivo(&acao.caminho, &acao.conteudo_final)?;
            *atualizados += 1;
            Ok(())
        }
        OperacaoPlanejada::Conflito { .. } => {
            if acao.caminho.exists() {
                let backup = caminho_backup(&acao.caminho);
                fs::copy(&acao.caminho, &backup).map_err(|fonte| ErroDePlano::Io {
                    caminho: backup.clone(),
                    fonte,
                })?;
                backups_criados.push(backup);
            }
            gravar_arquivo(&acao.caminho, &acao.conteudo_final)?;
            *atualizados += 1;
            Ok(())
        }
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
            OperacaoPlanejada::Atualizar => (false, "atualização pendente".to_owned()),
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
) -> AcaoPlanejada {
    let desejado = normalizar_quebras(&arquivo.conteudo);
    let Some(conteudo_atual) = atual else {
        let resumo_diff = Some(gerar_resumo_diff("", &desejado));
        return AcaoPlanejada {
            caminho: arquivo.caminho.clone(),
            operacao: OperacaoPlanejada::Criar,
            conteudo_final: desejado,
            estrategia: arquivo.estrategia,
            resumo_diff,
        };
    };

    let atual_normalizado = normalizar_quebras(conteudo_atual);
    if atual_normalizado == desejado {
        return AcaoPlanejada {
            caminho: arquivo.caminho.clone(),
            operacao: OperacaoPlanejada::SemAlteracoes,
            conteudo_final: desejado,
            estrategia: arquivo.estrategia,
            resumo_diff: None,
        };
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
    AcaoPlanejada {
        caminho: arquivo.caminho.clone(),
        operacao,
        conteudo_final: desejado,
        estrategia: arquivo.estrategia,
        resumo_diff,
    }
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
            (OperacaoPlanejada::SemAlteracoes, None)
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
        resumo_diff,
    })
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

    let qtd_inicio = texto.matches(inicio).count();
    let qtd_fim = texto.matches(fim).count();

    match (qtd_inicio, qtd_fim) {
        (0, 0) => {
            if texto.trim().is_empty() {
                Ok(bloco_completo)
            } else {
                Ok(format!(
                    "{bloco_completo}\n{}",
                    texto.trim_start_matches('\n')
                ))
            }
        }
        (1, 1)
            if let Some(i) = texto.find(inicio)
                && let Some(f) = texto.find(fim)
                && i + inicio.len() <= f =>
        {
            let antes = &texto[..i];
            let separador = if antes.is_empty() || antes.ends_with('\n') {
                ""
            } else {
                "\n"
            };
            let restante = &texto[f + fim.len()..];
            let depois = restante.strip_prefix('\n').unwrap_or(restante);
            Ok(format!("{antes}{separador}{bloco_completo}{depois}"))
        }
        _ => Err(ErroDePlano::DelimitadoresInvalidos {
            caminho: caminho.to_path_buf(),
            inicio,
            fim,
        }),
    }
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

    let seq = CONTADOR_TEMP_ESCRITA.fetch_add(1, Ordering::Relaxed);
    let nome_temp = format!(".sexta-feira-tmp-{}-{seq}", std::process::id());
    let caminho_temp = dir_pai.join(nome_temp);

    if let Err(fonte) = fs::write(&caminho_temp, conteudo) {
        let _ = fs::remove_file(&caminho_temp);
        return Err(ErroDePlano::Io {
            caminho: caminho.to_path_buf(),
            fonte,
        });
    }

    if let Err(fonte) = fs::rename(&caminho_temp, caminho) {
        let _ = fs::remove_file(&caminho_temp);
        return Err(ErroDePlano::Io {
            caminho: caminho.to_path_buf(),
            fonte,
        });
    }

    Ok(())
}

fn carregar_estado(caminho: &Path) -> Result<EstadoAplicado, ErroDePlano> {
    let Some(conteudo) = ler_opcional(caminho)? else {
        return Ok(EstadoAplicado::default());
    };
    toml::from_str(&conteudo).map_err(|fonte| ErroDePlano::EstadoInvalido {
        caminho: caminho.to_path_buf(),
        fonte,
    })
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
    let base = format!("{}.bak", caminho.display());
    let inicial = PathBuf::from(&base);
    if !inicial.exists() {
        return inicial;
    }

    for indice in 1usize.. {
        let candidato = PathBuf::from(format!("{base}.{indice}"));
        if !candidato.exists() {
            return candidato;
        }
    }

    inicial
}

fn normalizar_quebras(texto: &str) -> String {
    texto.replace("\r\n", "\n")
}

fn checksum_conteudo(conteudo: &str) -> String {
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
    let mut saida = String::new();

    for linha in &linhas_antes {
        if !linhas_depois.contains(linha) {
            let _ = writeln!(saida, "- {linha}");
        }
    }
    for linha in &linhas_depois {
        if !linhas_antes.contains(linha) {
            let _ = writeln!(saida, "+ {linha}");
        }
    }

    if saida.is_empty() {
        let _ = writeln!(saida, "~ alteração de formatação ou quebras de linha");
    }
    saida.trim_end().to_owned()
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
            ArquivoDesejado {
                caminho: soul.clone(),
                conteudo: "# Sexta-feira\n".to_owned(),
                estrategia: EstrategiaDeEscrita::ArquivoIntegral,
            },
            ArquivoDesejado {
                caminho: user.clone(),
                conteudo: "Tratamento: senhor\n".to_owned(),
                estrategia: EstrategiaDeEscrita::BlocoDelimitado {
                    inicio: "<!-- inicio -->",
                    fim: "<!-- fim -->",
                },
            },
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
            ArquivoDesejado {
                caminho: novo.clone(),
                conteudo: "Tratamento: senhor\n".to_owned(),
                estrategia: EstrategiaDeEscrita::BlocoDelimitado {
                    inicio: "<!-- inicio -->",
                    fim: "<!-- fim -->",
                },
            },
            ArquivoDesejado {
                caminho: existente,
                conteudo: "# Sexta-feira\n".to_owned(),
                estrategia: EstrategiaDeEscrita::ArquivoIntegral,
            },
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

        let desejados = vec![ArquivoDesejado {
            caminho: soul.clone(),
            conteudo: "# Versão gerenciada\n".to_owned(),
            estrategia: EstrategiaDeEscrita::ArquivoIntegral,
        }];

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

        let v1 = vec![ArquivoDesejado {
            caminho: user.clone(),
            conteudo: "Tratamento: senhor\n".to_owned(),
            estrategia: EstrategiaDeEscrita::BlocoDelimitado { inicio, fim },
        }];
        let p1 = planejar(v1, estado.clone()).expect("plano sem marcadores");
        assert!(matches!(p1.acoes[0].operacao, OperacaoPlanejada::Atualizar));
        aplicar(&p1, false).expect("aplica bloco no topo");

        let apos_v1 = fs::read_to_string(&user).expect("lê após v1");
        assert!(apos_v1.contains("Tratamento: senhor"));
        assert!(apos_v1.contains("- Prefere café sem açúcar."));

        let com_prefixo = format!("Cabeçalho do usuário\n\n{apos_v1}\nRodapé do usuário\n");
        fs::write(&user, com_prefixo).expect("adiciona antes e depois");

        let v2 = vec![ArquivoDesejado {
            caminho: user.clone(),
            conteudo: "Tratamento: senhor\nIdioma: pt-BR\n".to_owned(),
            estrategia: EstrategiaDeEscrita::BlocoDelimitado { inicio, fim },
        }];
        let p2 = planejar(v2, estado.clone()).expect("plano com marcadores");
        aplicar(&p2, false).expect("atualiza só o bloco interno");

        let apos_v2 = fs::read_to_string(&user).expect("lê após v2");
        assert!(apos_v2.contains("Cabeçalho do usuário"));
        assert!(apos_v2.contains("Idioma: pt-BR"));
        assert!(apos_v2.contains("- Prefere café sem açúcar."));
        assert!(apos_v2.contains("Rodapé do usuário"));

        fs::write(&user, format!("{inicio}\nSem marcador de fim\n")).expect("escreve inválido");
        let v3 = vec![ArquivoDesejado {
            caminho: user,
            conteudo: "Tratamento: senhor\n".to_owned(),
            estrategia: EstrategiaDeEscrita::BlocoDelimitado { inicio, fim },
        }];
        let err = planejar(v3, estado).expect_err("marcador único deve falhar");
        assert!(matches!(err, ErroDePlano::DelimitadoresInvalidos { .. }));
    }

    #[test]
    fn aplicar_com_forcar_nao_sobrescreve_backup_anterior() {
        let dir = DirTemporario::novo("backup-numerado");
        let estado = dir.path().join(".sexta-feira-estado.toml");
        let soul = dir.path().join("SOUL.md");
        let desejados = vec![ArquivoDesejado {
            caminho: soul.clone(),
            conteudo: "# Gerenciado\n".to_owned(),
            estrategia: EstrategiaDeEscrita::ArquivoIntegral,
        }];

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
        let v1 = vec![ArquivoDesejado {
            caminho: soul.clone(),
            conteudo: "# V1\n".to_owned(),
            estrategia: EstrategiaDeEscrita::ArquivoIntegral,
        }];

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

        let v2 = vec![ArquivoDesejado {
            caminho: soul,
            conteudo: "# V2\n".to_owned(),
            estrategia: EstrategiaDeEscrita::ArquivoIntegral,
        }];
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
            ArquivoDesejado {
                caminho: ok_path.clone(),
                conteudo: "# Ok\n".to_owned(),
                estrategia: EstrategiaDeEscrita::ArquivoIntegral,
            },
            ArquivoDesejado {
                caminho: bloqueado,
                conteudo: "# Falha\n".to_owned(),
                estrategia: EstrategiaDeEscrita::ArquivoIntegral,
            },
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

        let v1 = vec![ArquivoDesejado {
            caminho: soul_redundante,
            conteudo: "# V1\n".to_owned(),
            estrategia: EstrategiaDeEscrita::ArquivoIntegral,
        }];
        let p1 = planejar(v1, estado.clone()).expect("plano v1");
        aplicar(&p1, false).expect("aplica v1");

        let v2 = vec![ArquivoDesejado {
            caminho: soul_limpo,
            conteudo: "# V2\n".to_owned(),
            estrategia: EstrategiaDeEscrita::ArquivoIntegral,
        }];
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

        let desejados = vec![ArquivoDesejado {
            caminho: user,
            conteudo: "Tratamento: senhor\n".to_owned(),
            estrategia: EstrategiaDeEscrita::BlocoDelimitado { inicio, fim },
        }];

        let err = planejar(desejados, estado).expect_err("marcadores duplicados devem falhar");
        assert!(matches!(err, ErroDePlano::DelimitadoresInvalidos { .. }));
    }

    #[test]
    fn bloco_delimitado_separa_linha_quando_prefixo_nao_termina_com_quebra() {
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

        let desejados = vec![ArquivoDesejado {
            caminho: user.clone(),
            conteudo: "novo\n".to_owned(),
            estrategia: EstrategiaDeEscrita::BlocoDelimitado { inicio, fim },
        }];

        let plano = planejar(desejados, estado).expect("planeja");
        aplicar(&plano, false).expect("aplica");

        let resultado = fs::read_to_string(&user).expect("lê resultado");
        assert_eq!(
            resultado,
            format!("Prefixo sem quebra\n{inicio}\nnovo\n{fim}\n")
        );
    }
}
