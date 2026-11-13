# Vortex RPG

RPG voxel multiplayer em Rust, com servidor autoritativo e cliente Bevy.

O mundo é dividido em chunks 16x16x16, gerados proceduralmente a partir de um
heightmap determinístico. O cliente só envia input; quem simula o movimento e
decide a posição dos jogadores é o servidor (20 Hz), que transmite snapshots via
WebSocket.

## Estrutura do workspace

```
crates/
├── shared/  # tipos comuns: voxels, protocolo de rede e simulação de movimento
├── server/  # servidor autoritativo (Axum + Tokio, WebSocket)
└── client/  # cliente gráfico (Bevy + Tokio Tungstenite)
```

- **shared** — `voxel.rs` (chunks e geração de terreno), `protocol.rs`
  (mensagens cliente/servidor em JSON) e `movement.rs` (simulação determinística
  compartilhada).
- **server** — gera o mundo inicial (8x2x8 chunks em torno da origem), recebe
  input, simula a 20 Hz e faz broadcast de snapshots.
- **client** — renderiza o mundo, faz a malha dos chunks (meshing) e envia o
  input a 20 Hz. Sem áudio, gamepad ou glTF para manter leve em GPU integrada.

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

Com Wayland nativo:

```bash
cargo run -p client --features wayland
```

## Controles

| Tecla / botão      | Ação                   |
| ------------------ | ---------------------- |
| `W` / `S`          | frente / trás         |
| `A` / `D`          | esquerda / direita    |
| `Espaço` / `Shift` | subir / descer        |
| Mouse              | olhar (yaw/pitch)     |
| Clique esquerdo    | capturar o cursor     |
| `Esc`              | liberar o cursor      |

## Build

```bash
cargo check --workspace
cargo build --release -p server
cargo build --release -p client
```

O perfil de desenvolvimento usa `opt-level = 1` no workspace e `opt-level = 3`
nas dependências, porque o Bevy é lento em debug sem otimização. O perfil
release usa `lto = "thin"` e `codegen-units = 1`.

## Estado atual

Projeto em estágio inicial. O protocolo trafega JSON para facilitar a depuração;
a ideia é migrar para um formato binário (bincode/postcard) quando o volume de
chunks começar a pesar.

## Licença

MIT OR Apache-2.0
