import {
  AppWindow,
  ArrowUpFromLine,
  BrushCleaning,
  HardDrive,
  History,
  House,
  Info,
  Monitor,
  Settings,
} from 'lucide-react'
import { ru } from '@/i18n/ru'
import type { PageId } from '@/preview/data'
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from '@/ui/sidebar'
import {
  Dialog,
  DialogTrigger,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '@/ui/dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/ui/select'
import type { Theme } from '@/hooks/use-theme'
import type { ReactNode } from 'react'
import { routeHash } from '@/lib/routes'
import { DialogClose } from '@/ui/dialog'
import { Button } from '@/ui/button'

const navigation = [
  { id: 'overview', icon: House },
  { id: 'cleanup', icon: BrushCleaning },
  { id: 'disks', icon: HardDrive },
  { id: 'startup', icon: ArrowUpFromLine },
  { id: 'apps', icon: AppWindow },
] as const
export function ThemeSelect({
  theme,
  onThemeChange,
  disabled = false,
}: {
  theme: Theme
  onThemeChange: (theme: Theme) => void
  disabled?: boolean
}) {
  return (
    <Select
      value={theme}
      disabled={disabled}
      onValueChange={(value) => {
        if (value === 'system' || value === 'light' || value === 'dark')
          onThemeChange(value)
      }}
    >
      <SelectTrigger aria-label={ru.theme}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {Object.entries(ru.themes).map(([value, label]) => (
          <SelectItem key={value} value={value}>
            {label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
export function AppNavigation({
  page,
  onNavigate,
  theme,
  onThemeChange,
  preview = false,
  settingsContent,
  settingsOpen,
  onSettingsOpen,
  onPreview,
}: {
  page: PageId
  onNavigate: (page: PageId) => void
  theme: Theme
  onThemeChange: (theme: Theme) => void
  preview?: boolean
  settingsContent?: ReactNode
  settingsOpen: boolean
  onSettingsOpen: (open: boolean) => void
  onPreview: () => void
}) {
  const { setOpenMobile } = useSidebar()
  const navigate = (target: PageId) => {
    onNavigate(target)
    setOpenMobile(false)
  }
  return (
    <Sidebar collapsible="icon" className="app-navigation">
      <SidebarHeader>
        <div className="brand">
          <Monitor aria-hidden="true" />
          <span>PC Care</span>
        </div>
      </SidebarHeader>
      <SidebarContent>
        <SidebarGroup>
          <nav aria-label={ru.navigation}>
            <SidebarMenu>
              {navigation.map(({ id, icon: Icon }) => (
                <SidebarMenuItem key={id}>
                  <SidebarMenuButton
                    tooltip={ru[id]}
                    aria-label={ru[id]}
                    isActive={page === id}
                    aria-current={page === id ? 'page' : undefined}
                    asChild
                  >
                    <a
                      href={routeHash(id, preview)}
                      onClick={(event) => {
                        event.preventDefault()
                        navigate(id)
                      }}
                    >
                      <Icon aria-hidden="true" />
                      <span>{ru[id]}</span>
                    </a>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </nav>
        </SidebarGroup>
      </SidebarContent>
      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              tooltip={ru.history}
              aria-label={ru.history}
              isActive={page === 'history'}
              aria-current={page === 'history' ? 'page' : undefined}
              asChild
            >
              <a
                href={routeHash('history', preview)}
                onClick={(event) => {
                  event.preventDefault()
                  navigate('history')
                }}
              >
                <History aria-hidden="true" />
                <span>{ru.history}</span>
              </a>
            </SidebarMenuButton>
          </SidebarMenuItem>
          <SidebarMenuItem>
            <Dialog open={settingsOpen} onOpenChange={onSettingsOpen}>
              <DialogTrigger asChild>
                <SidebarMenuButton
                  tooltip={ru.settings}
                  aria-label={ru.settings}
                >
                  <Settings aria-hidden="true" />
                  <span>{ru.settings}</span>
                </SidebarMenuButton>
              </DialogTrigger>
              <DialogContent className="detail-dialog">
                <DialogHeader>
                  <DialogTitle>{ru.settings}</DialogTitle>
                  <DialogDescription>
                    {preview ? ru.settingsPreview : ru.settingsLocal}
                  </DialogDescription>
                </DialogHeader>
                {settingsContent ?? (
                  <>
                    <div className="settings-field">
                      <span>{ru.theme}</span>
                      <ThemeSelect
                        theme={theme}
                        onThemeChange={onThemeChange}
                      />
                    </div>
                    <div className="settings-field">
                      <span>{ru.language}</span>
                      <Select value="ru" disabled>
                        <SelectTrigger aria-label={ru.language}>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="ru">{ru.russian}</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                  </>
                )}
              </DialogContent>
            </Dialog>
          </SidebarMenuItem>
          <SidebarMenuItem>
            <Dialog>
              <DialogTrigger asChild>
                <SidebarMenuButton tooltip={ru.about} aria-label={ru.about}>
                  <Info aria-hidden="true" />
                  <span>{ru.about}</span>
                </SidebarMenuButton>
              </DialogTrigger>
              <DialogContent className="detail-dialog">
                <DialogHeader>
                  <Monitor className="mb-2 size-7" aria-hidden="true" />
                  <DialogTitle>{ru.aboutTitle}</DialogTitle>
                  <DialogDescription>{ru.license}</DialogDescription>
                </DialogHeader>
                <p className="text-sm leading-6">{ru.aboutDescription}</p>
                <p className="supporting-text">{ru.noChanges}</p>
                <DialogClose asChild>
                  <Button variant="outline" onClick={onPreview}>
                    {ru.openPrototype}
                  </Button>
                </DialogClose>
                <span className="text-xs text-muted-foreground">
                  v{__APP_VERSION__}
                </span>
              </DialogContent>
            </Dialog>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>
    </Sidebar>
  )
}
