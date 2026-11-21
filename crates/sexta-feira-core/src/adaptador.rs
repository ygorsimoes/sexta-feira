//! Contrato agnóstico para tradutores de setup de cada harness.

use std::path::PathBuf;

use crate::definicao::DefinicaoDoMordomo;
use crate::plano::ArquivoDesejado;

/// Erro devolvido por um adaptador ao traduzir o setup do mordomo.
#[derive(Debug, thiserror::Error)]
pub enum ErroDeAdaptador {
    /// Falha ao montar ou validar o destino do adaptador.
    #[error("{0}")]
    Mensagem(String),
}

/// Contrato implementado por cada adapter para gerar arquivos declarativos a
/// partir da definição agnóstica do mordomo.
pub trait AdaptadorDeSetup {
    /// Identificador único do harness correspondente.
    fn id(&self) -> &'static str;

    /// Caminho do arquivo de estado gerenciado para este destino.
    fn arquivo_estado(&self) -> PathBuf;

    /// Gera a lista de arquivos esperados para o destino configurado.
    ///
    /// # Errors
    ///
    /// Retorna erro se não for possível construir ou validar os artefatos.
    fn planejar_arquivos(
        &self,
        definicao: &DefinicaoDoMordomo,
    ) -> Result<Vec<ArquivoDesejado>, ErroDeAdaptador>;
}
