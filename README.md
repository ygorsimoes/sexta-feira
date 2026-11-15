# Sexta-feira

[![CI](https://github.com/ygorsimoes/sexta-feira/actions/workflows/ci.yml/badge.svg)](https://github.com/ygorsimoes/sexta-feira/actions/workflows/ci.yml)

Plugin agnóstico de harness que transforma um agente de IA em um mordomo/assistente virtual pessoal, inspirado no J.A.R.V.I.S. e na Sexta-feira dos filmes do Homem de Ferro.

O projeto está no início: a fundação do repositório existe, o motor ainda não.

## Documentação

- [Visão e arquitetura](docs/visao.md)
- [Stack](docs/stack.md)
- [Como contribuir](CONTRIBUTING.md)

## Harnesses previstos

- Hermes Agent (primeiro)
- OpenClaw
- Claude Code
- Claude Desktop / claude.ai
- OpenCode

## Desenvolvimento

```bash
cargo run -p sexta-feira-cli -- harnesses   # lista os harnesses previstos
just ci                                     # formatação, lints, testes e dependências
```

## Licença

[MIT](LICENSE)
