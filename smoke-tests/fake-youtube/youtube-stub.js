// The fake YouTube Data API the smoke suite points the daemon at (see
// catalogue.json and README.md). Started by scripts/run-smoke-tests.sh on
// SMOKE_YOUTUBE_STUB_PORT.
import { createServer } from 'node:http'

const PORT = Number(process.env.SMOKE_YOUTUBE_STUB_PORT ?? 18326)

createServer((request, response) => {
  response.writeHead(404).end()
}).listen(PORT, () => console.log(`YouTube stub listening on ${PORT}`))
