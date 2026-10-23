import { useState } from 'react'
import { HardDrive, Info, Monitor } from 'lucide-react'
import { Button } from '@/ui/button'
import { ru } from '@/i18n/ru'

export function App() {
  const [showAbout, setShowAbout] = useState(false)

  return (
    <div className="mx-auto flex min-h-svh max-w-6xl flex-col px-6 sm:px-10">
      <header className="flex flex-wrap items-center justify-between gap-4 border-b py-5">
        <span className="flex items-center gap-2.5 font-semibold">
          <Monitor className="size-5 text-primary" aria-hidden="true" />
          {ru.appName}
        </span>
        <span className="text-sm text-muted-foreground">{ru.preview}</span>
      </header>
      <main className="flex-1 py-8 sm:py-10">
        <h1 className="text-2xl font-semibold tracking-tight">{ru.overview}</h1>
        <section
          aria-labelledby="empty-title"
          className="mt-6 rounded-xl border bg-card px-6 py-12 sm:px-10"
        >
          <HardDrive
            className="mb-5 size-9 text-muted-foreground"
            aria-hidden="true"
          />
          <h2 id="empty-title" className="text-lg font-semibold">
            {ru.emptyTitle}
          </h2>
          <p className="mt-2 max-w-lg text-sm leading-6 text-muted-foreground">
            {ru.emptyDescription}
          </p>
          <Button
            className="mt-6"
            variant="outline"
            aria-expanded={showAbout}
            aria-controls="about-project"
            onClick={() => setShowAbout(!showAbout)}
          >
            <Info aria-hidden="true" />
            {ru.about}
          </Button>
          <div id="about-project" hidden={!showAbout}>
            <p className="mt-6 max-w-lg text-sm leading-6">
              {ru.aboutDescription}
            </p>
          </div>
        </section>
      </main>
      <footer className="flex flex-wrap justify-between gap-3 border-t py-5 text-xs leading-5 text-muted-foreground">
        <span>{ru.noChanges}</span>
        <span>v{__APP_VERSION__}</span>
      </footer>
    </div>
  )
}
