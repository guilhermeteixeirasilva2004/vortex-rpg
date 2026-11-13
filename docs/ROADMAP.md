# Roadmap

Plano guia das mecânicas básicas de RPG. O mundo é feito de blocos, mas o
jogo não é de construção: o foco é personagem, combate, progressão, itens e
missões. O terreno é cenário fixo.

Cada tarefa é uma fatia vertical pequena: termina com algo que dá para abrir
o jogo e testar. Faça uma por vez, na ordem; as regras de engenharia estão em
`CLAUDE.md`.

Para pedir uma tarefa ao Claude Code:

```text
Dê git pull no main e execute a tarefa T1 do docs/ROADMAP.md seguindo o CLAUDE.md.
```

## Estado

- [x] T0 — Base: mundo voxel, movimento autoritativo, arquitetura modular
- [ ] T1 — Ajustes de movimento
- [ ] T2 — Mira: raycast e bloco em destaque
- [ ] T3 — Entidades e uma criatura que anda
- [ ] T4 — Atributos, vida e morte
- [ ] T5 — Alvo e combate corpo a corpo
- [ ] T6 — Inimigo que persegue e ataca
- [ ] T7 — Experiência e níveis
- [ ] T8 — Inventário e loot
- [ ] T9 — Equipamento
- [ ] T10 — Habilidades, mana e recarga
- [ ] T11 — NPCs e diálogo
- [ ] T12 — Missões
- [ ] T13 — Salvar personagem e progresso
- [ ] T14 — Predição de movimento no cliente

Ideias para depois, sem ordem: classes, efeitos de status (veneno, lentidão),
ouro e lojas, grupo de jogadores, chat, zonas e masmorras, tipos variados de
inimigo e chefes, ciclo de dia e noite, ruído de verdade no terreno, protocolo
binário.

## Tarefas

### T1 — Ajustes de movimento

Corrige as pendências conhecidas do movimento.

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
  núcleo, porque o servidor vai usar a mesma em T5). Mira no centro da tela.

Só visual: nada muda no servidor nem no protocolo.

**Testar jogando:** o contorno segue o bloco para onde você olha, some ao
olhar para o céu ou para longe demais, e marca o bloco certo em quinas.

### T3 — Entidades e uma criatura que anda

- `game-sim`: generalize "jogador" para "entidade" (id, tipo, corpo físico),
  mantendo o jogador como um tipo. Criaturas usam o mesmo `Body` da física.
- Novo `engine-ai`: máquina de estados mínima (parado, andando até um ponto),
  determinística, com seed recebida por parâmetro.
- Algumas criaturas nascem em pontos fixos perto do spawn e perambulam em
  volta do ponto de origem; pulam degraus de 1 bloco.
- Tipos de criatura são conteúdo: tabela em `game-sim` (nome, tamanho,
  velocidade), não código na engine.
- `game-protocol`/`client`: entidades no snapshot, desenhadas com cor e
  tamanho próprios, diferentes dos jogadores.

**Testar jogando:** ver as criaturas andando pelo terreno sem atravessar
blocos e sem se afastar demais da origem; com dois clientes, os dois veem as
criaturas nos mesmos lugares.

### T4 — Atributos, vida e morte

- Novo `engine-stats`: atributos base (ex.: força, agilidade, vitalidade,
  inteligência), valores derivados calculados a partir deles (vida máxima,
  dano, etc.) e recurso com atual/máximo (vida), com dano, cura e morte. A
  engine dá o mecanismo; as fórmulas e os valores iniciais são conteúdo do
  jogo.
- `game-sim`: toda entidade tem atributos e vida. Dano de queda acima de uma
  velocidade de impacto; cair no vazio mata. Ao morrer o jogador renasce no
  spawn com vida cheia. Voo debug não toma dano.
- `game-protocol`: vida de cada entidade no snapshot; evento de morte.
- `client`: barra de vida do jogador no HUD e aviso rápido ao morrer.

