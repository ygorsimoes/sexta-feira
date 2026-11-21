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
| Público | Opinativo para fluxo de uso pessoal, distribuído em código aberto (MIT) para adoção e contribuições |
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
| Persona/contexto | `SOUL.md` / `memories/USER.md` | `SOUL.md` / `AGENTS.md` | `CLAUDE.md` | instruções de projeto | `AGENTS.md` |
| Agendamento nativo | sim | sim | limitado | não | não |
| Canais de mensagem | sim | sim | não | não | não |

## 5. Estrutura do repositório

```text
sexta-feira/
├── crates/
│   ├── sexta-feira-core/      # modelo agnóstico e motor declarativo de setup
│   ├── sexta-feira-adapters/  # catálogo de harnesses e tradução para cada destino
│   └── sexta-feira-cli/       # binário `sexta-feira`
├── recursos/
│   ├── mordomo/               # fonte única da persona e do perfil do usuário
│   └── skills/                # skills (SKILL.md), incluindo instalação guiada por agente
├── docs/                      # visão, arquitetura, stack e especificações
└── spikes/                    # experimentos descartáveis, fora do workspace
```

Ainda a criar, conforme os próximos incrementos avançarem:

- Demais skills (`SKILL.md`), scripts utilitários e presets de `config.yaml` em `recursos/`.
- Demais adapters por harness.

As bibliotecas e ferramentas estão em [stack.md](stack.md).

## 6. MVP

O primeiro recorte prova o setup automatizado e idempotente em um único harness, o **Hermes Agent**:

1. **Concluído no Recorte 1**: fonte única da **persona e das regras de conduta** da Sexta-feira em `pt-BR` (tom J.A.R.V.I.S., tratamento como `"senhor"`), embutida em `recursos/mordomo/`.
2. **Concluído no Recorte 1**: adapter do Hermes (`$HERMES_HOME/profiles/sexta-feira/`) gerando `SOUL.md` integral (com detecção de conflitos por `.sexta-feira-estado.toml`) e `memories/USER.md` por bloco delimitado.
3. **Concluído no Recorte 1**: subcomandos `plano`, `aplicar` e `verificar` na CLI `sexta-feira`.

Outros blocos (memória persistente, canais como Telegram, scripts de apoio, integrações, `config.yaml` e rotinas agendadas no próprio harness) serão incorporados ao pacote de setup em incrementos seguintes, conforme forem detalhados e validados. Os demais adapters entram depois do MVP, na ordem a definir.

## 7. Questões em aberto

- Validar na documentação dos demais harnesses (`OpenClaw`, `Claude Code`, `Claude Desktop/.ai`, `OpenCode`) os formatos exatos de instalação antes de implementar cada adapter.
- Avaliar quais blocos entram na sequência do MVP no Hermes (por exemplo: `config.yaml`, memória persistente, Telegram, integrações e rotinas).
