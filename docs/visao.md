# Sexta-feira — Visão e Arquitetura

> Status: rascunho inicial, focado no pacote de setup e nos adapters por harness.

## 1. O que é

A Sexta-feira é um pacote opinativo e agnóstico de harness que transforma um agente de IA em um mordomo virtual pessoal, inspirado no J.A.R.V.I.S. e na Sexta-feira dos filmes do Homem de Ferro.

Seu objetivo nesta fase é **instalar, configurar e manter** um ambiente completo de mordomo (persona, regras de conduta, skills, scripts e preferências pessoais) no harness escolhido — começando pelo **Hermes Agent** em uma VPS — para que não seja necessário reconfigurar tudo do zero a cada instalação.

O princípio central é manter o conhecimento do mordomo versionado fora do harness, em uma fonte única. O pacote define **o que** o mordomo é e **como** ele deve se comportar; cada adapter apenas traduz e aplica essa configuração no formato esperado pelo harness correspondente.

## 2. Decisões tomadas

| Tema | Decisão |
|---|---|
| Escopo atual | Pacote de setup opinativo do mordomo (persona, regras, skills, scripts e configurações de ambiente) |
| Primeiro alvo | Hermes Agent em VPS pessoal |
| Persona | Tom do J.A.R.V.I.S. (mordomo formal), nome Sexta-feira |
| Idioma | `pt-BR` (inglês no futuro) |
| Harnesses previstos | Hermes Agent (primeiro), OpenClaw, Claude Code, Claude Desktop/claude.ai, OpenCode |
| Público | Uso pessoal (um único usuário) |
| Linguagem da CLI e dos adapters | Rust |
| Execução em runtime | Feita pelo próprio harness; a Sexta-feira não mantém um servidor/motor contínuo próprio por enquanto |

## 3. Arquitetura

```text
  recursos do mordomo (persona · regras · skills · scripts)
                             │
                ┌────────────┴────────────┐
                │   sexta-feira (Rust)    │
                │  core (modelo) + CLI    │
                └────────────┬────────────┘
                             │ adapters
     ┌──────────┬────────────┼────────────┬───────────┐
   Hermes    OpenClaw    Claude Code   Claude       OpenCode
   (1º)                                Desktop/.ai
```

### 3.1 Papéis de cada parte

- **Definição do mordomo (`sexta-feira-core`)**: representa de forma agnóstica de harness a configuração do mordomo (persona, regras de conduta, catálogo de skills, scripts e verificações).
- **Adapters (`sexta-feira-adapters`)**: traduzem a definição do mordomo para os arquivos, diretórios e convenções de cada harness, aplicando o setup de maneira idempotente.
- **CLI (`sexta-feira-cli`)**: binário `sexta-feira` usado para instalar, atualizar e validar o setup no ambiente do usuário.
- **Recursos compartilhados**: fonte única versionada da persona, das regras, das skills (`SKILL.md`) e dos scripts de apoio.

## 4. Pontos de extensão dos harnesses

O denominador comum entre os harnesses é **MCP + skills no formato SKILL.md**. A tabela abaixo é preliminar e precisa ser validada na documentação atual de cada projeto antes da implementação do respectivo adapter.

| | Hermes | OpenClaw | Claude Code | Claude Desktop/.ai | OpenCode |
|---|---|---|---|---|---|
| MCP | sim | sim | sim | sim (remoto) | sim |
| Skills (SKILL.md) | sim | sim | sim | sim | a confirmar |
| Persona/contexto | config própria | `SOUL.md` / `AGENTS.md` | `CLAUDE.md` | instruções de projeto | `AGENTS.md` |
| Agendamento nativo | sim | sim | limitado | não | não |
| Canais de mensagem | sim | sim | não | não | não |

## 5. Estrutura do repositório

```text
sexta-feira/
├── crates/
│   ├── sexta-feira-core/      # modelo e contratos agnósticos de harness
│   ├── sexta-feira-adapters/  # tradução e aplicação do setup para cada harness
│   └── sexta-feira-cli/       # binário `sexta-feira`
├── docs/                      # visão, arquitetura e stack
└── spikes/                    # experimentos descartáveis, fora do workspace
```

Ainda a criar, conforme o desenvolvimento avançar:

- Fonte única da persona, regras de conduta, skills (`SKILL.md`) e scripts utilitários.
- Modelos de configuração e instalação específicos de cada harness (começando pelo Hermes).

As bibliotecas e ferramentas estão em [stack.md](stack.md).

## 6. MVP

O primeiro recorte prova o setup automatizado e idempotente em um único harness, o **Hermes Agent**:

1. Fonte única da **persona e das regras de conduta** da Sexta-feira em `pt-BR` (tom J.A.R.V.I.S.).
2. Adapter do Hermes capaz de instalar e validar essa configuração no ambiente do Hermes sem passos manuais repetitivos.
3. Comando na CLI `sexta-feira` para aplicar e verificar o setup do Hermes na VPS.

Outros blocos (memória persistente, canais como Telegram, scripts de apoio, integrações e rotinas agendadas no próprio harness) serão incorporados ao pacote de setup em incrementos seguintes, conforme forem detalhados e validados. Os demais adapters entram depois do MVP, na ordem a definir.

## 7. Questões em aberto

- Validar a tabela da seção 4 na documentação atual do Hermes (onde ficam persona/system prompt, skills, MCP, cron e `config.yaml`).
- Definir o formato da fonte única da persona e das regras de conduta e se esses ativos serão embutidos no binário `sexta-feira`.
- Definir como o setup garante idempotência sem sobrescrever ajustes locais feitos manualmente pelo usuário no harness.
- Avaliar quais blocos entram na sequência do MVP (por exemplo, memória com `ai-memory` — ainda pendente dos testes descritos em [spikes/ai-memory-hermes](../spikes/ai-memory-hermes/README.md) —, Telegram, integrações e rotinas).