**Testar jogando:** pular de 3 blocos não machuca; cair de bem alto tira
vida; de mais alto ainda mata e renasce no spawn; F3 não toma dano.

### T5 — Alvo e combate corpo a corpo

- `engine-physics`: raio contra AABB de entidade. O raycast em voxels de T2
  é reaproveitado para o terreno bloquear a linha de visão.
- `game-sim`: comando de ataque. O servidor refaz o raio a partir do olho e do
  yaw/pitch do jogador, valida alcance, linha de visão e recarga, e calcula o
  dano a partir dos atributos (T4). Empurrão no alvo. Criatura morta some e
  renasce na origem depois de um tempo.
- `game-protocol`: ataque do cliente; evento de dano com valor.
- `client`: o destaque da mira (T2) passa a marcar a entidade mirada; o
  contorno de bloco fica só no modo debug (F3). A entidade mirada mostra
  nome e barra de vida; clique esquerdo ataca; número de dano flutuando.

**Testar jogando:** mirar numa criatura e ver nome e vida; bater até morrer e
ver renascer; clicar rápido não bate mais rápido que a recarga; não acertar
de longe nem através de um morro; dois jogadores batendo na mesma criatura.

### T6 — Inimigo que persegue e ataca

- `engine-ai`: estados de perseguir, atacar e voltar para a origem, com raio
  de percepção e raio de desistência.
- `game-sim`: tipo de criatura hostil na tabela de conteúdo (dano, alcance,
  recarga, percepção). Ela persegue o jogador mais próximo, ataca com as
  mesmas regras de dano de T5 e volta para casa recuperando vida ao desistir.
  Criaturas pacíficas de T3 continuam existindo.

**Testar jogando:** chegar perto de um inimigo e ele vir atrás; tomar dano e
morrer se ficar parado; fugir e ver ele desistir e voltar; com dois
jogadores, ele escolhe o mais próximo.

### T7 — Experiência e níveis

- Novo `engine-progression`: experiência, curva de nível e pontos de atributo
  ganhos por nível. Curva e recompensas são conteúdo.
- `game-sim`: matar criatura dá experiência a quem causou dano; subir de nível
  dá pontos; comando para gastar ponto em um atributo, validado no servidor.
  Os valores derivados (vida máxima, dano) são recalculados.
- `game-protocol`: nível, experiência e pontos do próprio jogador.
- `client`: barra de experiência e nível no HUD; tela de personagem (tecla
  `C`) com atributos e botão para gastar pontos; aviso ao subir de nível.

**Testar jogando:** matar inimigos e ver a barra encher; subir de nível;
gastar um ponto em força e bater mais forte; gastar em vitalidade e ver a
vida máxima subir.

### T8 — Inventário e loot

- Novo `engine-inventory`: item, pilha com limite, inventário de slots
  (adicionar, remover, mover, empilhar). Puro e testado.
- `game-sim`: tabela de itens e tabela de loot por tipo de criatura, sorteada
  com seed determinística. Criatura morta deixa o loot no chão como entidade;
  o jogador pega chegando perto e apertando `E`. Um consumível (poção) cura
  ao ser usado.
- `game-protocol`: inventário do próprio jogador; comandos de pegar, usar e
  mover item.
- `client`: itens no chão visíveis; tela de inventário (tecla `I`); usar item
  com clique.

**Testar jogando:** matar um inimigo, ver o item cair, pegar com `E`, abrir o
inventário e ver a pilha; usar uma poção e a vida subir; inventário cheio
recusa o item, que continua no chão; com dois jogadores, só um pega.

### T9 — Equipamento

- `engine-inventory` ou crate próprio: slots de equipamento (arma, armadura)
  com regra de qual item entra em qual slot.
- `engine-stats`: modificadores de atributo vindos de fora (equipamento
  agora, efeitos depois), somados aos valores base.
