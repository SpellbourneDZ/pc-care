import { useEffect, useState } from 'react'
export type Theme = 'system' | 'light' | 'dark'
export function useTheme() {
  const [theme, setTheme] = useState<Theme>('system')
  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const apply = () =>
      document.documentElement.classList.toggle(
        'dark',
        theme === 'dark' || (theme === 'system' && media.matches),
      )
    apply()
    media.addEventListener('change', apply)
    return () => media.removeEventListener('change', apply)
  }, [theme])
  return [theme, setTheme] as const
}
