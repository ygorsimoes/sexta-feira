# Política de Segurança

## Versões suportadas

Apenas a série mais recente publicada a partir da branch `main` (`0.x`) recebe atualizações e correções de segurança.

| Versão | Suporte |
|---|---|
| `0.x` (`main`) | Ativo |

## Como reportar uma vulnerabilidade

**Não abra uma issue pública** para falhas de segurança, vazamento de credenciais ou brechas que possam expor dados de usuários em máquinas locais ou VPS.

Use um dos canais privados abaixo, em ordem de preferência:

1. **GitHub Private Vulnerability Reporting** (preferencial): acesse a aba **Security** -> **Advisories** -> **Report a vulnerability** em `https://github.com/ygorsimoes/sexta-feira/security/advisories/new`.
2. **E-mail do mantenedor**: envie os detalhes para `contato.ygorsimoes@gmail.com` com o assunto `[SECURITY] sexta-feira`.

### O que incluir no relato

- Descrição clara do problema e do impacto potencial (por exemplo: escrita fora do destino configurado, permissão incorreta de arquivo sensível, injeção de comando ou vazamento de segredo).
- Passos reproduzíveis, arquivos de configuração de exemplo ou trecho de código afetado.
- Versão da CLI (`sexta-feira --version`) ou commit da branch `main`.

### Prazos e divulgação coordenada

- Confirmação de recebimento em até **72 horas**.
- Avaliação de severidade e definição do plano de correção após reprodução do problema.
- Pedimos que não haja divulgação pública até que a correção seja publicada ou que seja combinado um prazo de divulgação responsável com o mantenedor.
