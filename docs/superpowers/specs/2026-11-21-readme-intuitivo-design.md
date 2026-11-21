# Design: `README.md` mais bonito e intuitivo

Data: 2026-11-21
Status: rascunho para revisão

- **Intenção e porquê:** tornar a página inicial do repositório acolhedora, visual e imediatamente acionável para quem baixa o binário da Release ou desenvolve no projeto, já documentando todas as novidades das PRs #14, #16 e #18.
- **Objetivos:**
  - Cabeçalho centralizado com badges (`CI`, `Release`, `Rust 1.100+`, `Licença MIT`) e proposta de valor em uma frase.
  - Diagrama visual (`mermaid`) mostrando as camadas de configuração (`recursos/mordomo/` + `~/.config/sexta-feira/` + flags) alimentando `sexta-feira` e os adapters.
  - Seção de **Início rápido** passo a passo (download de Release com checksums ou `cargo run`), seguida de **Personalização** (exemplo curto de `~/.config/sexta-feira/usuario.toml`), **Referência rápida da CLI** (`--destino`, `--perfil`, `--persona`, `--usuario`, `--forcar`, `--json`, `SEXTA_FEIRA_LOG`), **Situação dos harnesses**, **Estrutura** e **Desenvolvimento**.
- **Não-objetivos:** alterar código Rust, workflows ou demais documentos em `docs/` (além deste design/plano).
- **Abordagens consideradas:** (1) `README.md` focado no fluxo real com Mermaid + exemplos curtos copiáveis (Escolhida); (2) README minimalista apenas com lista de comandos (descartada por não ajudar quem chega pela primeira vez a instalar e personalizar o mordomo).
- **Restrições:** idioma `pt-BR`, apenas comandos, flags, alvos de release e comportamentos observados no código atual.
