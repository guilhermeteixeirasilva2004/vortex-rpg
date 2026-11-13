# Roadmap

Plano guia das mecânicas básicas. Cada tarefa é uma fatia vertical pequena:
termina com algo que dá para abrir o jogo e testar. Faça uma por vez, na
ordem; as regras de engenharia estão em `CLAUDE.md`.

Para pedir uma tarefa ao Claude Code:

```text
Execute a tarefa T1 do docs/ROADMAP.md seguindo o CLAUDE.md.
```

## Estado

- [x] T0 — Base: mundo voxel, movimento autoritativo, arquitetura modular
- [ ] T1 — Ajustes de movimento
- [ ] T2 — Mira: raycast e bloco em destaque
- [ ] T3 — Quebrar e colocar blocos
- [ ] T4 — Inventário e hotbar
- [ ] T5 — Vida, dano de queda e morte
- [ ] T6 — Entidades e um NPC que anda
- [ ] T7 — Combate corpo a corpo
- [ ] T8 — Salvar o mundo em disco
- [ ] T9 — Predição de movimento no cliente
- [ ] T10 — Ciclo de dia e noite

Ideias para depois, sem ordem: crafting, chat, experiência e níveis, ruído de
verdade no terreno, carregamento de chunks por distância, protocolo binário.

## Tarefas

### T1 — Ajustes de movimento

Corrige as três pendências conhecidas do movimento.

- Toque curto no Espaço (menos de um tick) não pode se perder: o cliente
  guarda o toque até o próximo input enviado.
- O pulo alcança ~1,05 bloco, mas o comentário promete ~1,25. Ajuste para
  ~1,25 medido a 20 Hz, com teste que fixa a altura.
- Sair do voo abaixo de y=0 derruba o jogador no vazio. Dentro da área gerada,
  ele deve voltar para a superfície.
- `Ctrl` esquerdo também desce no voo, além do `Shift`.

Muda o movimento de propósito: regere os goldens de simulação.

**Testar jogando:** toques rápidos no Espaço sempre pulam; dá para subir um
degrau de 1 bloco com folga e não de 2; F3, descer até atravessar o mundo,
F3 de novo e o jogador reaparece em cima do terreno.

### T2 — Mira: raycast e bloco em destaque

- `engine-voxel`: raycast em voxels (DDA) que devolve o primeiro bloco sólido
  no caminho, a face atingida e a distância. Testes com mundo sintético.
- `client`: plugin que lança o raio da câmera a cada frame e desenha um
  contorno no bloco mirado, até um alcance máximo (constante compartilhada no
  núcleo, porque o servidor vai usar a mesma em T3). Mira no centro da tela.

Só visual: nada muda no servidor nem no protocolo.

**Testar jogando:** o contorno segue o bloco para onde você olha, some ao
olhar para o céu ou para longe demais, e marca o bloco certo em quinas.

### T3 — Quebrar e colocar blocos

- `game-sim`: comandos "quebrar o bloco mirado" e "colocar bloco na face
  mirada". O servidor refaz o raycast a partir do olho e do yaw/pitch do
  jogador e valida o alcance; não confia em coordenada vinda do cliente sem
  conferir. Não deixa colocar bloco dentro de um jogador.
- O mundo passa a ser mutável. Hoje os chunks são um `Arc<ChunkMap>` imutável
  compartilhado com o handshake da rede; resolva isso com uma única fonte de
  verdade na simulação.
- `game-protocol`: mensagens do cliente para as duas ações e mensagem do
  servidor com a mudança de bloco. Quem entra depois recebe o mundo já
  modificado.
- `client`: clique esquerdo quebra, clique direito coloca; o chunk (e o
  vizinho, na borda) é remalhado.

**Testar jogando:** quebrar e colocar blocos; cavar um buraco e cair nele;
não conseguir agir além do alcance; com dois clientes, um vê as mudanças do
outro; um cliente que entra depois vê tudo como ficou.

### T4 — Inventário e hotbar

- Novo `engine-inventory`: item, pilha com limite, inventário de slots
  (adicionar, remover, mover, empilhar). Puro e testado.
