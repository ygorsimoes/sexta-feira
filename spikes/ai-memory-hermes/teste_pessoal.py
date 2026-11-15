"""Grava memórias pessoais FICTÍCIAS em pt-BR e mede a recuperação."""
import asyncio, json, sys
from mcpc import run, text

WS, PROJ = "sexta-feira", "pessoal"
PAGINAS = [
 ("pessoas/marina.md", "pessoa", ["marina"], "# Marina (esposa)\n\nMarina é a esposa do senhor. Aniversário em 14 de março. Trabalha como arquiteta. É alérgica a camarão.\n"),
 ("pessoas/dr-paulo.md", "pessoa", ["dr paulo"], "# Dr. Paulo (cardiologista)\n\nCardiologista do senhor. Consultório na Av. Paulista. Retorno a cada seis meses. Telefone da secretária: (11) 5555-0100.\n"),
 ("pessoas/rafael.md", "pessoa", ["rafael"], "# Rafael (sócio)\n\nSócio na empresa. Prefere falar por telefone a trocar e-mails. Viaja toda segunda-feira.\n"),
 ("preferencias/agenda.md", "preferencia", [], "# Regras de agenda\n\n- Nunca marcar reunião antes das 10h.\n- Sexta-feira à tarde fica livre, sem compromissos.\n- Reuniões têm no máximo 45 minutos.\n"),
 ("preferencias/comida.md", "preferencia", [], "# Preferências de comida\n\nO senhor não come carne vermelha. Restaurante favorito: Cantina do Bixiga. Café sem açúcar.\n"),
 ("preferencias/viagem.md", "preferencia", [], "# Preferências de viagem\n\nAssento no corredor. Prefere voos pela manhã. Hotel sempre com academia. Programa de milhagem: Smiles.\n"),
 ("rotinas/manha.md", "rotina", [], "# Rotina matinal\n\nAcorda às 6h30. Academia às 7h, exceto quarta. Briefing do dia deve chegar às 8h no Telegram.\n"),
 ("rotinas/financas.md", "rotina", [], "# Rotina financeira\n\nFatura do cartão vence no dia 10. Aluguel pago no dia 5. Revisão de gastos todo domingo à noite.\n"),
 ("casa/wifi-e-servicos.md", "casa", [], "# Casa: serviços\n\nDiarista vem às terças e quintas. Provedor de internet: Vivo Fibra. O portão da garagem fecha sozinho às 23h.\n"),
 ("saude/medicamentos.md", "saude", [], "# Medicamentos\n\nLosartana 50 mg toda manhã. Vitamina D aos domingos. Alergia a dipirona.\n"),
 ("projetos/sexta-feira.md", "projeto", ["sexta-feira"], "# Projeto Sexta-feira\n\nPlugin agnóstico de harness, motor em Rust na VPS. Primeiro harness: Hermes.\n"),
 ("datas/importantes.md", "data", [], "# Datas importantes\n\nAniversário de casamento: 22 de setembro. Aniversário da mãe (dona Lúcia): 3 de julho.\n"),
]
# (consulta, caminho esperado, tipo)
CONSULTAS = [
 ("Marina", "pessoas/marina.md", "termo exato"),
 ("cardiologista", "pessoas/dr-paulo.md", "termo exato"),
 ("reunião", "preferencias/agenda.md", "termo exato"),
 ("reuniao", "preferencias/agenda.md", "sem acento"),
 ("alergico", "pessoas/marina.md|saude/medicamentos.md", "sem acento + flexão (alérgica/alergia)"),
 ("reuniões", "preferencias/agenda.md", "plural (página tem as duas formas)"),
 ("medicamento", "saude/medicamentos.md", "singular x plural"),
 ("esposa aniversário", "pessoas/marina.md", "dois termos"),
 ("qual o nome da minha esposa", "pessoas/marina.md", "pergunta natural"),
 ("que horas posso marcar reunião de manhã", "preferencias/agenda.md", "pergunta natural"),
 ("mulher", "pessoas/marina.md", "sinônimo (esposa)"),
 ("médico do coração", "pessoas/dr-paulo.md", "paráfrase (cardiologista)"),
 ("remédios", "saude/medicamentos.md", "sinônimo (medicamentos)"),
 ("o que não posso servir no jantar para a Marina", "pessoas/marina.md", "pergunta com inferência"),
 ("quando vence o cartão", "rotinas/financas.md", "pergunta natural"),
 ("voo", "preferencias/viagem.md", "singular x plural (voos)"),
 ("faxineira", "casa/wifi-e-servicos.md", "sinônimo (diarista)"),
]

async def main(s):
    if "--gravar" in sys.argv:
        for path, kind, ents, body in PAGINAS:
            r = await s.call_tool("memory_write_page", {"workspace": WS, "project": PROJ, "path": path, "kind": kind, "entities": ents, "body": body, "pinned": True})
            print("gravou", path, "ERRO" if r.is_error else "ok", "" if not r.is_error else text(r)[:300])
    ok = 0
    for q, esperado, tipo in CONSULTAS:
        r = await s.call_tool("memory_query", {"workspace": WS, "project": PROJ, "query": q, "limit": 3})
        raw = text(r)
        try:
            d = json.loads(raw); hits = [h.get("path") for h in d.get("hits", d.get("results", []))]
        except Exception:
            hits = ["<não-JSON> " + raw[:200]]
        achou = any(e in hits[:3] for e in esperado.split("|"))
        ok += achou
        print(f"{'✓' if achou else '✗'} [{tipo}] {q!r} -> {hits[:3]}")
    print(f"\n{ok}/{len(CONSULTAS)} consultas acharam a página esperada no top 3")

asyncio.run(run(main))
