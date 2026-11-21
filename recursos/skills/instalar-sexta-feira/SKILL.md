---
name: instalar-sexta-feira
description: Instala e configura a Sexta-feira (mordomo virtual) no harness em que o agente está rodando, usando a CLI `sexta-feira` quando houver adapter oficial e fazendo tradução assistida de `recursos/mordomo/*.toml` nos demais harnesses.
---

# Instalar a Sexta-feira neste agente

> **REGRA DE ATIVAÇÃO IMEDIATA:** Se o usuário pediu para instalar ou configurar a Sexta-feira agora (por exemplo, enviando a URL ou o texto desta skill no chat), **execute os passos abaixo imediatamente**. Não apenas salve este arquivo como skill sem aplicá-lo, a menos que o usuário peça explicitamente somente para guardar a skill.

## Princípios deste procedimento

1. **CLI primeiro, inteligência do agente ao redor:** use a CLI `sexta-feira` sempre que o harness atual tiver adapter oficial (`disponivel: true`), aproveitando plano, diff, estado idempotente, backups e verificação.
2. **Fallback transparente:** se o harness ainda não tiver adapter na CLI (ou se não for possível obter/compilar a CLI no ambiente), **avise o usuário antes** e ofereça a instalação assistida lendo `recursos/mordomo/*.toml`.
3. **Zero ações destrutivas às cegas:** nunca sobrescreva arquivos existentes, nunca use `--forcar` e nunca ignore `SHA256SUMS.txt` sem mostrar o impacto e obter confirmação explícita do usuário.
4. **Sem suposições inventadas:** inspecione o ambiente real (`PATH`, variáveis de ambiente, diretórios do harness, `--help` local do harness) em vez de presumir caminhos ou flags.

---

## Passo 1 — Diagnóstico do ambiente e do harness

Colete apenas o necessário para decidir os próximos passos:

1. **Sistema e arquitetura:** execute `uname -s` e `uname -m`.
2. **Ferramentas disponíveis:** verifique `which sexta-feira`, `curl`, `tar`, `sha256sum` ou `shasum`, `git`, `cargo` e `rustup`.
3. **Repositório local:** verifique se o diretório atual já é um clone de `sexta-feira` (possui `Cargo.toml` com `sexta-feira-cli` e `recursos/mordomo/`).
4. **Harness atual:** identifique em qual harness você está executando agora (ex.: Hermes Agent, OpenClaw, Claude Code, Claude Desktop, OpenCode ou outro) e localize seus diretórios/arquivos de configuração reais no ambiente (ex.: `$HERMES_HOME` ou `~/.hermes` no Hermes; `~/.claude` ou `CLAUDE.md` no Claude Code; `AGENTS.md` / configurações locais em outros harnesses).

---

## Passo 2 — Obter ou localizar a CLI `sexta-feira`

Tente, nesta ordem, disponibilizar um comando `sexta-feira` executável:

1. **Já disponível no sistema:** se `sexta-feira --version` funcionar no `PATH`, use-o diretamente.
2. **Dentro do repositório local:** se `./target/release/sexta-feira` existir, use-o; caso contrário, se `cargo` estiver disponível, compile (`cargo build --release --package sexta-feira-cli`) ou execute via `cargo run -q -p sexta-feira-cli --`.
3. **Binário oficial de Release (GitHub `ygorsimoes/sexta-feira`):**
   - Mapeie o alvo publicado no CD:
     - Linux `x86_64` → `x86_64-unknown-linux-musl`
     - Linux `aarch64` / `arm64` → `aarch64-unknown-linux-musl`
     - macOS `arm64` (`Apple Silicon`) → `aarch64-apple-darwin`
   - Consulte a última release em `https://api.github.com/repos/ygorsimoes/sexta-feira/releases/latest` (ou via `gh release`).
   - Baixe o `.tar.gz` correspondente **e** `SHA256SUMS.txt`.
   - **Obrigatório:** valide o checksum com `sha256sum -c` (Linux) ou `shasum -a 256 -c` (macOS) filtrando pelo arquivo baixado antes de extrair. Se o hash **divergir**, aborte imediatamente e avise o usuário. Se `SHA256SUMS.txt` ou a ferramenta de checksum estiver indisponível, não extraia às cegas: tente o item 4 (compilação), siga para o item 5 (fallback) ou peça consentimento explícito explicando o risco.
   - Extraia o binário `sexta-feira` e instale em um diretório do `PATH` do usuário (ex.: `~/.local/bin/sexta-feira`). Se precisar criar o diretório ou ajustar o `PATH` do shell, confirme antes com o usuário.
