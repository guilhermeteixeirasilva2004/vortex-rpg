use engine_physics::MoveIntent;

use crate::PlayerId;

/// Input mais recente de um jogador, já validado pela borda.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InputFrame {
    /// Número de sequência; inputs com `seq` menor que o atual são ignorados.
    pub seq: u32,
    /// O que o jogador quer fazer (o voo ainda passa pela permissão).
    pub intent: MoveIntent,
    /// Radianos, olhar pra cima/baixo (só repassado no snapshot).
    pub pitch: f32,
}

/// O que pode acontecer com a simulação entre ticks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    /// Jogador entrou: nasce no spawn.
    Join {
        /// Quem entrou.
        id: PlayerId,
    },
    /// Jogador saiu.
    Leave {
        /// Quem saiu.
        id: PlayerId,
    },
    /// Novo input "segurado" do jogador.
    Input {
        /// De quem é o input.
        id: PlayerId,
        /// O input.
        input: InputFrame,
    },
}

/// Algo notável que aconteceu durante um tick (para log/telemetria).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimEvent {
    /// O jogador caiu abaixo do mundo e voltou ao spawn.
    FellIntoVoid(PlayerId),
}
