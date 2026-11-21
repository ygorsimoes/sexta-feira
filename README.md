# Sexta-feira

[![CI](https://github.com/ygorsimoes/sexta-feira/actions/workflows/ci.yml/badge.svg)](https://github.com/ygorsimoes/sexta-feira/actions/workflows/ci.yml)
[![Release](https://github.com/ygorsimoes/sexta-feira/actions/workflows/release.yml/badge.svg)](https://github.com/ygorsimoes/sexta-feira/actions/workflows/release.yml)

Pacote opinativo e agnóstico de harness para configurar um agente de IA como um mordomo virtual pessoal, inspirado no J.A.R.V.I.S. e na Sexta-feira dos filmes do Homem de Ferro.

A identidade, as regras de conduta, as skills e as configurações preferidas ficam versionadas fora do harness em uma fonte única. Cada adapter apenas traduz e aplica esse setup no harness escolhido, evitando ter que configurar tudo do zero em cada máquina ou VPS.

## Situação

O primeiro recorte do MVP está implementado: o núcleo declarativo em `sexta-feira-core` carrega a persona padrão (estilo J.A.R.V.I.S. em `pt-BR`, tratando o usuário por `"senhor"`) e o adapter do Hermes provisiona o perfil isolado `sexta-feira` (`SOUL.md` e `memories/USER.md`) via CLI.

| Parte | Situação |
|---|---|
| Fundação do repositório | pronta |
| Modelo do setup e motor declarativo (`sexta-feira-core`) | implementado (Recorte 1 — persona) |
| Adapter do Hermes (`SOUL.md` e `memories/USER.md`) | implementado (Recorte 1) |
| Skills, scripts, `config.yaml` e rotinas no Hermes | próximos incrementos |
| Demais adapters (`OpenClaw`, `Claude Code`, `Claude Desktop`, `OpenCode`) | depois do MVP |

## Harnesses previstos

- Hermes Agent (primeiro — implementado para persona e perfil)
- OpenClaw
- Claude Code
- Claude Desktop / claude.ai
- OpenCode

## Estrutura

```text
crates/
├── sexta-feira-core/      # modelo agnóstico do mordomo e motor declarativo
├── sexta-feira-adapters/  # catálogo de harnesses e tradução para cada destino
└── sexta-feira-cli/       # binário `sexta-feira`
recursos/
└── mordomo/               # fonte única da persona e do perfil do usuário
docs/                      # visão, arquitetura, stack e especificações
spikes/                    # experimentos descartáveis, fora do workspace
```

## Desenvolvimento

A versão do Rust está fixada em `rust-toolchain.toml`; o `rustup` a instala sozinho.

```bash
cargo run -p sexta-feira-cli -- harnesses               # lista os harnesses previstos
cargo run -p sexta-feira-cli -- plano hermes            # exibe o plano sem alterar o disco
cargo run -p sexta-feira-cli -- aplicar hermes          # aplica o perfil `sexta-feira` no Hermes
cargo run -p sexta-feira-cli -- verificar hermes        # verifica se o perfil está sincronizado
just                                                    # lista as tarefas locais
```

Todo trabalho entra na `main` por pull request, e a validação é o CI da PR: formatação, lints, testes, documentação, dependências e segurança dos workflows. O fluxo completo, das issues ao merge, está em [CONTRIBUTING.md](CONTRIBUTING.md).

## Documentação

- [Visão e arquitetura](docs/visao.md)
- [Stack](docs/stack.md)
- [Como contribuir](CONTRIBUTING.md)
- [Guia para agentes](AGENTS.md)

## Licença

[MIT](LICENSE)
