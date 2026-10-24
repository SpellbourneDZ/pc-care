import { useState } from 'react'
import { ArrowUpRight, Check, Package, Search, Trash2 } from 'lucide-react'
import { ru } from '@/i18n/ru'
import { appExamples } from './data'
import { Button } from '@/ui/button'
import { Badge } from '@/ui/badge'
import { Input } from '@/ui/input'
import {
  Dialog,
  DialogTrigger,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '@/ui/dialog'
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

export function AppsScreen() {
  const [search, setSearch] = useState('')
  const [completed, setCompleted] = useState<string[]>([])
  const [leftovers, setLeftovers] = useState<string[]>([])
  const rows = appExamples.filter((row) =>
    row.name
      .toLocaleLowerCase('ru-RU')
      .includes(search.toLocaleLowerCase('ru-RU').trim()),
  )
  return (
    <>
      <div className="view-toolbar">
        <label className="search-field">
          <Search aria-hidden="true" />
          <Input
            aria-label={ru.searchApps}
            placeholder={ru.searchPlaceholder}
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
        </label>
      </div>
      <div className="table-panel">
        <Table>
          <TableCaption className="sr-only">
            {ru.apps} · {ru.exampleData}
          </TableCaption>
          <TableHeader>
            <TableRow>
              <TableHead>{ru.apps}</TableHead>
              <TableHead>{ru.installed}</TableHead>
              <TableHead className="text-right">{ru.size}</TableHead>
              <TableHead className="w-16">
                <span className="sr-only">{ru.details}</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((row) => (
              <TableRow key={row.id}>
                <TableCell>
                  <span className="row-title">
                    {row.name} <Badge variant="outline">{row.type}</Badge>
                  </span>
                  <span className="row-detail">{row.publisher}</span>
                </TableCell>
                <TableCell className="numeric text-muted-foreground">
                  {row.installed}
                </TableCell>
                <TableCell className="numeric text-right">{row.size}</TableCell>
                <TableCell>
                  <Dialog>
                    <DialogTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label={`${ru.details}: ${row.name}`}
                      >
                        <ArrowUpRight aria-hidden="true" />
                      </Button>
                    </DialogTrigger>
                    <DialogContent className="detail-dialog">
                      <DialogHeader>
                        <Package className="mb-2 size-7" aria-hidden="true" />
                        <DialogTitle>{row.name}</DialogTitle>
                        <DialogDescription>
                          {row.publisher} · {row.version} · {row.type}
                        </DialogDescription>
                      </DialogHeader>
                      <dl className="details-list">
                        <dt>{ru.installed}</dt>
                        <dd>{row.installed}</dd>
                        <dt>{ru.size}</dt>
                        <dd>{row.size === '—' ? ru.unavailable : row.size}</dd>
                      </dl>
                      {completed.includes(row.id) && (
                        <p className="notice" role="status">
                          <Check aria-hidden="true" />
                          {ru.exampleNoChanges}
                        </p>
                      )}
                      <ConfirmAction
                        title={ru.uninstallConfirm}
                        description={ru.uninstallWarning}
                        trigger={
                          <Button variant="outline">
                            <Trash2 aria-hidden="true" />
                            {ru.uninstall}
                          </Button>
                        }
                        onConfirm={() =>
                          setCompleted((previous) => [...previous, row.id])
                        }
                      >
                        <p className="text-sm font-medium">{row.name}</p>
                      </ConfirmAction>
                      {completed.includes(row.id) && (
                        <Button
                          variant="secondary"
                          onClick={() =>
                            setLeftovers((previous) => [...previous, row.id])
                          }
                        >
                          {ru.findLeftovers}
                        </Button>
                      )}
                      {leftovers.includes(row.id) && (
                        <div className="notice">
                          <div>
                            <strong>{ru.leftoversTitle}</strong>
                            <p>{ru.leftoversDescription}</p>
                          </div>
                        </div>
                      )}
                    </DialogContent>
                  </Dialog>
                </TableCell>
              </TableRow>
            ))}
            {!rows.length && (
              <TableRow>
                <TableCell
                  colSpan={4}
                  className="py-10 text-center text-muted-foreground"
                >
                  {ru.noSearchResults}
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>
    </>
  )
}
