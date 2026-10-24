import type { ReactElement, ReactNode } from 'react'
import { ShieldCheck } from 'lucide-react'
import { ru } from '@/i18n/ru'
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

export function ConfirmAction({
  trigger,
  title,
  description,
  children,
  onConfirm,
}: {
  trigger: ReactElement
  title: string
  description: string
  children?: ReactNode
  onConfirm: () => void
}) {
  return (
    <AlertDialog>
      <AlertDialogTrigger asChild>{trigger}</AlertDialogTrigger>
      <AlertDialogContent className="confirm-dialog">
        <AlertDialogHeader>
          <ShieldCheck className="mb-2 size-6" aria-hidden="true" />
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>{description}</AlertDialogDescription>
        </AlertDialogHeader>
        {children}
        <p className="notice text-xs">{ru.previewWarning}</p>
        <AlertDialogFooter>
          <AlertDialogCancel>{ru.cancel}</AlertDialogCancel>
          <AlertDialogAction onClick={onConfirm}>
            {ru.confirmExample}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
