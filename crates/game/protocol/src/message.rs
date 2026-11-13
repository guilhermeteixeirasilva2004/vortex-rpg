use serde::{Deserialize, Serialize};

use game_sim::{CreatureId, PlayerId};

/// Estado de input "segurado" no momento. O cliente não manda posição:
/// o servidor aplica o input e decide onde o jogador está.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct PlayerInput {
    /// Número de sequência, crescente.
    pub seq: u32,
    /// -1..1 (S..W)
    pub forward: f32,
    /// -1..1 (A..D)
    pub right: f32,
    /// -1..1 (Shift..Space). Andando, `up > 0.5` é pulo; voando, sobe/desce.
    pub up: f32,
    /// Pedido de voo livre (debug, F3). O servidor decide se permite.
    #[serde(default)]
    pub fly: bool,
    /// Radianos, rotação em torno de Y.
    pub yaw: f32,
    /// Radianos, olhar pra cima/baixo.
    pub pitch: f32,
}

/// Mensagens do cliente para o servidor.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Input atual do jogador.
    Input(PlayerInput),
}

/// Estado de um jogador como o cliente recebe.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PlayerState {
    /// Jogador.
    pub id: PlayerId,
    /// Posição do olho.
    pub position: [f32; 3],
    /// Radianos, em torno de Y.
    pub yaw: f32,
    /// Radianos, olhar pra cima/baixo.
    pub pitch: f32,
    /// Voo livre (debug) ativo para este jogador.
    #[serde(default)]
    pub flying: bool,
    /// Último `seq` de input processado (base para predição/reconciliação futura).
    pub last_input_seq: u32,
}

/// Estado de uma criatura como o cliente recebe.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CreatureState {
    /// Criatura (espaço de ids separado do dos jogadores).
    pub id: CreatureId,
    /// Nome do tipo (ex.: "coelho"); o cliente escolhe a aparência por ele.
    pub kind: String,
    /// Posição do pé (centro da base do corpo).
    pub feet: [f32; 3],
    /// Radianos, em torno de Y.
    pub yaw: f32,
    /// Largura do corpo em X e Z (blocos).
    pub width: f32,
    /// Altura do corpo (blocos).
    pub height: f32,
}

/// Posição de chunk no fio.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ChunkPosDto {
    /// Eixo X, em chunks.
    pub x: i32,
    /// Eixo Y, em chunks.
    pub y: i32,
    /// Eixo Z, em chunks.
    pub z: i32,
}

/// Chunk no fio: posição + bytes crus (índice = x + z*16 + y*256).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkDto {
    /// Posição do chunk.
    pub pos: ChunkPosDto,
    /// Um byte por bloco; validado na conversão para `Chunk`.
    pub blocks: Vec<u8>,
}

/// Mensagens do servidor para o cliente.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Primeira mensagem da conexão.
    Welcome {
        /// Id atribuído ao cliente.
        player_id: PlayerId,
        /// Onde o jogador nasce.
        spawn: [f32; 3],
    },
    /// Um chunk do mundo.
    ChunkData(ChunkDto),
    /// Estado das entidades após um tick.
    Snapshot {
        /// Número do tick.
        tick: u64,
        /// Jogadores.
        players: Vec<PlayerState>,
        /// Criaturas. Omitido quando vazio, para o JSON sem criaturas ficar
        /// igual ao de antes delas.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        creatures: Vec<CreatureState>,
    },
}
