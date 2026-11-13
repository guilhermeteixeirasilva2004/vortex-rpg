# Vortex RPG

RPG voxel multiplayer em Rust, com servidor autoritativo e cliente Bevy.

O mundo é dividido em chunks 16x16x16, gerados proceduralmente a partir de um
heightmap determinístico. O cliente só envia input; quem simula o movimento e
decide a posição dos jogadores e das criaturas é o servidor (20 Hz), que
transmite snapshots via WebSocket.

## Estrutura do workspace

```
crates/
├── engine/            # núcleo reutilizável: sem I/O, sem rede, sem framework
│   ├── voxel/         # Block, Chunk, ChunkMap, VoxelQuery, WorldGenerator
│   ├── physics/       # Body + BodySpec + MoveIntent: gravidade, pulo, colisão AABB, voo
│   ├── meshing/       # geometria de chunk (posições, normais, cores, índices)
│   └── ai/            # Wander (parado / andando até um ponto) e Rng com seed
├── game/
│   ├── sim/           # simulação autoritativa pura: entidades, comandos, tick, snapshot
│   └── protocol/      # DTOs do fio, conversões DTO <-> domínio, codec JSON
├── server/            # adaptadores: WebSocket (axum), loop de tick (tokio), config
└── client/            # plugins Bevy: rede, mundo, câmera, input, jogadores, criaturas, HUD
```

- **engine-voxel** — chunks 16³ e `ChunkMap` (a única busca de bloco por
  coordenada de mundo). `WorldGenerator` é injetado na simulação;
  `HeightmapGenerator` é o terreno atual.
- **engine-physics** — `Body::step` recebe um `MoveIntent` (andar, `jump`,
  `fly`, `fly_vertical`), nunca um tipo de rede. Tamanho e velocidade de cada
  corpo vêm de um `BodySpec` (`BodySpec::PLAYER` para o jogador).
- **engine-ai** — `Wander` decide para onde uma entidade anda (fica parada,
  escolhe um ponto no raio da origem, anda até ele ou desiste). Determinístico:
  a aleatoriedade sai de um `Rng` com seed recebida por parâmetro.
- **engine-meshing** — culling de faces ocultas; o cliente só converte para
  `bevy::Mesh`.
- **game-sim** — `Simulation::apply(Command)`, `tick()` e `snapshot()`. Jogadores
  e criaturas são entidades com o mesmo `Body`. Não lê relógio, ambiente nem
  rede; regras (tamanho do mundo, voo permitido) e a seed vêm de `SimConfig`.
  Conteúdo (tipos de criatura e onde nascem) fica em `content.rs`.
- **game-protocol** — formato no fio (JSON) e a borda de validação: o input do
  cliente é sanitizado aqui e o eixo `up` vira `jump`/`fly_vertical`.
- **server** — `config.rs` (único lugar que lê o ambiente), `tick.rs` (loop a
  20 Hz e broadcast), `net.rs` (WebSocket) e `main.rs` (composition root).
- **client** — um plugin por responsabilidade. Sem áudio, gamepad ou glTF para
  manter leve em GPU integrada.

### Regra de dependência

As dependências apontam sempre para dentro:

```
client ──┐
         ├──> game-protocol ──> game-sim ──> engine-physics ──> engine-voxel
server ──┘                              └──> engine-ai
                                             engine-meshing ──> engine-voxel
```

- `engine-*` não depende de `game-*`, `server`, `client`, tokio, axum nem bevy.
- `game-sim` não depende de tokio, axum, bevy nem serde_json.
- `server` e `client` dependem do núcleo, nunca o contrário.

Para conferir: `cargo tree -p game-sim -e normal` (e o mesmo para cada
`engine-*`).

## Requisitos

- Rust 1.100+ (edição 2021)
- Linux: dependências de sistema do Bevy/Winit (`libx11-dev`, `libxkbcommon-dev`,
  etc.). Para Wayland nativo, habilite a feature `wayland` do crate `client`
  (requer `libwayland-dev`); sem ela o cliente roda via XWayland.

## Como rodar

Em um terminal, suba o servidor:

```bash
cargo run -p server
```

Em outro, suba o cliente:

```bash
cargo run -p client
```

Por padrão o servidor escuta em `0.0.0.0:3000` e o cliente conecta em
`ws://127.0.0.1:3000/ws`. As variáveis de ambiente abaixo sobrescrevem isso:

| Variável         | Onde    | Padrão                     |
| ---------------- | ------- | -------------------------- |
| `VORTEX_BIND`    | server  | `0.0.0.0:3000`             |
| `VORTEX_SERVER`  | client  | `ws://127.0.0.1:3000/ws`   |
| `RUST_LOG`       | server  | `info`                     |
| `VORTEX_ALLOW_FLY` | server | `1` em debug, `0` em release |

Com Wayland nativo:

```bash
cargo run -p client --features wayland
```

## Controles

| Tecla / botão      | Ação                   |
| ------------------ | ---------------------- |
| `W` / `S`          | frente / trás         |
| `A` / `D`          | esquerda / direita    |
| `Espaço`           | pular                 |
| `F3`               | modo voo (debug)      |
| `Espaço` / `Shift` | subir / descer (voo)  |
| Mouse              | olhar (yaw/pitch)     |
| Clique esquerdo    | capturar o cursor     |
| `Esc`              | liberar o cursor      |

## Build e qualidade

```bash
cargo check --workspace
cargo build --release -p server
cargo build --release -p client
```

O CI (`.github/workflows/ci.yml`) roda os mesmos comandos que valem localmente:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Os lints ficam em `[workspace.lints]` no `Cargo.toml` raiz. Os testes "golden"
(`crates/game/protocol/tests/golden.rs` e `crates/game/sim/tests/golden.rs`)
fixam o JSON no fio e o resultado da simulação bit a bit: se mudarem, mudou
comportamento.

O perfil de desenvolvimento usa `opt-level = 1` no workspace e `opt-level = 3`
nas dependências, porque o Bevy é lento em debug sem otimização. O perfil
release usa `lto = "thin"` e `codegen-units = 1`.

## Estado atual

Projeto em estágio inicial. O protocolo trafega JSON para facilitar a depuração;
a ideia é migrar para um formato binário (bincode/postcard) quando o volume de
chunks começar a pesar.

## Licença

MIT OR Apache-2.0
