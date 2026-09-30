import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import './index.css'
import App from './App'

// A failed fetch shows its error at once; the next poll retries anyway.
const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })

const container = document.getElementById('root')
if (!container) {
  throw new Error('missing #root element')
}

createRoot(container).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </StrictMode>,
)
