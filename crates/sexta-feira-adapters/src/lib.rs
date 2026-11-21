//! Adapters da Sexta-feira.
//!
//! Cada adapter traduz o modelo agnóstico (`sexta-feira-core`) para os arquivos
//! e convenções de um harness específico.

pub mod catalogo;
pub mod hermes;

pub use catalogo::{Harness, HarnessDesconhecido};
pub use hermes::{AdaptadorHermes, ConfiguracaoHermes, ErroDeHermes};
