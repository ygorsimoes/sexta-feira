//! Adapters da Sexta-feira.
//!
//! Cada adapter traduz o modelo agnóstico (`sexta-feira-core`) para os arquivos
//! e convenções de um harness específico.

pub mod catalogo;
pub mod hermes;

pub use catalogo::{
    CaminhoDiagnosticado, DiagnosticoHarness, Harness, HarnessDesconhecido,
    OrigemResolucaoDestino, inspecionar_ambiente_harnesses,
};
pub use hermes::{AdaptadorHermes, ConfiguracaoHermes, ErroDeHermes};