- `game-sim`: bloco quebrado entra no inventário do jogador; colocar bloco
  consome do slot selecionado e falha se estiver vazio.
- `game-protocol`: estado do inventário do próprio jogador e seleção de slot.
- `client`: hotbar no HUD com contagem; teclas `1`–`9` e roda do mouse
  selecionam.

**Testar jogando:** começar sem blocos e não conseguir colocar; quebrar grama
e ver a contagem subir; colocar e ver descer; trocar de slot muda o bloco
colocado.

### T5 — Vida, dano de queda e morte

- Novo `engine-vitals`: vida com máximo, dano, cura, morte. Puro e testado.
- `game-sim`: dano de queda proporcional à velocidade de impacto acima de um
  limite; cair no vazio mata; ao morrer o jogador renasce no spawn com vida
  cheia. Voo debug não toma dano.
- `game-protocol`: vida do próprio jogador no snapshot; evento de morte.
- `client`: barra de vida no HUD e um aviso rápido ao morrer.

**Testar jogando:** pular de 3 blocos não machuca; cair de bem alto tira
vida; cair de mais alto ainda mata e renasce no spawn; F3 não toma dano.

### T6 — Entidades e um NPC que anda

- `game-sim`: generalize "jogador" para "entidade" (id, tipo, corpo físico),
  mantendo o jogador como um tipo. NPCs usam o mesmo `Body` da física.
- Novo `engine-ai`: máquina de estados mínima (parado, andando até um ponto),
  determinística com seed recebida por parâmetro.
- Um NPC nasce perto do spawn e perambula; pula degraus de 1 bloco.
- `game-protocol`/`client`: entidades no snapshot, desenhadas com cor
  diferente dos jogadores.

**Testar jogando:** ver o NPC andando pelo terreno, subindo degraus e sem
atravessar blocos; cavar um buraco (T3) e ver ele cair dentro.

### T7 — Combate corpo a corpo

- `engine-physics`: raio contra AABB de entidade.
- `game-sim`: comando de ataque com alcance e tempo de recarga validados no
  servidor; dano (T5), empurrão e morte do NPC, que deixa um item no
  inventário de quem matou (T4) e renasce depois de um tempo.
- `client`: clique esquerdo ataca quando a mira está numa entidade (senão,
  quebra bloco); feedback visual do acerto.

**Testar jogando:** bater no NPC até morrer, ver o empurrão, receber o item,
ver ele renascer; clicar rápido não bate mais rápido que a recarga; dois
jogadores conseguem se atacar (ou não: registre a decisão).

### T8 — Salvar o mundo em disco

- `game-sim`: trait de armazenamento do mundo (porta) com uma implementação
  em memória para testes.
- `server`: adaptador que grava em disco só os chunks modificados, ao sair e
  a cada intervalo; carrega ao iniciar. Pasta configurável por variável de
  ambiente, lida em `config.rs`.

**Testar jogando:** construir algo, fechar o servidor, abrir de novo e
encontrar tudo no lugar; apagar a pasta de save volta ao mundo gerado.

### T9 — Predição de movimento no cliente

- `client`: roda o mesmo `engine-physics` localmente para o próprio jogador e
  reconcilia com o servidor usando `last_input_seq`, reaplicando os inputs
  ainda não confirmados. O servidor continua sendo a autoridade.

**Testar jogando:** o movimento responde na hora, sem o atraso de hoje; com
atraso de rede artificial o jogador não "volta" visivelmente ao andar; bater
numa parede não treme.

### T10 — Ciclo de dia e noite

- Novo `engine-time`: relógio de mundo avançado por tick, com duração do dia
  configurável. Puro e testado.
- `game-sim`/`game-protocol`: hora do mundo no snapshot.
- `client`: direção e intensidade do sol e cor do céu seguem a hora.

**Testar jogando:** o céu escurece e clareia num ciclo curto de teste; dois
clientes veem a mesma hora.

## Pendências

Problemas encontrados fora do escopo de uma tarefa entram aqui, com uma linha
cada, até virarem tarefa.

- O cliente depende de `game-sim` indiretamente, via `game-protocol`.
