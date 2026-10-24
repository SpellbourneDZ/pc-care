import { useState } from 'react'
import { Check, Trash2 } from 'lucide-react'
import { ru } from '@/i18n/ru'
import { cleanupExamples, formatBytes } from './data'
import { Button } from '@/ui/button'
import { Checkbox } from '@/ui/checkbox'
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
import { RiskLabel } from '@/components/risk-label'

export function CleanupScreen() {
  const [selected, setSelected] = useState(['temp', 'cache'])
  const [completed, setCompleted] = useState(false)
  const selectable = cleanupExamples.filter((row) => row.risk !== 'protected')
  const bulk = selectable.filter((row) => row.risk === 'safe')
  const bulkSelected = bulk.filter((row) => selected.includes(row.id)).length
  const selectedRows = selectable.filter((row) => selected.includes(row.id))
  const total = selectedRows.reduce((bytes, row) => bytes + (row.bytes ?? 0), 0)
  const all = bulkSelected === bulk.length
  return (
    <>
      {completed && (
        <div className="notice" role="status">
          <Check aria-hidden="true" />
          <div>
            <strong>{ru.exampleResult}</strong>
            <p>{ru.exampleNoChanges}</p>
          </div>
        </div>
      )}
      <div className="table-panel">
        <Table>
          <TableCaption className="sr-only">
            {ru.cleanup} · {ru.exampleData}
          </TableCaption>
          <TableHeader>
            <TableRow>
              <TableHead className="w-12">
                <Checkbox
                  aria-label={ru.allCategories}
                  checked={all ? true : bulkSelected ? 'indeterminate' : false}
                  onCheckedChange={(checked) =>
                    setSelected((previous) =>
                      checked
                        ? [
                            ...new Set([
                              ...previous,
                              ...bulk.map((row) => row.id),
                            ]),
                          ]
                        : previous.filter(
                            (id) => !bulk.some((row) => row.id === id),
                          ),
                    )
                  }
                />
              </TableHead>
              <TableHead>{ru.category}</TableHead>
              <TableHead>{ru.risk}</TableHead>
              <TableHead className="text-right">{ru.size}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {cleanupExamples.map((row) => (
              <TableRow
                key={row.id}
                data-state={selected.includes(row.id) ? 'selected' : undefined}
              >
                <TableCell>
                  <Checkbox
                    aria-label={`${ru.selected}: ${row.name}`}
                    checked={selected.includes(row.id)}
                    disabled={row.risk === 'protected'}
                    onCheckedChange={(checked) =>
                      setSelected((previous) =>
                        checked
                          ? [...previous, row.id]
                          : previous.filter((id) => id !== row.id),
                      )
                    }
                  />
                </TableCell>
                <TableCell>
                  <span className="row-title">{row.name}</span>
                  <span className="row-detail">{row.path}</span>
                </TableCell>
                <TableCell>
                  <RiskLabel risk={row.risk} />
                </TableCell>
                <TableCell className="numeric text-right">
                  {row.bytes === null ? '—' : formatBytes(row.bytes)}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
      <p className="supporting-text">{ru.protectedDescription}</p>
      <div className="action-bar">
        <div>
          <strong>
            {ru.selected}: {selected.length}
          </strong>
          <span className="numeric ml-3">{formatBytes(total)}</span>
        </div>
        <ConfirmAction
          title={ru.cleanupConfirm}
          description={ru.recycleWarning}
          onConfirm={() => setCompleted(true)}
          trigger={
            <Button disabled={!selected.length}>
              <Trash2 aria-hidden="true" />
              {ru.cleanupAction}
            </Button>
          }
        >
          <ul className="confirmation-list">
            {selectedRows.map((row) => (
              <li key={row.id}>
                <span>{row.name}</span>
                <span className="numeric">{formatBytes(row.bytes ?? 0)}</span>
              </li>
            ))}
          </ul>
        </ConfirmAction>
      </div>
    </>
  )
}
