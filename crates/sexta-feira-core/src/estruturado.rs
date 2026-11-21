//! Suporte interno à mesclagem estruturada de documentos JSON e TOML.

use std::path::Path;

use crate::plano::{ErroDePlano, FormatoEstruturado, OperacaoPlanejada, checksum_conteudo};

pub(crate) struct ResultadoPlanejamentoEstruturado {
    pub(crate) operacao: OperacaoPlanejada,
    pub(crate) conteudo_final: String,
    pub(crate) checksum_patch: String,
    pub(crate) texto_anterior: Option<String>,
}

pub(crate) fn planejar_documento(
    formato: FormatoEstruturado,
    caminho: &Path,
    patch_texto: &str,
    atual: Option<&str>,
    checksum_registrado: Option<&str>,
) -> Result<ResultadoPlanejamentoEstruturado, ErroDePlano> {
    match formato {
        FormatoEstruturado::Json => {
            planejar_com::<serde_json::Value>(caminho, patch_texto, atual, checksum_registrado)
        }
        FormatoEstruturado::Toml => {
            planejar_com::<toml::Value>(caminho, patch_texto, atual, checksum_registrado)
        }
    }
}

trait ValorDocumento: Sized + PartialEq + Clone {
    type Tabela: PartialEq + Clone;

    fn interpretar_raiz(texto: &str) -> Result<Self::Tabela, String>;
    fn tabela_vazia() -> Self::Tabela;
    fn como_tabela(&self) -> Option<&Self::Tabela>;
    fn como_tabela_mut(&mut self) -> Option<&mut Self::Tabela>;
    fn de_tabela(tabela: Self::Tabela) -> Self;
    fn serializar_bonito(tabela: &Self::Tabela) -> Result<String, String>;
    fn serializar_canonico(tabela: &Self::Tabela) -> Result<String, String>;
    fn iter_tabela<'a>(tabela: &'a Self::Tabela) -> impl Iterator<Item = (&'a str, &'a Self)>
    where
        Self: 'a;
    fn get_tabela<'a>(tabela: &'a Self::Tabela, chave: &str) -> Option<&'a Self>;
    fn get_mut_tabela<'a>(tabela: &'a mut Self::Tabela, chave: &str) -> Option<&'a mut Self>;
    fn insert_tabela(tabela: &mut Self::Tabela, chave: String, valor: Self);
}

impl ValorDocumento for serde_json::Value {
    type Tabela = serde_json::Map<String, Self>;

    fn interpretar_raiz(texto: &str) -> Result<Self::Tabela, String> {
        let valor: Self = serde_json::from_str(texto).map_err(|e| e.to_string())?;
        match valor {
            Self::Object(mapa) => Ok(mapa),
            _ => Err("a raiz do documento JSON deve ser um objeto".to_owned()),
        }
    }

    fn tabela_vazia() -> Self::Tabela {
        Self::Tabela::new()
    }

    fn como_tabela(&self) -> Option<&Self::Tabela> {
        self.as_object()
    }

    fn como_tabela_mut(&mut self) -> Option<&mut Self::Tabela> {
        self.as_object_mut()
    }

    fn de_tabela(tabela: Self::Tabela) -> Self {
        Self::Object(tabela)
    }

    fn serializar_bonito(tabela: &Self::Tabela) -> Result<String, String> {
        serde_json::to_string_pretty(tabela)
            .map(|s| normalizar_saida(&s))
            .map_err(|e| e.to_string())
    }

    fn serializar_canonico(tabela: &Self::Tabela) -> Result<String, String> {
        serde_json::to_string(tabela).map_err(|e| e.to_string())
    }

    fn iter_tabela<'a>(tabela: &'a Self::Tabela) -> impl Iterator<Item = (&'a str, &'a Self)>
    where
        Self: 'a,
    {
        tabela.iter().map(|(k, v)| (k.as_str(), v))
    }

    fn get_tabela<'a>(tabela: &'a Self::Tabela, chave: &str) -> Option<&'a Self> {
        tabela.get(chave)
    }

    fn get_mut_tabela<'a>(tabela: &'a mut Self::Tabela, chave: &str) -> Option<&'a mut Self> {
        tabela.get_mut(chave)
    }

    fn insert_tabela(tabela: &mut Self::Tabela, chave: String, valor: Self) {
        tabela.insert(chave, valor);
    }
}