- `game-sim`: equipar e desequipar validados no servidor; arma muda dano,
  alcance e recarga do ataque; armadura reduz dano recebido.
- `client`: slots de equipamento na tela de inventário; valores atualizados
  na tela de personagem.

**Testar jogando:** equipar uma espada caída de um inimigo e matar mais
rápido; equipar armadura e tomar menos dano; desequipar volta ao normal.

### T10 — Habilidades, mana e recarga

- Novo `engine-abilities`: habilidade com custo, recarga, alcance e lista de
  efeitos (dano, cura, empurrão); controle de recarga por entidade. As
  habilidades em si são conteúdo.
- `engine-stats`: segundo recurso (mana) com regeneração por tick.
- `game-sim`: duas ou três habilidades na tabela de conteúdo (ex.: golpe
  forte, cura, projétil à distância); comando de usar habilidade validado no
  servidor (custo, recarga, alcance, alvo).
- `client`: barra de mana; barra de habilidades com teclas `1`–`4`, mostrando
  a recarga; efeito visual simples ao usar.

**Testar jogando:** usar cada habilidade e ver o efeito; ficar sem mana e não
conseguir usar; ver a recarga contando; a mana voltar com o tempo.

### T11 — NPCs e diálogo

- Novo `engine-dialogue`: árvore de diálogo (falas, opções, saltos), pura e
  definida por dados.
- `game-sim`: tipo de entidade NPC amigável, que não pode ser atacado; comando
  de interagir (`E`) validado por distância; estado da conversa por jogador.
- `client`: indicação ao mirar num NPC; caixa de diálogo com opções
  escolhidas pelo teclado.

**Testar jogando:** falar com um NPC perto do spawn, navegar pelas opções e
encerrar; não conseguir falar de longe; não conseguir atacá-lo; dois
jogadores conversam com o mesmo NPC sem interferir um no outro.

### T12 — Missões

- Novo `engine-quests`: missão com objetivos (matar N de um tipo, coletar N de
  um item, falar com alguém), progresso e estados (disponível, ativa,
  concluída, entregue). Reage a eventos, não conhece o jogo.
- `game-sim`: missões na tabela de conteúdo, oferecidas e entregues por NPC
  via diálogo (T11); os eventos da simulação (morte, coleta, conversa)
  alimentam o progresso; recompensa em experiência e itens.
- `client`: lista de missões ativas no HUD com progresso; aviso ao concluir.

**Testar jogando:** aceitar "mate 3 inimigos" com o NPC, ver o contador
subir, voltar, entregar e receber experiência e item; não dá para entregar
duas vezes.

### T13 — Salvar personagem e progresso

- `game-sim`: trait de armazenamento de personagem (porta), com implementação
  em memória para testes. Salva nível, experiência, atributos, inventário,
  equipamento e missões.
- `game-protocol`: o cliente informa um nome de personagem ao entrar.
- `server`: adaptador que grava em disco ao sair e a cada intervalo e carrega
  ao entrar. Pasta configurável por variável de ambiente, lida em `config.rs`.

**Testar jogando:** evoluir o personagem, fechar cliente e servidor, abrir de
novo com o mesmo nome e encontrar tudo; outro nome começa do zero.

### T14 — Predição de movimento no cliente

- `client`: roda o mesmo `engine-physics` localmente para o próprio jogador e
  reconcilia com o servidor usando `last_input_seq`, reaplicando os inputs
  ainda não confirmados. O servidor continua sendo a autoridade.

**Testar jogando:** o movimento responde na hora, sem o atraso de hoje; bater
numa parede não treme; tomar um empurrão de inimigo não causa "teleporte"
estranho.

## Pendências

Problemas encontrados fora do escopo de uma tarefa entram aqui, com uma linha
cada, até virarem tarefa.

- O cliente depende de `game-sim` indiretamente, via `game-protocol`.
