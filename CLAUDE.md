# Vortex RPG — regras de engenharia

Este arquivo vale para toda tarefa neste repositório. Leia antes de escrever
código. O plano de evolução está em `docs/ROADMAP.md`.

## Arquitetura

Hexagonal: o núcleo é puro e os adaptadores ficam nas bordas. As dependências
apontam sempre para dentro.

```
client ──┐
         ├──> game-protocol ──> game-sim ──> engine-* 
server ──┘
```

| Camada | Onde | Pode | Não pode |
| --- | --- | --- | --- |
| Engine | `crates/engine/*` | lógica genérica e reutilizável em qualquer jogo | I/O, relógio, rede, serde obrigatório, tokio, axum, bevy, depender de `game-*` |
| Simulação | `crates/game/sim` | regras deste jogo, estado autoritativo | I/O, relógio, ambiente, tokio, axum, bevy, formato de fio |
| Protocolo | `crates/game/protocol` | DTOs, validação do que vem da rede, conversão DTO <-> domínio | transporte, render |
| Servidor | `crates/server` | adaptadores (WebSocket, loop de tick, config, disco) e composition root | regra de jogo |
| Cliente | `crates/client` | plugins Bevy finos: input, render, rede | regra de jogo autoritativa |

`./scripts/check-deps.sh` confere isso e roda no CI. Se ele falhar, o desenho
está errado: mova o código, não afrouxe o script.

## Onde colocar código novo

Pergunte nesta ordem e pare na primeira resposta "sim":

1. Serve para qualquer jogo e não precisa de I/O? → crate em `crates/engine/`.
2. É regra deste jogo (quem pode o quê, quando)? → `game-sim`.
3. É formato de mensagem ou validação de entrada da rede? → `game-protocol`.
4. Fala com o mundo externo (socket, disco, relógio, ambiente)? → `server`,
   atrás de um trait definido no núcleo quando o núcleo precisar chamá-lo.
5. É teclado, câmera, malha, HUD? → um plugin em `client`.

Um crate de engine novo faz **uma** coisa (atributos, inventário, missões), tem
`description` no `Cargo.toml`, `[lints] workspace = true`, e entra em
`[workspace.dependencies]`.

## Receita de uma mecânica nova (fatia vertical)

Toda mecânica atravessa as camadas de dentro para fora, nesta ordem, e só
está pronta quando dá para jogar:

1. **Engine** — tipos e lógica pura, com testes unitários.
2. **Simulação** — `Command` novo (o que o jogador pede), validação no
   servidor, efeito no `tick()`, `SimEvent`/snapshot com o resultado.
3. **Protocolo** — DTO novo e conversão; campos novos com `#[serde(default)]`.
4. **Servidor** — só encanamento, se precisar.
5. **Cliente** — plugin novo ou sistema novo num plugin existente.

O servidor é autoritativo: o cliente manda intenção ("quero atacar o alvo
que estou olhando"), nunca resultado ("o inimigo X perdeu 10 de vida"). O
servidor valida alcance, recarga, custo e estado antes de aplicar.

## Sistema na engine, conteúdo no jogo

O jogo é um RPG; os blocos são só o visual do mundo. Os crates de engine
trazem os **sistemas** (atributos, inventário, habilidades, missões) sem
conhecer nenhum item, inimigo ou habilidade específica. O **conteúdo** (quais
itens existem, quanto de vida tem cada inimigo, o que cada habilidade faz) é
dado, definido em `game-sim` em tabelas num módulo próprio, nunca espalhado em
`if` pelo código nem dentro da engine.

## Regras de código

- **Sem abstração especulativa.** Trait só com duas implementações reais ou
  quando é a costura que permite testar sem I/O. Sem genérico que ninguém usa.
- **Tipos de domínio não são DTOs.** Nada de `Serialize` em tipo do núcleo
  fora de feature opcional; a conversão fica em `game-protocol`.
- **Uma responsabilidade por módulo.** Arquivo passando de ~300 linhas ou
  módulo com dois motivos para mudar: divida.
- **Sem duplicação de regra.** Constante ou cálculo usado em dois crates mora
  no crate mais interno e é importado.
- **Sem números mágicos** de gameplay espalhados: constante nomeada, com
  unidade no comentário.
- **Erros:** `thiserror` nas bibliotecas; `Box<dyn Error>`/`anyhow` só nos
  binários. Sem `unwrap()`/`expect()` em caminho que dado de rede alcança.
- **Sem `unsafe`** (proibido por lint). Sem `todo!()`/`dbg!()` commitado.
- **Determinismo no núcleo:** nada de relógio ou aleatoriedade global; seed e
  `dt` entram por parâmetro.
- **Documentação:** todo item público com doc comment de uma linha. Comentário
  explica o porquê, não o quê. Tudo em português, como o resto do repositório.
- **Não mude comportamento fora do escopo da tarefa.** Problema encontrado no
  caminho vai para "Pendências" no `docs/ROADMAP.md`.

## Testes

- Engine e simulação: teste unitário para cada regra e cada caso de borda
  (limite de alcance, inventário cheio, vida zero, recarga ainda ativa).
- Mecânica nova na simulação: um teste de integração em `crates/game/sim/tests`
  que aplica comandos e confere snapshot/eventos, sem rede.
- Protocolo: todo DTO novo ganha caso em `crates/game/protocol/tests/golden.rs`.
  JSON de mensagem existente só muda de propósito, e o commit diz isso.
- Os goldens de simulação (`crates/game/sim/tests/golden/`) só são regerados
  quando a tarefa muda o movimento de propósito; explique no commit.
- Nunca apague, pule ou enfraqueça um teste para passar.

## Pronto significa

Antes de dizer que terminou, rode e mostre a saída:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/check-deps.sh
```

E entregue no relatório final:

- o que mudou, por crate;
- **Como testar jogando:** passos exatos (teclas, o que deve aparecer na
  tela, o que testar com dois clientes abertos);
- o que foi deixado de fora e por quê.

Depois marque a tarefa como feita em `docs/ROADMAP.md` e atualize o README se
controles, variáveis de ambiente ou a estrutura de crates mudaram.

## Fluxo de trabalho

- Uma tarefa do roadmap por vez, em uma branch `feat/<tarefa>` a partir de
  `main`. Não misture duas tarefas nem refatoração solta no mesmo PR.
- Commits pequenos no padrão do histórico (`feat:`, `fix:`, `refactor:`,
  `test:`, `chore:`), em português, cada um compilando e com testes passando.
- Se a tarefa exigir uma decisão de desenho que o roadmap não cobre, escolha a
  opção mais simples que respeita este arquivo e registre a decisão no
  relatório.
