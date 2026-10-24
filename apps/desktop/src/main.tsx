import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { setNonce } from 'get-nonce'
import { App } from './app'
import './index.css'

// Tauri replaces this token per response and adds the nonce to style-src.
const styleNonce = document.querySelector<HTMLMetaElement>(
  'meta[name="pc-care-style-nonce"]',
)?.content
if (styleNonce && /^\d{1,20}$/.test(styleNonce)) setNonce(styleNonce)

const root = document.getElementById('root')
if (!root) throw new Error('Application root is missing')

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
