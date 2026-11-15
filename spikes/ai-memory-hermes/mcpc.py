"""Cliente MCP mínimo: fala com o ai-memory pela mesma ponte stdio que o Hermes usa."""
import asyncio, json, os, sys
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

SPIKE = os.environ["SPIKE"]
PARAMS = StdioServerParameters(
    command=f"{SPIKE}/bin/ai-memory",
    args=["mcp-bridge", "--no-session-id", "--server-url", "http://127.0.0.1:49374/mcp", "--data-dir", f"{SPIKE}/data"],
)

async def run(fn):
    async with stdio_client(PARAMS) as (r, w):
        async with ClientSession(r, w) as s:
            await s.initialize()
            return await fn(s)

def text(res):
    return "\n".join(c.text for c in res.content if hasattr(c, "text"))

if __name__ == "__main__":
    async def schemas(s):
        tools = (await s.list_tools()).tools
        for t in tools:
            if t.name in sys.argv[1:]:
                print("==", t.name);  print(json.dumps(t.input_schema, indent=1))
    asyncio.run(run(schemas))
