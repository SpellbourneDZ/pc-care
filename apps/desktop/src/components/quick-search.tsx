import { useEffect, useRef, useState } from 'react'
import {
  AppWindow,
  ArrowUpFromLine,
  BrushCleaning,
  HardDrive,
  History,
  House,
  Search,
  Settings,
} from 'lucide-react'
import { ru } from '@/i18n/ru'
import type { PageId } from '@/preview/data'
import { Button } from '@/ui/button'
import {
  Command,
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/ui/command'

export function QuickSearch({
  onNavigate,
  onSettings,
}: {
  onNavigate: (page: PageId) => void
  onSettings: () => void
}) {
  const [open, setOpen] = useState(false)
  const trigger = useRef<HTMLButtonElement>(null)
  useEffect(() => {
    const handle = (event: KeyboardEvent) => {
      if (event.ctrlKey && !event.altKey && event.key.toLowerCase() === 'k') {
        event.preventDefault()
        setOpen((previous) => !previous)
      }
    }
    window.addEventListener('keydown', handle)
    return () => window.removeEventListener('keydown', handle)
  }, [])
  const choose = (target: PageId) => {
    setOpen(false)
    onNavigate(target)
  }
  return (
    <>
      <Button
        ref={trigger}
        variant="outline"
        className="quick-search-trigger"
        onClick={() => setOpen(true)}
        aria-label={ru.quickSearch}
      >
        <Search aria-hidden="true" />
        <span>{ru.quickSearch}</span>
        <kbd>Ctrl K</kbd>
      </Button>
      <CommandDialog
        open={open}
        onOpenChange={setOpen}
        title={ru.quickSearch}
        description={ru.quickSearchDescription}
        onCloseAutoFocus={(event) => {
          event.preventDefault()
          trigger.current?.focus()
        }}
      >
        <Command>
          <CommandInput
            placeholder={ru.quickSearchPlaceholder}
            aria-label={ru.quickSearch}
          />
          <CommandList>
            <CommandEmpty>{ru.nothingFound}</CommandEmpty>
            <CommandGroup heading={ru.tools}>
              {(
                [
                  { id: 'overview', icon: House },
                  { id: 'cleanup', icon: BrushCleaning },
                  { id: 'disks', icon: HardDrive },
                  { id: 'startup', icon: ArrowUpFromLine },
                  { id: 'apps', icon: AppWindow },
                  { id: 'history', icon: History },
                ] as const
              ).map(({ id, icon: Icon }) => (
                <CommandItem
                  key={id}
                  value={ru[id]}
                  onSelect={() => choose(id)}
                >
                  <Icon aria-hidden="true" />
                  <span>{ru[id]}</span>
                </CommandItem>
              ))}
              <CommandItem
                value={ru.settings}
                onSelect={() => {
                  setOpen(false)
                  onSettings()
                }}
              >
                <Settings aria-hidden="true" />
                {ru.settings}
              </CommandItem>
            </CommandGroup>
          </CommandList>
        </Command>
      </CommandDialog>
    </>
  )
}
