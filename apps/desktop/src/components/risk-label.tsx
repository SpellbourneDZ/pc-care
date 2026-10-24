import { CircleAlert, LockKeyhole, ShieldCheck } from 'lucide-react'
import { ru } from '@/i18n/ru'
import { Badge } from '@/ui/badge'
import type { Risk } from '@/preview/data'
export function RiskLabel({ risk }: { risk: Risk }) {
  const Icon =
    risk === 'safe'
      ? ShieldCheck
      : risk === 'review'
        ? CircleAlert
        : LockKeyhole
  return (
    <Badge variant="outline" className="risk-label">
      <Icon aria-hidden="true" />
      {ru.risks[risk]}
    </Badge>
  )
}
