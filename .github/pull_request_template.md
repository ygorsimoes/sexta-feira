<!-- 💡 O título deve seguir Conventional Commits em pt-BR, ex.: feat(core): adiciona motor de políticas -->

## ✨ O que muda

> **Resumo:** descreva de forma objetiva **o que** mudou e **por quê**.

- **Área / componente:** ...

## 🔗 Issue

<!-- Ex.: Closes #123. Se não houver issue, explique o motivo. -->

## ✅ Como foi verificado

- **Localmente:** `just ci` (ou comandos `cargo` executados)
- **CI da PR:** checks automáticos (`CI`, `Título da PR`, `zizmor`, `cargo-deny`)
- **Observações:** *(se algo não pôde ser verificado localmente, registre aqui)*

## 📋 Checklist

- [ ] **Escopo único:** a PR trata de um assunto só.
- [ ] **Testes:** comportamento novo ou corrigido possui testes (ou explicação acima).
- [ ] **Documentação:** `README.md`, `docs/` e `AGENTS.md` acompanham a mudança.
- [ ] **Isolamento do núcleo:** nada específico de harness entrou em `sexta-feira-core`.
- [ ] **Dependências:** nenhuma biblioteca fora de `docs/stack.md` entrou sem confirmação.
- [ ] **Segurança:** nenhum segredo, token ou dado sensível entrou no repositório.
