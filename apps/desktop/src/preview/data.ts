import { ru } from '@/i18n/ru'

export type ModuleId = 'cleanup' | 'disks' | 'startup' | 'apps'
export type PageId = 'overview' | ModuleId | 'history'
export type PreviewState = keyof typeof ru.states
export type Risk = keyof typeof ru.risks

export const cleanupExamples: {
  id: string
  name: string
  path: string
  bytes: number | null
  risk: Risk
}[] = [
  {
    id: 'temp',
    name: ru.cleanupNames.temp,
    path: 'C:\\Users\\Пример\\AppData\\Local\\Temp',
    bytes: 1_073_741_824,
    risk: 'safe',
  },
  {
    id: 'cache',
    name: ru.cleanupNames.cache,
    path: 'C:\\Users\\Пример\\AppData\\Local\\Example\\Cache',
    bytes: 314_572_800,
    risk: 'safe',
  },
  {
    id: 'recycle',
    name: ru.cleanupNames.recycle,
    path: ru.recycleManual,
    bytes: 524_288_000,
    risk: 'review',
  },
  {
    id: 'documents',
    name: ru.cleanupNames.documents,
    path: 'C:\\Users\\Пример\\Documents',
    bytes: null,
    risk: 'protected',
  },
]
export const diskExamples: {
  id: string
  name: string
  path: string
  bytes: number
  risk: Risk
}[] = [
  {
    id: 'videos',
    name: ru.diskNames.videos,
    path: 'C:\\Users\\Пример\\Videos',
    bytes: 51_539_607_552,
    risk: 'review',
  },
  {
    id: 'apps',
    name: ru.diskNames.apps,
    path: 'C:\\Program Files',
    bytes: 17_179_869_184,
    risk: 'protected',
  },
  {
    id: 'windows',
    name: ru.diskNames.windows,
    path: 'C:\\Windows',
    bytes: 17_179_869_184,
    risk: 'protected',
  },
  {
    id: 'temp',
    name: ru.diskNames.temp,
    path: 'C:\\Users\\Пример\\AppData\\Local\\Temp',
    bytes: 17_179_869_184,
    risk: 'review',
  },
  {
    id: 'downloads',
    name: ru.diskNames.downloads,
    path: 'C:\\Users\\Пример\\Downloads',
    bytes: 17_179_869_184,
    risk: 'review',
  },
]
export const startupExamples = [
  {
    id: 'drive',
    name: ru.startupNames.drive,
    publisher: ru.examplePublisher,
    source: 'HKCU · Run',
    enabled: true,
    protected: false,
  },
  {
    id: 'notes',
    name: ru.startupNames.notes,
    publisher: ru.examplePublisher,
    source: ru.startupFolder,
    enabled: false,
    protected: false,
  },
  {
    id: 'security',
    name: ru.startupNames.security,
    publisher: ru.unknownPublisher,
    source: 'HKLM · Run',
    enabled: true,
    protected: true,
  },
]
export const appExamples = [
  {
    id: 'editor',
    name: ru.appNames.editor,
    publisher: ru.examplePublisher,
    size: '284 МБ',
    installed: '12.10.2026',
    type: 'MSI',
    version: '2.4.0',
  },
  {
    id: 'archive',
    name: ru.appNames.archive,
    publisher: ru.examplePublisher,
    size: '—',
    installed: '—',
    type: 'EXE',
    version: '1.8.2',
  },
  {
    id: 'player',
    name: ru.appNames.player,
    publisher: ru.unknownPublisher,
    size: '146 МБ',
    installed: '05.10.2026',
    type: 'MSIX',
    version: '3.1.0',
  },
]
export function formatBytes(bytes: number) {
  const units = ['Б', 'КБ', 'МБ', 'ГБ', 'ТБ']
  const index =
    bytes > 0
      ? Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)
      : 0
  return `${new Intl.NumberFormat('ru-RU', { maximumFractionDigits: 1 }).format(bytes / 1024 ** index)} ${units[index]}`
}