4. **Compilação alternativa via código-fonte:** se a arquitetura/SO não tiver binário pré-compilado (por exemplo, macOS Intel) **ou se não for possível baixar/verificar o pacote de Release**, mas `git` e `cargo`/`rustup` estiverem instalados, ofereça clonar o repositório e rodar `cargo build --release --package sexta-feira-cli`.
5. **CLI indisponível ou recusada:** se o ambiente não tiver acesso a Releases/Cargo, se a compilação falhar (ex.: `rustc` incompatível com `rust-toolchain.toml` sem `rustup`/rede), se o sistema não executar o binário ou se o usuário preferir não baixar/compilar a CLI agora, registre o motivo claramente e pule o Passo 3 seguindo para o **Passo 4** e depois **Passo 5B (Fallback assistido)**.

---

## Passo 3 — Verificar suporte do harness e diagnosticar ambiente na CLI

Se a CLI estiver disponível, execute o diagnóstico completo (ou `harnesses`):

```bash
sexta-feira --json diagnostico
```

Você também pode combinar `--json` com `-v` ou `-vv` em qualquer comando para receber contexto rico em `stderr` (camadas de configuração, destino resolvido, estratégias e métricas) sem poluir o JSON de `stdout`.

Interprete os campos de `harnesses` (`id`, `disponivel`, `status`, `sincronizado`) e `definicao`:

- Se o harness atual corresponder a um item com `"disponivel": true`, siga pelo **Passo 5A** após a personalização.
- Se aparecer com `"disponivel": false` (status `"em breve"`) ou se o harness atual nem estiver listado no catálogo, você seguirá pelo **Passo 5B** após a personalização.

---

## Passo 4 — Personalização em camadas (`${XDG_CONFIG_HOME:-~/.config}/sexta-feira/`)

A CLI lê configurações em camadas (`figment`), nesta ordem de precedência:
1. Padrão embutido (`recursos/mordomo/persona.toml` e `usuario.toml`).
2. Diretório de configuração do usuário (`<dir-config>`): se `XDG_CONFIG_HOME` estiver definida e não vazia, usa **exclusivamente** `$XDG_CONFIG_HOME/sexta-feira/`; caso contrário, usa `~/.config/sexta-feira/`.
3. Arquivos explícitos via `--persona <arquivo.toml>` e `--usuario <arquivo.toml>`.

Antes de aplicar:

1. Verifique se `<dir-config>/usuario.toml` (ou `persona.toml`) já existe e mostre o estado atual ao usuário.
2. Faça **uma pergunta curta e prática** ao usuário para confirmar ou ajustar o básico em `usuario.toml`:
   - Forma de tratamento desejada (`tratamento`, padrão `"senhor"` — ex.: `"senhor"`, nome próprio, etc.);
   - Preferências pessoais específicas para definir em `preferencias` (ou manter o padrão).
3. Se o usuário quiser personalizar e a CLI estiver disponível, prefira usar `sexta-feira iniciar --nao-interativo --tratamento "<valor>"` (com `--sobrescrever` se o arquivo já existir e houver mudanças; gera `.bak` e valida em memória antes de gravar). Se estiver editando `<dir-config>/usuario.toml` ou `persona.toml` diretamente, observe estas regras rígidas do motor:
   - **Sem chaves desconhecidas:** `#[serde(deny_unknown_fields)]` rejeita qualquer chave fora de:
     - **`usuario.toml`**: `tratamento` (string), `idioma` (string, ex.: `"pt-BR"`), `preferencias` (lista de strings).
     - **`persona.toml`**: `nome` (string), `arquetipo` (string), `idioma` (string), `resumo` (string), `tom` (lista de strings), `diretrizes` (lista de strings), `limites` (lista de strings).
   - **Listas substituem integralmente o padrão:** definir `preferencias`, `tom`, `diretrizes` ou `limites` substitui toda a lista base (não concatena). Para *acrescentar* itens mantendo os padrões, consulte `recursos/mordomo/*.toml` e inclua os itens desejados; para herdar o padrão, omita a chave.
   - **Sem valores vazios e mantendo concisão:** nunca grave strings vazias/em branco nem listas vazias (`[]`), pois falham na validação. No Hermes, `SOUL.md` tem limite de 80 linhas — mantenha listas e textos objetivos.

---

## Passo 5A — Caminho oficial via CLI (`disponivel: true`)

Quando o harness atual tiver adapter oficial (ex.: `hermes`):

1. **Definir alvo:**
   - Verifique o diretório base (`--destino`, ou `$HERMES_HOME` / `~/.hermes` no Hermes) e confirme se o usuário quer usar o perfil isolado padrão (`sexta-feira`) ou informar `--perfil <nome>`.