impl ValorDocumento for toml::Value {
    type Tabela = toml::map::Map<String, Self>;

    fn interpretar_raiz(texto: &str) -> Result<Self::Tabela, String> {
        toml::from_str(texto).map_err(|e| e.to_string())
    }

    fn tabela_vazia() -> Self::Tabela {
        Self::Tabela::new()
    }

    fn como_tabela(&self) -> Option<&Self::Tabela> {
        self.as_table()
    }

    fn como_tabela_mut(&mut self) -> Option<&mut Self::Tabela> {
        self.as_table_mut()
    }

    fn de_tabela(tabela: Self::Tabela) -> Self {
        Self::Table(tabela)
    }

    fn serializar_bonito(tabela: &Self::Tabela) -> Result<String, String> {
        toml::to_string_pretty(tabela)
            .map(|s| normalizar_saida(&s))
            .map_err(|e| e.to_string())
    }

    fn serializar_canonico(tabela: &Self::Tabela) -> Result<String, String> {
        toml::to_string(tabela).map_err(|e| e.to_string())
    }

    fn iter_tabela<'a>(tabela: &'a Self::Tabela) -> impl Iterator<Item = (&'a str, &'a Self)>
    where
        Self: 'a,
    {
        tabela.iter().map(|(k, v)| (k.as_str(), v))
    }

    fn get_tabela<'a>(tabela: &'a Self::Tabela, chave: &str) -> Option<&'a Self> {
        tabela.get(chave)
    }

    fn get_mut_tabela<'a>(tabela: &'a mut Self::Tabela, chave: &str) -> Option<&'a mut Self> {
        tabela.get_mut(chave)
    }

    fn insert_tabela(tabela: &mut Self::Tabela, chave: String, valor: Self) {
        tabela.insert(chave, valor);
    }
}

fn planejar_com<T: ValorDocumento>(
    caminho: &Path,
    patch_texto: &str,
    atual: Option<&str>,
    checksum_registrado: Option<&str>,
) -> Result<ResultadoPlanejamentoEstruturado, ErroDePlano> {
    let patch = T::interpretar_raiz(patch_texto).map_err(|detalhe| {
        ErroDePlano::ConteudoEstruturadoInvalido {
            caminho: caminho.to_path_buf(),
            detalhe,
        }
    })?;

    let patch_canonico = T::serializar_canonico(&patch).map_err(|detalhe| {
        ErroDePlano::ConteudoEstruturadoInvalido {
            caminho: caminho.to_path_buf(),
            detalhe,
        }
    })?;
    let checksum_patch = checksum_conteudo(&patch_canonico);

    let patch_bonito = T::serializar_bonito(&patch).map_err(|detalhe| {
        ErroDePlano::ConteudoEstruturadoInvalido {
            caminho: caminho.to_path_buf(),
            detalhe,
        }
    })?;

    let Some(texto_atual) = atual else {
        return Ok(ResultadoPlanejamentoEstruturado {
            operacao: OperacaoPlanejada::Criar,
            conteudo_final: patch_bonito,
            checksum_patch,
            texto_anterior: None,
        });
    };

    let atual_normalizado = texto_atual.replace("\r\n", "\n");
    let atual_tabela = if atual_normalizado.trim().is_empty() {
        T::tabela_vazia()
    } else {
        match T::interpretar_raiz(&atual_normalizado) {
            Ok(tabela) => tabela,
            Err(_) => {
                return Ok(ResultadoPlanejamentoEstruturado {
                    operacao: OperacaoPlanejada::Conflito {
                        motivo: "o arquivo estruturado existente não pôde ser interpretado no formato esperado"
                            .to_owned(),
                    },
                    conteudo_final: patch_bonito,
                    checksum_patch,
                    texto_anterior: Some(atual_normalizado),
                });
            }
        }
    };

    let mut mesclado = atual_tabela.clone();
    mesclar_tabelas::<T>(&mut mesclado, &patch);

    if mesclado == atual_tabela {
        return Ok(ResultadoPlanejamentoEstruturado {
            operacao: OperacaoPlanejada::SemAlteracoes,
            conteudo_final: atual_normalizado.clone(),
            checksum_patch,
            texto_anterior: Some(atual_normalizado),
        });
    }

    let conteudo_final = T::serializar_bonito(&mesclado).map_err(|detalhe| {
        ErroDePlano::ConteudoEstruturadoInvalido {
            caminho: caminho.to_path_buf(),
            detalhe,
        }
    })?;

    let projecao = projetar_tabela::<T>(&atual_tabela, &patch);
    let projecao_canonica = T::serializar_canonico(&projecao).map_err(|detalhe| {
        ErroDePlano::ConteudoEstruturadoInvalido {
            caminho: caminho.to_path_buf(),
            detalhe,
        }
    })?;
    let checksum_projecao = checksum_conteudo(&projecao_canonica);

    let operacao = match checksum_registrado {
        Some(reg) if reg == checksum_projecao => OperacaoPlanejada::Atualizar,
        Some(_) => OperacaoPlanejada::Conflito {
            motivo: "chaves gerenciadas foram modificadas manualmente desde a última aplicação"
                .to_owned(),
        },
        None if tem_colisao_inicial::<T>(&atual_tabela, &patch) => OperacaoPlanejada::Conflito {
            motivo: "chaves gerenciadas já existiam com valores distintos antes do setup"
                .to_owned(),
        },
        None => OperacaoPlanejada::Atualizar,
    };

    Ok(ResultadoPlanejamentoEstruturado {
        operacao,
        conteudo_final,
        checksum_patch,
        texto_anterior: Some(atual_normalizado),
    })
}

