//! Motor da Sexta-feira.
//!
//! Esta crate não conhece nenhum harness. Tudo o que é específico de um
//! harness fica em `sexta-feira-adapters`.

pub mod harness;

pub use harness::{Harness, HarnessDesconhecido};
