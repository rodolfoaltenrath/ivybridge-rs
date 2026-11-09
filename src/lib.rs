//! IvyBridge-RS — fase 1 (Probe v0.1).
//!
//! Utilitário de diagnóstico: lê o EDID do monitor, calcula timings
//! (CVT / CVT-RB / CVT-RBv2) e confronta cada modo com os tetos de pixel
//! clock conhecidos da saída. Nada aqui escreve em registradores da GPU
//! nem altera o modo de vídeo ativo.

pub mod display;
pub mod edid;
pub mod limits;
pub mod report;
pub mod timings;
