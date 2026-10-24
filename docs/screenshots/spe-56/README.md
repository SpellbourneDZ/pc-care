# SPE-56 — скриншоты для ревью

Данные во всех модулях — явно помеченные примеры. Снято в Edge на Windows; compact-125/150 используют browser DPR emulation, не изменение настроек ОС.

| Экран         | Light 1280×720                     | Dark 1920×1080                    | Compact 720×520 / DPR 1.25          | Compact / DPR 1.5                   |
| ------------- | ---------------------------------- | --------------------------------- | ----------------------------------- | ----------------------------------- |
| Очистка       | [PNG](light-1280-cleanup.png)      | [PNG](dark-1920-cleanup.png)      | [PNG](compact-125-cleanup.png)      | [PNG](compact-150-cleanup.png)      |
| Диски         | [PNG](light-1280-disks.png)        | [PNG](dark-1920-disks.png)        | [PNG](compact-125-disks.png)        | [PNG](compact-150-disks.png)        |
| Автозагрузка  | [PNG](light-1280-startup.png)      | [PNG](dark-1920-startup.png)      | [PNG](compact-125-startup.png)      | [PNG](compact-150-startup.png)      |
| Программы     | [PNG](light-1280-apps.png)         | [PNG](dark-1920-apps.png)         | [PNG](compact-125-apps.png)         | [PNG](compact-150-apps.png)         |
| Подтверждение | [PNG](light-1280-confirmation.png) | [PNG](dark-1920-confirmation.png) | [PNG](compact-125-confirmation.png) | [PNG](compact-150-confirmation.png) |

[Обзор](light-1280-overview.png), [ошибка](light-1280-error.png), [частичный результат](light-1280-partial.png), [анализ](light-1280-loading.png), [unsupported](light-1280-unsupported.png), [offline](light-1280-offline.png).

[Настоящий release WebView2](native-webview2.png), после проверки modal focus/confirmations и CSP. Browser images сами по себе не доказывают native поведение; фактические проверки перечислены в [отчёте](../../SPE-56-design-system.md).
