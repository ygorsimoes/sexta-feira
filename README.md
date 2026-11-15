# Sexta-feira

[![CI](https://github.com/ygorsimoes/sexta-feira/actions/workflows/ci.yml/badge.svg)](https://github.com/ygorsimoes/sexta-feira/actions/workflows/ci.yml)

Plugin agnóstico de harness que transforma um agente de IA em um mordomo/assistente virtual pessoal, inspirado no J.A.R.V.I.S. e na Sexta-feira dos filmes do Homem de Ferro.

O valor fica fora do harness: o motor fornece memória, ferramentas, regras de autonomia e rotinas; o harness fornece o LLM que raciocina. Cada adapter apenas traduz o motor para um harness.

## Situação

O projeto está no início. A fundação do repositório existe (workspace, CI, convenções); o motor ainda não. O primeiro recorte (MVP) está descrito em [docs/visao.md](docs/visao.md#6-mvp).

| Parte | Situação |
|---|---|
| Fundação do repositório | pronta |
| Motor (`sexta-feira-core`) | não iniciado |
| Adapter do Hermes | não iniciado |
| Demais adapters | depois do MVP |

## Harnesses previstos

- Hermes Agent (primeiro)
- OpenClaw
- Claude Code
- Claude Desktop / claude.ai
- OpenCode

## Estrutura

```
crates/
├── sexta-feira-core/      # motor, sem conhecimento de harness
├── sexta-feira-adapters/  # tradução do motor para cada harness
└── sexta-feira-cli/       # binário `sexta-feira`
docs/                      # visão, arquitetura e stack
spikes/                    # experimentos descartáveis, fora do workspace
```

## Desenvolvimento

A versão do Rust está fixada em `rust-toolchain.toml`; o `rustup` a instala sozinho.

```bash
cargo run -p sexta-feira-cli -- harnesses   # lista os harnesses previstos
just                                        # lista as tarefas locais
```

Todo trabalho entra na `main` por pull request, e a validação é o CI da PR: formatação, lints, testes, documentação, dependências, ortografia e segurança dos workflows. O fluxo completo, das issues ao merge, está em [CONTRIBUTING.md](CONTRIBUTING.md).

## Documentação

- [Visão e arquitetura](docs/visao.md)
- [Stack](docs/stack.md)
- [Como contribuir](CONTRIBUTING.md)
- [Guia para agentes](AGENTS.md)

## Licença

[MIT](LICENSE)
