import { useState } from 'react'
import { Check, LockKeyhole } from 'lucide-react'
import { ru } from '@/i18n/ru'
import { startupExamples } from './data'
import { Switch } from '@/ui/switch'
import {
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/ui/table'
import { ConfirmAction } from '@/components/confirm-action'

export function StartupScreen() {
  const [enabled, setEnabled] = useState(
    startupExamples.filter((row) => row.enabled).map((row) => row.id),
  )
  const [completed, setCompleted] = useState(false)
  return (
    <>
      {completed && (
        <p className="notice" role="status">
          <Check aria-hidden="true" />
          {ru.exampleNoChanges}
        </p>
      )}
      <div className="table-panel">
        <Table>
          <TableCaption className="sr-only">
            {ru.startup} · {ru.exampleData}
          </TableCaption>
          <TableHeader>
            <TableRow>
              <TableHead>{ru.apps}</TableHead>
              <TableHead>{ru.source}</TableHead>
              <TableHead>{ru.status}</TableHead>
              <TableHead className="w-20">
                <span className="sr-only">{ru.startup}</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {startupExamples.map((row) => (
              <TableRow key={row.id}>
                <TableCell>
                  <span className="row-title">{row.name}</span>
                  <span className="row-detail">{row.publisher}</span>
                </TableCell>
                <TableCell className="text-muted-foreground">
                  {row.source}
                </TableCell>
                <TableCell>
                  {row.protected ? (
                    <span className="inline-flex items-center gap-2 text-muted-foreground">
                      <LockKeyhole className="size-3.5" aria-hidden="true" />
                      {ru.readOnly}
                    </span>
                  ) : enabled.includes(row.id) ? (
                    ru.enabled
                  ) : (
                    ru.disabled
                  )}
                </TableCell>
                <TableCell>
                  {row.protected ? (
                    <Switch
                      checked={enabled.includes(row.id)}
                      disabled
                      aria-label={row.name}
                    />
                  ) : (
                    <ConfirmAction
                      title={ru.startupConfirm}
                      description={ru.startupWarning}
                      trigger={
                        <Switch
                          checked={enabled.includes(row.id)}
                          aria-label={row.name}
                        />
                      }
                      onConfirm={() => {
                        setEnabled((previous) =>
                          previous.includes(row.id)
                            ? previous.filter((id) => id !== row.id)
                            : [...previous, row.id],
                        )
                        setCompleted(true)
                      }}
                    >
                      <p className="text-sm font-medium">
                        {row.name} →{' '}
                        {enabled.includes(row.id) ? ru.disabled : ru.enabled}
                      </p>
                    </ConfirmAction>
                  )}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
      <p className="supporting-text">{ru.startupReadonly}</p>
    </>
  )
}
