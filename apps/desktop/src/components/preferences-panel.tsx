import { useLayoutEffect, useRef, useState } from 'react'
import { Check, CircleAlert, LoaderCircle, Plus, Trash2 } from 'lucide-react'
import { ru } from '@/i18n/ru'
import type { Preferences } from '@/generated/Preferences'
import { preferencesErrorMessage } from '@/lib/preferences'
import { ThemeSelect } from './app-navigation'
import { Button } from '@/ui/button'
import { Input } from '@/ui/input'
import { ScrollArea } from '@/ui/scroll-area'
import {
  AlertDialog,
  AlertDialogTrigger,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogCancel,
  AlertDialogAction,
} from '@/ui/alert-dialog'

export function PreferencesPanel({
  preferences,
  pending,
  saving,
  saved,
  error,
  onReload,
  onSave,
}: {
  preferences?: Preferences
  pending: boolean
  saving: boolean
  saved: boolean
  error: unknown
  onReload: () => void
  onSave: (preferences: Preferences) => Promise<void>
}) {
  const [path, setPath] = useState('')
  const input = useRef<HTMLInputElement>(null)
  const pendingFocus = useRef(false)
  const confirmedRemoval = useRef(false)
  useLayoutEffect(() => {
    if (!saving && pendingFocus.current) {
      input.current?.focus()
      pendingFocus.current = false
    }
  }, [saving, preferences, error])
  const saveExclusions = (value: Preferences) => {
    pendingFocus.current = true
    return onSave(value)
  }
  if (pending)
    return (
      <p className="notice" role="status">
        <LoaderCircle
          className="animate-spin motion-reduce:animate-none"
          aria-hidden="true"
        />
        {ru.settingsLoading}
      </p>
    )
  const disabled = saving || !preferences
  return (
    <div className="preferences-panel">
      {Boolean(error) && (
        <div className="notice" role="alert">
          <CircleAlert aria-hidden="true" />
          <div>
            <p>{preferencesErrorMessage(error)}</p>
            <Button
              variant="outline"
              size="sm"
              className="mt-2"
              onClick={onReload}
              disabled={saving}
            >
              {ru.reloadSettings}
            </Button>
          </div>
        </div>
      )}
      {preferences && (
        <>
          <div className="settings-field">
            <span>{ru.theme}</span>
            <ThemeSelect
              theme={preferences.theme}
              disabled={disabled}
              onThemeChange={(theme) => {
                void onSave({ ...preferences, theme }).catch(() => {})
              }}
            />
          </div>
          <div className="settings-field">
            <span>{ru.language}</span>
            <span>{ru.russian}</span>
          </div>
          <section aria-labelledby="exclusions-title">
            <h3 id="exclusions-title" className="text-sm font-medium">
              {ru.exclusions}
            </h3>
            <p className="supporting-text mt-2">{ru.exclusionsDescription}</p>
            <form
              className="exclusion-form"
              onSubmit={(event) => {
                event.preventDefault()
                if (path.trim() && !disabled)
                  void saveExclusions({
                    ...preferences,
                    exclusions: [...preferences.exclusions, path.trim()],
                  })
                    .then(() => {
                      setPath('')
                    })
                    .catch(() => {})
              }}
            >
              <Input
                ref={input}
                aria-label={ru.exclusionPath}
                placeholder="C:\\…"
                value={path}
                onChange={(event) => setPath(event.target.value)}
                disabled={disabled}
                maxLength={32_000}
              />
              <Button
                type="submit"
                variant="outline"
                disabled={disabled || !path.trim()}
              >
                <Plus aria-hidden="true" />
                {ru.add}
              </Button>
            </form>
            {!preferences.exclusions.length ? (
              <p className="supporting-text">{ru.noExclusions}</p>
            ) : (
              <ScrollArea className="exclusions-scroll">
                <ul>
                  {preferences.exclusions.map((exclusion) => (
                    <li className="exclusion-row" key={exclusion}>
                      <span title={exclusion}>{exclusion}</span>
                      <AlertDialog>
                        <AlertDialogTrigger asChild>
                          <Button
                            variant="ghost"
                            size="icon"
                            disabled={disabled}
                            aria-label={`${ru.removeExclusion}: ${exclusion}`}
                          >
                            <Trash2 aria-hidden="true" />
                          </Button>
                        </AlertDialogTrigger>
                        <AlertDialogContent
                          className="confirm-dialog"
                          onCloseAutoFocus={(event) => {
                            if (confirmedRemoval.current) {
                              // The confirmed trigger can disappear before Radix restores focus.
                              event.preventDefault()
                              confirmedRemoval.current = false
                              if (!input.current?.disabled)
                                input.current?.focus()
                            }
                          }}
                        >
                          <AlertDialogHeader>
                            <AlertDialogTitle>
                              {ru.removeExclusion}
                            </AlertDialogTitle>
                            <AlertDialogDescription>
                              {ru.removeExclusionDescription}
                            </AlertDialogDescription>
                          </AlertDialogHeader>
                          <p className="break-words text-sm">{exclusion}</p>
                          <AlertDialogFooter>
                            <AlertDialogCancel>{ru.cancel}</AlertDialogCancel>
                            <AlertDialogAction
                              onClick={() => {
                                confirmedRemoval.current = true
                                void saveExclusions({
                                  ...preferences,
                                  exclusions: preferences.exclusions.filter(
                                    (value) => value !== exclusion,
                                  ),
                                }).catch(() => {})
                              }}
                            >
                              {ru.removeExclusion}
                            </AlertDialogAction>
                          </AlertDialogFooter>
                        </AlertDialogContent>
                      </AlertDialog>
                    </li>
                  ))}
                </ul>
              </ScrollArea>
            )}
          </section>
        </>
      )}
      <p className="settings-save-status" role="status">
        {saving ? (
          <>
            <LoaderCircle
              className="animate-spin motion-reduce:animate-none"
              aria-hidden="true"
            />
            {ru.settingsSaving}
          </>
        ) : saved && !error ? (
          <>
            <Check aria-hidden="true" />
            {ru.settingsSaved}
          </>
        ) : (
          ru.settingsLocal
        )}
      </p>
    </div>
  )
}
