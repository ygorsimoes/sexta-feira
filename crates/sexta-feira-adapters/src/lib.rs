//! Adapters da Sexta-feira.
//!
//! Cada adapter traduz o modelo agnóstico (`sexta-feira-core`) para os arquivos
//! e convenções de um harness específico.

pub mod catalogo;

pub use catalogo::{Harness, HarnessDesconhecido};
