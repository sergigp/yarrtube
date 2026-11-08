// A stand-in Ollama server for the end-to-end suite: answers the requests
// yarrtube makes to check a server (version, installed models, each model's
// capabilities) and its chats. qwen3:4b can think, gemma3:4b can't.
import { createServer } from 'node:http'

export const OLLAMA_VERSION = '0.40.13'

/** What every summary chat answers. */
export const STUB_SUMMARY = 'A chess video.'

const MODELS = [
  { name: 'qwen3:4b', size: 2497293918, capabilities: ['completion', 'thinking'] },
  { name: 'gemma3:4b', size: 3338801804, capabilities: ['completion', 'vision'] },
]

/** Starts a stub on `port` (every interface, so a container can reach it). */
export function startOllamaStub(port) {
  const server = createServer((req, res) => {
    const { pathname } = new URL(req.url, 'http://stub')
    if (req.method === 'GET' && pathname === '/api/version') {
      return json(res, { version: OLLAMA_VERSION })
    }
    if (req.method === 'GET' && pathname === '/api/tags') {
      return json(res, { models: MODELS.map(({ name, size }) => ({ name, model: name, size })) })
    }
    if (req.method === 'POST' && pathname === '/api/show') {
      return readJson(req).then(({ model }) => {
        const found = MODELS.find((candidate) => candidate.name === model)
        return found
          ? json(res, { capabilities: found.capabilities })
          : res.writeHead(404).end()
      })
    }
    if (req.method === 'POST' && pathname === '/api/chat') {
      return readJson(req).then((chat) => json(res, chatReply(chat)))
    }
    res.writeHead(404).end()
  })
  return new Promise((resolve) => {
    server.listen(port, '0.0.0.0', () =>
      resolve({ port, close: () => new Promise((done) => server.close(done)) }),
    )
  })
}

/**
 * A categorization chat (it asks for a JSON `format`) always fits the
 * category; any other chat is a summary.
 */
function chatReply({ format }) {
  const content = format
    ? JSON.stringify({ reason: 'The stub says every video fits.', belongs: true })
    : STUB_SUMMARY
  return { message: { role: 'assistant', content }, prompt_eval_count: 100, done_reason: 'stop' }
}

function readJson(req) {
  return new Promise((resolve) => {
    let body = ''
    req.on('data', (chunk) => (body += chunk))
    req.on('end', () => resolve(JSON.parse(body || '{}')))
  })
}

function json(res, body) {
  res.writeHead(200, { 'Content-Type': 'application/json' }).end(JSON.stringify(body))
}
