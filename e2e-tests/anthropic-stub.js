// A stand-in Anthropic API for the end-to-end suite: answers the requests
// yarrtube makes to list the models its key can use, newest first, and its
// messages. Every daemon is started with a key, pointed here.
import { createServer } from 'node:http'

/** The key every daemon is started with. */
export const STUB_API_KEY = 'e2e-anthropic-key'

/** What every message answers. */
export const STUB_ANSWER = 'A chess video.'

export const MODELS = [
  { id: 'claude-sonnet-5-5', display_name: 'Claude Sonnet 5.5', max_input_tokens: 1000000 },
  { id: 'claude-haiku-5-5', display_name: 'Claude Haiku 5.5', max_input_tokens: 1000000 },
]

/** Starts a stub on `port` (every interface, so a container can reach it). */
export function startAnthropicStub(port) {
  const server = createServer((req, res) => {
    const { pathname } = new URL(req.url, 'http://stub')
    if (req.headers['x-api-key'] !== STUB_API_KEY) {
      return json(res, error('authentication_error', 'invalid x-api-key'), 401)
    }
    if (req.method === 'GET' && pathname === '/v1/models') {
      const data = MODELS.map((model) => ({ type: 'model', ...model }))
      return json(res, { data, has_more: false, first_id: data[0].id, last_id: data.at(-1).id })
    }
    if (req.method === 'POST' && pathname === '/v1/messages') {
      return json(res, {
        type: 'message',
        role: 'assistant',
        content: [{ type: 'text', text: STUB_ANSWER }],
        stop_reason: 'end_turn',
        usage: { input_tokens: 100, output_tokens: 10 },
      })
    }
    json(res, error('not_found_error', `no ${req.method} ${pathname}`), 404)
  })
  return new Promise((resolve) => {
    server.listen(port, '0.0.0.0', () =>
      resolve({ port, close: () => new Promise((done) => server.close(done)) }),
    )
  })
}

function error(type, message) {
  return { type: 'error', error: { type, message } }
}

function json(res, body, status = 200) {
  res.writeHead(status, { 'Content-Type': 'application/json' }).end(JSON.stringify(body))
}
