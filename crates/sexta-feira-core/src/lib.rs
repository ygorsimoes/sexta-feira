//! Núcleo agnóstico da Sexta-feira.
//!
//! Esta crate não conhece nenhum harness. Tudo o que é específico de um
//! harness fica em `sexta-feira-adapters`.

pub mod definicao;

pub use definicao::{DefinicaoDoMordomo, ErroDeDefinicao, PerfilDoUsuario, Persona};
