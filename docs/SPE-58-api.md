# SPE-58 — typed operation API

## Граница

Эта задача добавляет IPC-контракт и mock-движок, а не сканирование Windows. Foundation SPE-55 остаётся в [PR #1](https://github.com/SpellbourneDZ/pc-care/pull/1); ветка SPE-58 основана на ней и не сливает её автоматически.

Единственная пользовательская IPC-команда — `operation_api`. Она принимает закрытый Rust enum ApiRequest; capability `allow-operation-api` действует только для локального окна main. AppManifest в build.rs генерирует allow/deny permissions. FS, shell, registry plugins и remote capabilities отсутствуют.

## Контракт

- `start_scan`: kind=mock, scenario=complete/adapter_error, timeout_ms=1…30000, приватный Channel<TaskProgress>. Mock разрешён только при debug_assertions; release отвечает unsupported.
- `get_task`, `cancel_task`: только backend-generated UUID задачи. Повторная отмена terminal-задачи не меняет результат.
- `get_scan_result`: UUID завершённого snapshot, offset и limit=1…500. Запрос за концом возвращает пустую страницу.
- `build_operation_plan`: UUID snapshot и непустой список уникальных candidate IDs из этого snapshot. Protected отклоняется. Для Review есть предупреждение.
- `execute_plan`: только UUID плана и explicit_confirmation. План действует 30 секунд и потребляется один раз, включая конкурентные запросы.

Все запросы и DTO используют snake_case на wire, без отдельного слоя преобразований. Unknown fields/variants и некорректные UUID отклоняет Serde. API не принимает исполняемые строки или пути к объектам.

Candidate содержит собственный UUID, mock source/source_key, display_path mock://, identity, rule_id, bytes и SafetyClass. Snapshot хранится в backend и отдаётся копиями страниц. Исполнение повторно сопоставляет target со snapshot и актуальным mock fixture; изменённые/protected элементы получают skipped с причиной.

OperationPlan и OperationResult имеют simulated=true. Даже успешная симуляция возвращает actual_reclaimed_bytes="0", без заявлений об освобождённом месте. Системные адаптеры и реальная защита файлов относятся к SPE-59/61/63.

## Типы и ошибки

Rust-модели — источник истины: domain/contracts.rs. ts-rs генерирует src/generated/*.ts без any. UUID — строки; ByteCount — десятичная строка, включая точное u64::MAX.

```powershell
pnpm types:generate
pnpm types:check
```

Генерируемые файлы коммитятся, не форматируются вручную; Rust-тест генерирует независимую копию под target и сравнивает весь набор файлов. CI проверяет drift до TS/frontend checks.

ApiReply — union ok/value или error/code. Функция operationApi в src/lib/operation-api.ts использует Tauri Channel, возвращает типизированные domain errors и скрывает сырой transport error. IPC acknowledgement ограничен 5 секундами; scan timeout отдельно проверяется backend. Автоматических повторов нет.

**Timeout не доказывает, что запрос не был выполнен.** Нельзя автоматически повторять execute. До любых реальных эффектов в SPE-63 обязательны durable outcomes/history/recovery; текущая синхронная симуляция этого не реализует. Отмена mock scan — кооперативная, с проверкой каждые 30 мс.

## Память и жизненный цикл

В памяти сохраняются максимум 32 задачи/snapshots и 32 активных плана. При новом scan вытесняется самая старая terminal-задача вместе со snapshot/планами; работающие задачи не вытесняются. Когда все slots заняты работающими задачами — capacity. Планы очищаются при создании следующего; использованные/просроченные ID могут стать not_found. После перезапуска все ID недействительны; persistence/history — отдельная задача.

В mock всегда три искусственных кандидата. Эти ограничения не являются заявлением о готовности сканера больших дисков.

## Проверка

Rust: lifecycle/progress, pagination/ID ownership, confirmation/protected, expiry/replay, changed identity, cancel, adapter failure, timeout/disconnection, capacity/retention, JSON/UUID/large-byte boundaries, Tauri Channel/dispatch roundtrip, TS drift.

Frontend: typed request/response, private-channel progress, domain/transport errors, acknowledgement timeout/no retries, browser fallback. Existing UI/keyboard smoke сохраняются.

Native release smoke проверяет настоящий IPC в WebView2: доступ к разрешённому запросу, typed not_found, отказ в mock scan и отклонение произвольной delete_file операции. Реальных пользовательских файлов/реестра в тестах нет.

Локально 24.10.2026 на Windows 11 прошли: 11 Rust tests, 6 frontend tests, 3 browser smoke, types:check, check/lint/format, release build и native smoke. Дополнительно проверен настоящий TypeScript-клиент через Vite/debug WebView2: progress, pagination, plan/confirmation, simulation, cancel, timeout, adapter error. Эта дополнительная debug-проверка выполнена локально, отдельно от автоматического release smoke.

UI не меняется и не показывает mock findings пользователю. Новые скриншоты для этой backend-задачи не требуются; монохромная foundation остаётся прежней.
