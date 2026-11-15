"""LLM de mentira (API de chat da OpenAI) para testar o encanamento Hermes -> ai-memory sem chave.
Roteiro fixo: na mensagem do usuário, pede a ferramenta memory_query; ao receber o resultado, responde com ele."""
import json, sys, time
from http.server import BaseHTTPRequestHandler, HTTPServer

LOG = open(sys.argv[1], "a")

def log(**k):
    LOG.write(json.dumps(k, ensure_ascii=False) + "\n"); LOG.flush()

class H(BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def do_GET(self):
        body = json.dumps({"object": "list", "data": [{"id": "stub", "object": "model"}]}).encode()
        self.send_response(200); self.send_header("content-type", "application/json"); self.send_header("content-length", str(len(body))); self.end_headers(); self.wfile.write(body)
    def do_POST(self):
        req = json.loads(self.rfile.read(int(self.headers["content-length"])))
        msgs, tools = req.get("messages", []), req.get("tools") or []
        names = [t["function"]["name"] for t in tools if t.get("type") == "function"]
        mq = next((n for n in names if n.endswith("memory_query")), None)
        last = msgs[-1] if msgs else {}
        sysmsg = next((m["content"] for m in msgs if m["role"] == "system"), "")
        if not isinstance(sysmsg, str): sysmsg = json.dumps(sysmsg, ensure_ascii=False)
        log(names=names, path=self.path, stream=req.get("stream"), n_tools=len(names), memory_tools=[n for n in names if "memory" in n],
            last_role=last.get("role"), last=str(last.get("content"))[:1500],
            system_mentions_ai_memory="ai-memory" in sysmsg, system_len=len(sysmsg))
        msg = {"role": "assistant", "content": None}
        finish = "stop"
        margs = {"workspace": "sexta-feira", "project": "pessoal", "query": "esposa", "limit": 2}
        if last.get("role") == "user" and (mq or "tool_call" in names):
            fn = {"name": mq, "arguments": json.dumps(margs)} if mq else {"name": "tool_call",
                  "arguments": json.dumps({"name": "mcp__ai_memory__memory_query", "arguments": margs})}
            msg["tool_calls"] = [{"id": "call_stub_1", "type": "function", "function": fn}]
            finish = "tool_calls"
        elif last.get("role") == "tool":
            msg["content"] = "RESULTADO DA MEMÓRIA (stub): " + str(last.get("content"))[:900]
        else:
            msg["content"] = "ok (stub)"
        base = {"id": "chatcmpl-stub", "created": int(time.time()), "model": req.get("model", "stub")}
        usage = {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        if req.get("stream"):
            self.send_response(200); self.send_header("content-type", "text/event-stream"); self.end_headers()
            delta = {"role": "assistant"}
            if msg.get("content"): delta["content"] = msg["content"]
            if msg.get("tool_calls"): delta["tool_calls"] = [dict(tc, index=i) for i, tc in enumerate(msg["tool_calls"])]
            for chunk in ({**base, "object": "chat.completion.chunk", "choices": [{"index": 0, "delta": delta, "finish_reason": None}]},
                          {**base, "object": "chat.completion.chunk", "choices": [{"index": 0, "delta": {}, "finish_reason": finish}], "usage": usage}):
                self.wfile.write(f"data: {json.dumps(chunk)}\n\n".encode())
            self.wfile.write(b"data: [DONE]\n\n")
        else:
            body = json.dumps({**base, "object": "chat.completion", "choices": [{"index": 0, "message": msg, "finish_reason": finish}], "usage": usage}).encode()
            self.send_response(200); self.send_header("content-type", "application/json"); self.send_header("content-length", str(len(body))); self.end_headers(); self.wfile.write(body)

HTTPServer(("127.0.0.1", 18080), H).serve_forever()
