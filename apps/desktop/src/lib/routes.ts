import { useSyncExternalStore } from 'react'
import type { PageId } from '@/preview/data'

const pages: PageId[] = [
  'overview',
  'cleanup',
  'disks',
  'startup',
  'apps',
  'history',
]
function subscribe(notify: () => void) {
  window.addEventListener('hashchange', notify)
  return () => window.removeEventListener('hashchange', notify)
}
export function parseRoute(hash: string): { page: PageId; preview: boolean } {
  const preview = hash.startsWith('#/preview/')
  const name = hash.slice(preview ? '#/preview/'.length : '#/'.length)
  return {
    page: pages.includes(name as PageId) ? (name as PageId) : 'overview',
    preview,
  }
}
export function routeHash(page: PageId, preview: boolean) {
  return `#/${preview ? 'preview/' : ''}${page}`
}
export function useRoute() {
  const hash = useSyncExternalStore(
    subscribe,
    () => window.location.hash,
    () => '#/overview',
  )
  return { ...parseRoute(hash), hash }
}
