# Teste: ai-memory como memória da Sexta-feira, conectado ao Hermes

Data: 2026-11-15. Versões: ai-memory 3.4.6 (binário da release), Hermes Agent no commit `18e659d` (Python 3.14, extra `mcp`).

Objetivo: verificar se o [ai-memory](https://github.com/akitaonrails/ai-memory), feito para agentes de programação, serve como memória **pessoal** da Sexta-feira e se conecta ao Hermes.

## Resultado

Adotar como opção preferida, com duas verificações ainda pendentes (ver "O que não foi testado").

| Verificação | Resultado |
|---|---|
| Hermes conecta ao ai-memory via MCP | Passou: `hermes mcp test ai-memory` descobriu 27 ferramentas |
| Gravar memórias pessoais em pt-BR | Passou: 12 páginas gravadas no projeto `sexta-feira/pessoal` |
| Turno completo do Hermes consultando a memória | Passou, com um LLM de mentira (roteiro fixo) |
| Captura das sessões do Hermes pelos hooks | Passou: 5 sessões registradas |
| Sessões caírem no projeto pessoal | Passou, mas exige configuração (ver abaixo) |
| Busca por texto em pt-BR | Parcial: 10 de 17 consultas acharam a página certa |
| Busca semântica (embeddings) | Não testada |
| Turno com um modelo real | Não testado |

## Busca por texto: 10 de 17

As memórias e as consultas são fictícias e estão em `teste_pessoal.py`.

Acertou:
- Termo exato, com ou sem acento (`reunião`, `reuniao`).
- Perguntas naturais cujas palavras aparecem na página (`qual o nome da minha esposa`, `quando vence o cartão`). As quatro perguntas naturais do teste acertaram.

Errou (as sete falhas):
- Sinônimos: `mulher` (página diz "esposa"), `remédios` ("medicamentos"), `faxineira` ("diarista"), `médico do coração` ("cardiologista").
- Flexão: `medicamento` ("medicamentos"), `voo` ("voos"), `alergico` ("alérgica"/"alergia").

Esse é o tipo de falha que a busca semântica resolve. O ai-memory tem essa busca, mas ela não pôde ser ligada neste ambiente: o modelo local é baixado de `huggingface.co`, bloqueado pela política de rede da sessão. O modelo local padrão (`all-MiniLM-L6-v2`) é treinado em inglês, então para pt-BR provavelmente será preciso um modelo multilíngue ou um provedor de embeddings externo.

## O que o teste revelou sobre a integração

- **O Hermes não entrega as ferramentas MCP direto ao modelo.** A requisição ao LLM tinha 18 ferramentas, nenhuma do ai-memory. O modelo chega até elas por `tool_search` / `tool_describe` / `tool_call`. O prompt de sistema cita o ai-memory por causa das skills instaladas. Isso vale para qualquer servidor MCP, inclusive o futuro MCP da Sexta-feira.
- **O roteamento das sessões é por diretório.** Sem configuração, as sessões do Hermes foram para um projeto `default/root`. Para caírem em `sexta-feira/pessoal` foi preciso definir `terminal.cwd` no Hermes e colocar um `.ai-memory.toml` nesse diretório. O marcador sozinho não bastou.
- **As chamadas via MCP precisam informar `workspace` e `project`** em toda consulta, porque o Hermes não envia identificação de sessão.
- **A skill do ai-memory manda gravar páginas só quando o usuário pede explicitamente.** Um mordomo precisa lembrar por conta própria, então a Sexta-feira terá de trazer a sua própria skill de memória.
- **A consolidação automática com LLM vem desligada.** As sessões capturadas viram apenas páginas de sessão; nada é extraído delas para páginas duráveis sem um provedor de LLM configurado no ai-memory.
- **Os diretórios do wiki têm nomes de UUID**, não os nomes do workspace e do projeto. As páginas em si são Markdown legível.
- **A instalação do Hermes exige Python 3.14** e não aceita `pip install` comum; funcionou com `uv sync --frozen --extra mcp`.

## O que não foi testado

1. **Turno com um modelo real.** Não havia chave de LLM no ambiente. O LLM de mentira (`stub_llm.py`) sempre pede a mesma consulta, então o teste prova o encanamento, e não se um modelo real decide consultar e gravar memória na hora certa.
2. **Busca semântica em pt-BR**, pelo bloqueio de rede descrito acima.
3. **Servidor remoto.** Tudo rodou na mesma máquina; o cenário da VPS com clientes externos não foi exercitado.
4. **Gateway do Telegram.** Só o modo `hermes chat` foi usado.

## Como reproduzir

Os scripts esperam a variável `SPIKE` apontando para um diretório com `bin/ai-memory` (release extraída) e `data/` (criado por `ai-memory init`), com `ai-memory serve --transport http` rodando.

- `mcpc.py`: cliente MCP mínimo, pela mesma ponte stdio que o Hermes usa.
- `teste_pessoal.py --gravar`: grava as 12 memórias e roda as 17 consultas.
- `stub_llm.py <arquivo-de-log>`: LLM de mentira na porta 18080, compatível com a API de chat da OpenAI.

Configuração do Hermes usada: os blocos impressos por `ai-memory install-hooks --agent hermes` e `ai-memory install-mcp --client hermes` colados no `config.yaml`, `hooks_auto_accept: true`, `ai-memory install-skills --scope global --agent hermes`, e `model.provider: custom` apontando para o stub.