2. **Pré-visualizar o plano (use `-v` / `-vv` se precisar inspecionar camadas e estratégias em `stderr`):**
   ```bash
   sexta-feira --json -v plano <harness> [--destino <dir>] [--perfil <nome>]
   ```
   Apresente ao usuário quais arquivos serão criados/atualizados/removidos.
3. **Tratar conflitos com segurança:**
   - Se `resumo.conflitos > 0`, exiba o caminho, o motivo e o diff de cada conflito.
   - Explique que `--forcar` sobrescreve gerando backup `.bak` numerado e **peça confirmação explícita** antes de usá-lo (ou proponha ajustar manualmente / escolher outro `--perfil`).
4. **Aplicar e verificar:**
   ```bash
   sexta-feira --json aplicar <harness> [--destino <dir>] [--perfil <nome>] [--forcar]
   sexta-feira --json verificar <harness> [--destino <dir>] [--perfil <nome>]
   ```
   Só considere a instalação concluída quando `verificar` sair com código `0` e `"sincronizado": true`.
5. **Orientar ativação:** informe o caminho do perfil aplicado e verifique localmente (ex.: `<harness> --help` ou documentação local) o comando exato para iniciar uma sessão usando aquele perfil, sem inventar flags não verificadas.

---

## Passo 5B — Fallback assistido pelo agente (sem adapter na CLI ou sem CLI)

Use este caminho quando `sexta-feira --json harnesses` mostrar `"disponivel": false` para o harness atual, quando o harness não estiver na lista ou quando a CLI não puder ser instalada/executada no ambiente:

1. **Aviso explícito obrigatório:** informe claramente ao usuário:
   > A CLI `sexta-feira` ainda não possui suporte/adapter oficial para `<nome-do-harness>` (ou não pôde ser executada neste ambiente). Se você concordar, posso tentar fazer uma **instalação assistida** lendo os recursos da Sexta-feira e adaptando diretamente para o formato deste harness.
   Aguarde o "sim" do usuário antes de alterar arquivos do harness.
2. **Ler a definição única do mordomo:**
   - Leia `recursos/mordomo/persona.toml` e `recursos/mordomo/usuario.toml` do repositório local, ou diretamente das URLs oficiais:
     - `https://raw.githubusercontent.com/ygorsimoes/sexta-feira/main/recursos/mordomo/persona.toml`
     - `https://raw.githubusercontent.com/ygorsimoes/sexta-feira/main/recursos/mordomo/usuario.toml`
   - Se existirem `persona.toml` ou `usuario.toml` em `<dir-config>` (`${XDG_CONFIG_HOME:-~/.config}/sexta-feira/`), aplique seus campos sobre o padrão (listas sobrescrevem a lista base; campos omitidos herdam o padrão).
3. **Mapear para as convenções reais do harness:**
   - Descubra onde este harness armazena instruções/persona e perfil/memórias do usuário (por exemplo: `CLAUDE.md` no projeto ou em `~/.claude/CLAUDE.md` no Claude Code; `AGENTS.md` / configuração do projeto no OpenCode ou OpenClaw; instruções de projeto no Claude Desktop; regras ou instruções equivalentes no harness atual).
   - Traduza os campos (`nome`, `arquetipo`, `idioma`, `resumo`, `tom`, `diretrizes`, `limites`, `tratamento`, `preferencias`) em Markdown limpo, fiel ao tom formal J.A.R.V.I.S. em `pt-BR`.
4. **Aplicar com segurança:**
   - Mostre antes ao usuário **quais arquivos** pretende criar ou editar e um resumo do conteúdo.
   - Se o arquivo destino **já existir** e tiver conteúdo prévio do usuário, **preserve-o**: insira/atualize apenas um bloco delimitado por `<!-- sexta-feira:inicio -->` e `<!-- sexta-feira:fim -->`, ou crie um backup `<arquivo>.bak` após confirmação explícita.
5. **Conferência final:** releia os arquivos gravados, confirme que o conteúdo e a formatação estão corretos e informe ao usuário como ativar ou testar a nova persona neste harness (se precisar reiniciar sessão/abrir nova conversa, avise).

---

## Passo 6 — Opcional: guardar esta skill para manutenção futura

Ao concluir a instalação, se o harness atual suportar skills no padrão `SKILL.md` e você ainda não tiver essa skill instalada localmente, pergunte se o usuário deseja salvar uma cópia de `instalar-sexta-feira/SKILL.md` no diretório de skills do harness para facilitar futuras atualizações, reverificações ou ajustes de perfil.
