//! Tipos compartilhados entre cliente e servidor: protocolo de rede e
//! constantes de conexão/tick.

pub mod protocol;

/// Endereço padrão do servidor (pode ser sobrescrito com `VORTEX_SERVER`).
pub const DEFAULT_SERVER_URL: &str = "ws://127.0.0.1:3000/ws";
pub const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3000";

/// Ticks de simulação por segundo no servidor. O cliente envia input na mesma taxa.
pub const SERVER_TICK_RATE: u32 = 20;
pub const TICK_DT: f32 = 1.0 / SERVER_TICK_RATE as f32;
