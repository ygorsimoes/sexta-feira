# Sexta-feira

[![CI](https://github.com/ygorsimoes/sexta-feira/actions/workflows/ci.yml/badge.svg)](https://github.com/ygorsimoes/sexta-feira/actions/workflows/ci.yml)

Pacote opinativo e agnóstico de harness para configurar um agente de IA como um mordomo virtual pessoal, inspirado no J.A.R.V.I.S. e na Sexta-feira dos filmes do Homem de Ferro.

A identidade, as regras de conduta, as skills e as configurações preferidas ficam versionadas fora do harness em uma fonte única. Cada adapter apenas traduz e aplica esse setup no harness escolhido, evitando ter que configurar tudo do zero em cada máquina ou VPS.

## Situação

O projeto está no início. A fundação do repositório existe (workspace, CI, convenções); o pacote de setup e os adapters ainda não. O primeiro recorte (MVP) está descrito em [docs/visao.md](docs/visao.md#6-mvp).

| Parte | Situação |
|---|---|
| Fundação do repositório | pronta |
| Modelo do setup e persona (`sexta-feira-core`) | não iniciado |
| Adapter do Hermes | não iniciado |
| Demais adapters | depois do MVP |

## Harnesses previstos

- Hermes Agent (primeiro)
- OpenClaw
- Claude Code
- Claude Desktop / claude.ai
- OpenCode

## Estrutura

```text
crates/
├── sexta-feira-core/      # modelo e contratos agnósticos de harness
├── sexta-feira-adapters/  # tradução e aplicação do setup para cada harness
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

Todo trabalho entra na `main` por pull request, e a validação é o CI da PR: formatação, lints, testes, documentação, dependências e segurança dos workflows. O fluxo completo, das issues ao merge, está em [CONTRIBUTING.md](CONTRIBUTING.md).

## Documentação

- [Visão e arquitetura](docs/visao.md)
- [Stack](docs/stack.md)
- [Como contribuir](CONTRIBUTING.md)
- [Guia para agentes](AGENTS.md)

## Licença

[MIT](LICENSE)
