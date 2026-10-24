# PC Care

Бесплатная локальная утилита для Windows 10/11 x64. Foundation, типизированный API и защитная политика находятся на ревью. SPE-56 добавляет монохромный интерактивный прототип четырёх модулей с явно помеченными примерами. Настоящее сканирование, очистка, автозагрузка и деинсталляция ещё не реализованы.

## Разработка

- [Инструкция для Windows](docs/README.windows.md)
- [Архитектура и постоянные инструкции](AGENTS.md)
- [Результат SPE-55 и проверки](docs/SPE-55-review.md)
- [Контракт API SPE-58 и ограничения симуляции](docs/SPE-58-api.md)
- [Защитная политика SPE-59 и границы допуска](docs/SPE-59-policy.md)
- [Desktop UX, tokens и прототип SPE-56](docs/SPE-56-design-system.md)
- [План и требования в Linear](https://linear.app/splbrn/project/pc-care-windows-cleaner-open-source-887c4dc0f0c3)

```powershell
pnpm install --frozen-lockfile
pnpm dev
```

Предварительно нужны Node 24, pnpm 10.34.8, Rust и Microsoft C++ Build Tools + Windows SDK; подробности в Windows-инструкции.

## Структура

```text
apps/desktop/
  src/                 React, локализация, UI
    ui/                локальные shadcn-компоненты
  src-tauri/
    src/               Tauri entrypoints
    domain/            предметная логика без Windows API
    win/adapters/      интеграции Windows
docs/                  Windows setup, решения, QA
.github/               базовая проверка Windows и шаблон PR
```

Приложение работает без облачного backend, рекламы и телеметрии. В текущей версии нет системных операций и команд удаления.

Исходный код: [SpellbourneDZ/pc-care](https://github.com/SpellbourneDZ/pc-care). Лицензия — [MIT](LICENSE); уведомления о заимствованных компонентах — [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). Установщик и релиз ещё не опубликованы.
