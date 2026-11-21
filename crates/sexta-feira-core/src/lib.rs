//! Núcleo agnóstico da Sexta-feira.
//!
//! Esta crate não conhece nenhum harness. Tudo o que é específico de um
//! harness fica em `sexta-feira-adapters`.

pub mod adaptador;
pub mod definicao;
mod estruturado;
pub mod plano;

pub use adaptador::{AdaptadorDeSetup, ErroDeAdaptador};
pub use definicao::{
    DefinicaoDoMordomo, ErroDeDefinicao, FontesDaDefinicao, PerfilDoUsuario, Persona,
};
pub use plano::{
    AcaoPlanejada, ArquivoDesejado, ErroDePlano, EstrategiaDeEscrita, FormatoEstruturado,
    ItemDeVerificacao, OperacaoPlanejada, PermissaoDeArquivo, PlanoDeSetup, ResultadoDaAplicacao,
    ResultadoDaVerificacao, aplicar, planejar, verificar,
};
