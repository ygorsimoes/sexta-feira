# Sexta-feira — Visão e Arquitetura

> Status: rascunho inicial, anterior ao desenvolvimento. Registra as decisões tomadas na fase de refinamento da ideia.

## 1. O que é

A Sexta-feira é um plugin agnóstico de harness que transforma um agente de IA em um mordomo/assistente virtual pessoal, inspirado no J.A.R.V.I.S. e na Sexta-feira dos filmes do Homem de Ferro.

Ela tem dois papéis:

- **Setup**: instala e configura a Sexta-feira no harness escolhido.
- **Motor**: roda continuamente e fornece memória, ferramentas, regras de autonomia e rotinas a qualquer harness conectado.

O princípio central é que o valor fica fora do harness. O motor é o corpo (memória, ferramentas, regras, agenda) e o harness é o cérebro (o LLM que raciocina). Cada adapter apenas traduz o motor para o harness correspondente.

## 2. Decisões tomadas

| Tema | Decisão |
|---|---|
| Escopo | Assistente pessoal completo: briefing, agenda, e-mail, tarefas, projetos, finanças, automação residencial, pesquisa |
| Autonomia | Alta: age sozinha no que é de baixo risco, pede confirmação no que é importante, e avisa o que fez |
| Canal | Telegram (único, por enquanto) |
| Memória | Única e compartilhada entre todos os harnesses |
| Persona | Tom do J.A.R.V.I.S. (mordomo formal), nome Sexta-feira |
| Idioma | pt-BR; inglês no futuro |
| Harnesses | Hermes Agent (primeiro), OpenClaw, Claude Code, Claude Desktop/claude.ai, OpenCode |
| Público | Uso pessoal (um único usuário) |
| Infraestrutura | VPS sempre ligada |
| Linguagem do core | Rust |
| Integrações | Composio |
| Privacidade | Sem restrições agora; a arquitetura já prevê restrições futuras |
| Quem "pensa" nas tarefas proativas | O motor acorda o harness principal (Hermes) para executar a tarefa; o motor não tem loop de LLM próprio |
| Dono do bot do Telegram | O gateway nativo do Hermes cuida da conversa; o motor apenas envia avisos proativos e pedidos de confirmação |

## 3. Arquitetura

```
                        Telegram
                           │
┌──────────────────────────┴──────────────────────────────┐
│                VPS · sexta-feira (Rust)                 │
│                                                         │
│  persona · memória · agendador · motor de políticas ·   │
│  auditoria · ponte Composio · servidor MCP (HTTP+auth)  │
└────┬──────────┬───────────┬──────────────┬──────────────┘
     │   MCP + skills + persona            │
  Hermes    OpenClaw    Claude Code /    OpenCode
  (1º)                  Desktop / .ai
```

### 3.1 Componentes do motor

- **Servidor MCP (HTTP, com autenticação)**: única interface entre os harnesses e o motor. Por ser remoto, atende também o Claude Desktop/claude.ai, que só aceitam conectores MCP remotos.
- **Memória central**: preferências, pessoas, rotinas e histórico relevante. Os harnesses consultam e gravam via MCP. Proposta inicial: SQLite com busca vetorial.
- **Ponte Composio**: todas as integrações externas (Gmail, Google Calendar etc.) passam pelo motor. Nenhum harness fala direto com o Composio.
- **Motor de políticas**: decide, para cada ação, se ela é livre, se é executada com aviso ou se exige confirmação.
- **Auditoria**: registro de tudo o que a Sexta-feira fez, com desfazer quando a ação permitir.
- **Agendador**: dispara as rotinas (ex.: briefing matinal) acordando o harness principal.
- **Persona**: fonte única da identidade; cada adapter a converte para o formato do harness.

### 3.2 Fluxo de uma ação externa

```
harness → MCP da Sexta-feira → motor de políticas → Composio → serviço externo
                                      │
                                      └→ (se exigir confirmação) Telegram ✅ / ❌
```

### 3.3 Motor de políticas

| Nível | Comportamento | Exemplos |
|---|---|---|
| Livre | Executa sem avisar | Ler e-mails, consultar agenda |
| Age e avisa | Executa e notifica no Telegram | Criar evento, criar lembrete |
| Confirmação | Só executa após aprovação por botão no Telegram | Enviar e-mail em nome do usuário, pagamentos |

A confiança é progressiva: o usuário pode promover uma ação de "confirmação" para "age e avisa" conforme a Sexta-feira se mostra confiável.

### 3.4 Privacidade futura

Cada dado e cada ferramenta recebe uma classificação desde o início (ex.: `privado`). Quando houver dados que não podem ir para um LLM na nuvem, a restrição vira uma regra do motor de políticas, sem reescrita.

## 4. Pontos de extensão dos harnesses

O denominador comum entre os harnesses é **MCP + skills no formato SKILL.md**. A tabela abaixo é preliminar e precisa ser validada na documentação atual de cada projeto antes da implementação do respectivo adapter.

| | Hermes | OpenClaw | Claude Code | Claude Desktop/.ai | OpenCode |
|---|---|---|---|---|---|
| MCP | sim | sim | sim | sim (remoto) | sim |
| Skills (SKILL.md) | sim | sim | sim | sim | a confirmar |
| Persona/contexto | config própria | `SOUL.md` / `AGENTS.md` | `CLAUDE.md` | instruções de projeto | `AGENTS.md` |
| Agendamento nativo | sim | sim | limitado | não | não |
| Canais de mensagem | sim | sim | não | não | não |

## 5. Estrutura prevista do repositório

```
sexta-feira/
├── core/            # motor em Rust (daemon + servidor MCP)
├── skills/          # SKILL.md compartilhadas entre os harnesses
├── persona/         # fonte única da identidade
├── adapters/
│   ├── hermes/
│   ├── openclaw/
│   ├── claude-code/
│   ├── claude-desktop/
│   └── opencode/
├── cli/             # sexta-feira setup --harness=<nome>
└── docs/
```

## 6. MVP

O primeiro recorte prova as partes difíceis (memória, autonomia, proatividade, conexões) em um único harness, o Hermes:

1. Persona Sexta-feira no Hermes.
2. Memória central.
3. Ponte Composio com motor de políticas, com Gmail e Google Calendar como primeiros casos.
4. Briefing matinal no Telegram.
5. Confirmações por botão no Telegram.

Os demais adapters entram depois do MVP, na ordem a definir.

## 7. Questões em aberto

- Validar a tabela da seção 4 na documentação de cada harness, começando pelo Hermes (plugins, skills, MCP, cron, gateway do Telegram, como acordá-lo de fora).
- Como o motor acorda o Hermes para as tarefas proativas (API, CLI ou cron nativo do Hermes).
- Como os botões de confirmação convivem com o gateway do Hermes, já que um bot só pode ter um processo lendo atualizações.
- Como acessar o Composio a partir do Rust (API REST ou MCP do Composio).
- Mecanismo de autenticação do servidor MCP exposto na VPS.
- Modelo de dados da memória.
