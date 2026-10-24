import { useState } from 'react'
import {
  Check,
  Folder,
  HardDrive,
  LayoutGrid,
  ListTree,
  Trash2,
} from 'lucide-react'
import { ru } from '@/i18n/ru'
import { diskExamples, formatBytes } from './data'
import { Button } from '@/ui/button'
import { Card, CardContent, CardHeader } from '@/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/ui/tabs'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/ui/select'
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbList,
  BreadcrumbPage,
} from '@/ui/breadcrumb'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/ui/table'
import { ConfirmAction } from '@/components/confirm-action'
import { RiskLabel } from '@/components/risk-label'

export function DisksScreen() {
  const [selectedId, setSelectedId] = useState('videos')
  const [completed, setCompleted] = useState(false)
  const selected =
    diskExamples.find((row) => row.id === selectedId) ?? diskExamples[0]!
  return (
    <>
      {completed && (
        <p className="notice" role="status">
          <Check aria-hidden="true" />
          {ru.exampleNoChanges}
        </p>
      )}
      <div className="disk-layout">
        <div className="min-w-0">
          <Tabs defaultValue="map">
            <div className="view-toolbar">
              <Select defaultValue="example">
                <SelectTrigger
                  aria-label={ru.disks}
                  className="w-auto min-w-52"
                >
                  <HardDrive aria-hidden="true" />
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="example">{ru.diskExample}</SelectItem>
                </SelectContent>
              </Select>
              <TabsList aria-label={ru.disks}>
                <TabsTrigger value="map">
                  <LayoutGrid aria-hidden="true" />
                  {ru.map}
                </TabsTrigger>
                <TabsTrigger value="tree">
                  <ListTree aria-hidden="true" />
                  {ru.tree}
                </TabsTrigger>
              </TabsList>
            </div>
            <Breadcrumb className="mb-4">
              <BreadcrumbList>
                <BreadcrumbItem>
                  <BreadcrumbPage>{ru.diskExample}</BreadcrumbPage>
                </BreadcrumbItem>
              </BreadcrumbList>
            </Breadcrumb>
            <TabsContent value="map">
              <div className="disk-map" aria-label={ru.map}>
                {diskExamples.map((row) => (
                  <button
                    key={row.id}
                    className={`map-tile map-${row.id}`}
                    data-selected={selectedId === row.id}
                    aria-pressed={selectedId === row.id}
                    onClick={() => setSelectedId(row.id)}
                  >
                    <Folder aria-hidden="true" />
                    <span>{row.name}</span>
                    <strong className="numeric">
                      {formatBytes(row.bytes)}
                    </strong>
                  </button>
                ))}
              </div>
            </TabsContent>
            <TabsContent value="tree">
              <div className="table-panel">
                <Table aria-label={ru.tree}>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{ru.fileName}</TableHead>
                      <TableHead className="text-right">
                        {ru.logicalSize}
                      </TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {diskExamples.map((row) => (
                      <TableRow
                        key={row.id}
                        data-state={
                          selectedId === row.id ? 'selected' : undefined
                        }
                      >
                        <TableCell>
                          <Button
                            variant="ghost"
                            onClick={() => setSelectedId(row.id)}
                          >
                            <Folder aria-hidden="true" />
                            {row.name}
                          </Button>
                        </TableCell>
                        <TableCell className="numeric text-right">
                          {formatBytes(row.bytes)}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            </TabsContent>
          </Tabs>
        </div>
        <Card className="inspector">
          <CardHeader>
            <h2 className="text-sm font-medium text-muted-foreground">
              {ru.inspector}
            </h2>
            <h3 className="text-lg font-semibold break-words">
              {selected.name}
            </h3>
            <RiskLabel risk={selected.risk} />
          </CardHeader>
          <CardContent>
            <dl className="details-list">
              <dt>{ru.path}</dt>
              <dd className="break-words">{selected.path}</dd>
              <dt>{ru.logicalSize}</dt>
              <dd className="numeric">{formatBytes(selected.bytes)}</dd>
              <dt>{ru.sizeOnDisk}</dt>
              <dd>{ru.unavailable}</dd>
            </dl>
            {selected.risk === 'protected' && (
              <p className="supporting-text">{ru.protectedDescription}</p>
            )}
            <ConfirmAction
              title={ru.recycleConfirm}
              description={ru.recycleWarning}
              onConfirm={() => setCompleted(true)}
              trigger={
                <Button
                  className="mt-5 w-full"
                  variant="outline"
                  disabled={selected.risk === 'protected'}
                >
                  <Trash2 aria-hidden="true" />
                  {ru.toRecycle}
                </Button>
              }
            >
              <p className="break-words text-sm">{selected.path}</p>
            </ConfirmAction>
          </CardContent>
        </Card>
      </div>
    </>
  )
}