fn mesclar_tabelas<T: ValorDocumento>(destino: &mut T::Tabela, patch: &T::Tabela) {
    for (chave, valor_patch) in T::iter_tabela(patch) {
        if let Some(sub_patch) = valor_patch.como_tabela()
            && let Some(existente) = T::get_mut_tabela(destino, chave)
            && let Some(sub_dest) = existente.como_tabela_mut()
        {
            mesclar_tabelas::<T>(sub_dest, sub_patch);
            continue;
        }
        T::insert_tabela(destino, chave.to_owned(), valor_patch.clone());
    }
}

fn projetar_tabela<T: ValorDocumento>(atual: &T::Tabela, patch: &T::Tabela) -> T::Tabela {
    let mut saida = T::tabela_vazia();
    for (chave, valor_patch) in T::iter_tabela(patch) {
        let Some(valor_atual) = T::get_tabela(atual, chave) else {
            continue;
        };
        if let Some(sub_patch) = valor_patch.como_tabela()
            && let Some(sub_atual) = valor_atual.como_tabela()
        {
            let sub_projecao = projetar_tabela::<T>(sub_atual, sub_patch);
            T::insert_tabela(&mut saida, chave.to_owned(), T::de_tabela(sub_projecao));
        } else {
            T::insert_tabela(&mut saida, chave.to_owned(), valor_atual.clone());
        }
    }
    saida
}

fn tem_colisao_inicial<T: ValorDocumento>(atual: &T::Tabela, patch: &T::Tabela) -> bool {
    for (chave, valor_patch) in T::iter_tabela(patch) {
        let Some(valor_atual) = T::get_tabela(atual, chave) else {
            continue;
        };
        if let Some(sub_patch) = valor_patch.como_tabela() {
            match valor_atual.como_tabela() {
                Some(sub_atual) => {
                    if tem_colisao_inicial::<T>(sub_atual, sub_patch) {
                        return true;
                    }
                }
                None => return true,
            }
        } else if valor_atual != valor_patch {
            return true;
        }
    }
    false
}

fn normalizar_saida(texto: &str) -> String {
    let mut normalizado = texto.replace("\r\n", "\n");
    let aparado = normalizado.trim_end_matches('\n').len();
    normalizado.truncate(aparado);
    normalizado.push('\n');
    normalizado
}
