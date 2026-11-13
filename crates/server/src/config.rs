//! Configuração do servidor. Único lugar que lê variáveis de ambiente.

/// Endereço padrão de escuta.
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3000";

/// Configuração lida do ambiente na inicialização.
#[derive(Debug, Clone)]
pub struct Config {
    /// `VORTEX_BIND`: endereço de escuta.
    pub bind_addr: String,
    /// `VORTEX_ALLOW_FLY`: aceita o voo livre (debug) pedido pelos clientes.
    pub allow_fly: bool,
    /// `RUST_LOG`: filtro de log; `None` (ou inválido) cai para `info`.
    pub log_filter: Option<String>,
}

impl Config {
    /// Lê a configuração do ambiente do processo.
    pub fn from_env() -> Self {
        let var = |name: &str| std::env::var(name).ok();
        Self::from_vars(var("VORTEX_BIND"), var("VORTEX_ALLOW_FLY"), var("RUST_LOG"))
    }

    fn from_vars(bind: Option<String>, allow_fly: Option<String>, log: Option<String>) -> Self {
        Self {
            bind_addr: bind.unwrap_or_else(|| DEFAULT_BIND_ADDR.into()),
            // Voo livre (F3) é ferramenta de debug: liberado em build de debug,
            // ou explicitamente com VORTEX_ALLOW_FLY=1 / desligado com =0.
            allow_fly: match allow_fly.as_deref() {
                Some("1" | "true") => true,
                Some("0" | "false") => false,
                _ => cfg!(debug_assertions),
            },
            log_filter: log,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let c = Config::from_vars(None, None, None);
        assert_eq!(c.bind_addr, DEFAULT_BIND_ADDR);
        assert_eq!(c.allow_fly, cfg!(debug_assertions));
        assert_eq!(c.log_filter, None);
    }

    #[test]
    fn allow_fly_values() {
        let fly = |v: &str| Config::from_vars(None, Some(v.into()), None).allow_fly;
        assert!(fly("1") && fly("true"));
        assert!(!fly("0") && !fly("false"));
        assert_eq!(fly("talvez"), cfg!(debug_assertions));
    }
}
