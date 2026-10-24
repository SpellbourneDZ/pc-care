# PC Care: постоянный контекст для Codex

## Источник требований и текущая граница

- Linear — источник требований. Перед каждой задачей читать issue, зависимости, PRD, Architecture & Security и Codex Playbook; при недоступности MCP сразу сообщать и не придумывать требования.
- Проект: https://linear.app/splbrn/project/pc-care-windows-cleaner-open-source-887c4dc0f0c3
- PRD: https://linear.app/splbrn/document/01-prd-scenarii-ux-i-granicy-mvp-d9240ad80aec
- Security: https://linear.app/splbrn/document/02-architecture-and-security-kontrakt-dlya-codex-777ce84d8c7a
- Playbook: https://linear.app/splbrn/document/03-codex-playbook-poryadok-razrabotki-i-pr-7bb3d73919a6
- Порядок milestones: 00 Foundation & UX → 01 Smart Cleanup → 02 Storage Analyzer → 03 Startup Manager → 04 Apps & Uninstaller → 05 QA & Open-source Release.
- SPE-55 — только инфраструктура и безопасный стартовый экран. Полные экраны/дизайн — SPE-56; IPC — SPE-58; защитная политика — SPE-59; расширенная Windows CI/VM стратегия — SPE-60. Не реализовывать следующие задачи внутри текущей.
- SPE-55 разблокирует SPE-56 и SPE-58. SPE-57 зависит от обеих. Сначала зависимости, затем потребители; сверять актуальные связи в Linear.
- SPE-55 и SPE-58 реализованы в PR #1/#2 (Windows CI зелёный), остаются In Review. SPE-59 — отдельная зависимая ветка поверх #2; не сливать PR автоматически.
- Уточнение владельца: после backend вернуться к SPE-56 → SPE-57 → SPE-60, не пропускать UX. SPE-59 (PR #3) тоже прошла Windows CI, остаётся на review.
- Не переводить задачи в Done до приёмки владельцем. Реализация → In Review с результатами и ограничениями.

## Стек и структура

- SPE-61: docs/SPE-61-cleanup-analysis.md. Release read-only cleanup scan через существующий start_scan kind=cleanup/scenario=complete; get_cleanup_rules отдаёт закрытый registry. UserTemp: только .tmp/.temp старше 168h Safe; Default HTTP Cache Chrome/Edge 24h Review. Browser profile markers/секреты/sidecars защищены даже внутри Temp, остальные Temp-форматы пропускаются. Никаких cookies/session/Storage и произвольных roots. UI подключать в SPE-62; cleanup execute пока unsupported до SPE-63.
- Cleanup Candidate.bytes — Windows-reported allocation, logical_bytes — EndOfFile, identity — непрозрачный полный metadata snapshot; не считать allocation гарантией freed bytes. TaskProgress.total nullable; issues содержат группированные причины/count/первый sample, truncated явно означает неполный анализ. Один cleanup worker; 32 retained tasks, caps 10000 candidates/100000 entries/128 issue groups/4 MiB учитываемых snapshot data. Exclusions фиксируются на старт, перед будущим execute перечитать settings/processes/policy.

- SPE-57: рабочие hash routes, Ctrl+K, Settings через typed IPC и TanStack Query; настройки меняются после backend acknowledgement. Прототип доступен отдельно через «О проекте», обычный запуск не использует mock scan. Контракт хранения — docs/SPE-57-shell-settings.md.
- Дополнительная проверка настроек: pnpm build:qa и pnpm test:settings. QA EXE — только target/native-qa/release; feature qa-fixtures выключена по умолчанию, принимает только UUID fixtures внутри target/native-fixtures, не публиковать её как обычную сборку. Production native smoke не сохраняет настройки пользователя.

- Windows 10/11 x64, Tauri 2, Rust stable (конкретная проверенная версия в rust-toolchain.toml), React, TypeScript strict, Vite, pnpm workspace.
- Tailwind CSS + shadcn/ui на Radix — основная компонентная база; Lucide — иконки. Использовать официальный CLI и Vite-инструкцию https://ui.shadcn.com/docs/installation/vite.
- apps/desktop/src/ui — локальные shadcn-компоненты. Не создавать вторую UI-библиотеку.
- apps/desktop/src — React/UI state и локализация (RU по умолчанию).
- apps/desktop/src-tauri — native host; domain/ — чистая предметная логика; win/adapters/ — Windows API и системные эффекты. src/lib.rs — сборка слоёв, src/main.rs — вход.
- Cargo workspace — в корне. pnpm-lock.yaml и Cargo.lock хранить в Git; прямые версии зависимостей фиксировать. Новую зависимость добавлять только при реальной необходимости.
- IPC SPE-58: один operation_api с закрытым ApiRequest; подробности docs/SPE-58-api.md. Rust contracts.rs — источник TS types, src/generated не редактировать вручную; pnpm types:generate и pnpm types:check. Wire snake_case, UUID строками, bytes десятичными строками.
- Mock scan только в debug, production отвечает unsupported. execute_plan пока только simulated=true с нулём reclaimed bytes. До реальных эффектов нужны policy/revalidation (SPE-59) и durable outcome recovery (SPE-63); timeout нельзя считать гарантией неисполнения и автоматически повторять execute.
- SPE-59: docs/SPE-59-policy.md. FilePolicy — backend-only read probe/snapshot/revalidate, не разрешение на удаление. Начальный допуск SPE-59 — обычные файлы UserTemp; SPE-61 добавляет только фиксированные HTTP-cache roots. Остальные источники default deny. Exclusions с родительским наследованием ограничивают scope. Настройки/сохранение исключений — SPE-57; правила отбора — SPE-61; связь с реальным executor и устранение финального check/use race — SPE-63.
- Системные операции только в Rust. UI не получает прямого доступа к файловой системе, реестру или shell. Typed IPC и минимальные capabilities, без raw arbitrary-path delete.
- Без сетевого backend, аккаунтов, рекламы, подписок и телеметрии по умолчанию. Работать offline; не грузить шрифты/ресурсы из сети в runtime.
- Настройки, исключения, история — локальные, с миграциями; без чувствительных данных в логах. Полное состояние задач и persistence реализовывать в соответствующих issue.

## Обязательные инварианты безопасности

- Scan ничего не изменяет. Любые мутации: immutable snapshot → selected stable IDs → backend OperationPlan → preview/явное подтверждение → повторная проверка цели → per-item результат.
- Backend повторно проверяет ID/source, canonical path, file identity/metadata; frontend-путь не является разрешением. При неопределённости default deny.
- Protected: корни, Windows/Installer/WinSxS, Program Files, ProgramData, Recovery, System Volume Information, системные профили/другие пользователи. Учитывать родительские исключения.
- Downloads, документы, пароли/cookies/сессии браузеров, restore points и component store не включать в очистку по умолчанию. Корзина — только отдельный ручной выбор.
- Не следовать symlink/junction/reparse/mountpoints по умолчанию. Проверять hardlinks, cloud placeholders, long paths/Unicode/casing, ACL/UAC, locks, сети/съёмные диски и race между scan/execute.
- По умолчанию IFileOperation + FOFX_RECYCLEONDELETE при поддержке. Не делать silent permanent-delete fallback и не обещать гарантированный откат или освобождённые байты.
- Приложение запускается без администратора; UAC только для конкретной подтверждённой операции. Без скрытой автозагрузки, служб и обхода UAC.
- Никогда не выполнять произвольные строки PowerShell/cmd/shell из UI или непроверенный UninstallString.
- Startup changes только с обратимым source-specific backup/restore; успех после подтверждения backend, не optimistic. Неизвестные/system/other-user источники read-only.
- Uninstall через официальные MSI/EXE/MSIX механизмы с проверкой identity и результата. Остатки — отдельный scan и ручной отбор с evidence/confidence, shared/system protected.
- Не удалять файлы/ключи по одному имени или подстроке. Реестр — отдельное подтверждение, экспорт и проверка резервной копии.
- Progress/cancel и success/skipped/error обязательны для долгих/bulk операций; ошибки не скрывать.
- Тесты никогда не удаляют реальные пользовательские/системные данные и не меняют реальный автозапуск/реестр. Только искусственные fixtures и VM со snapshots; сначала dry-run.
- Registry optimizer, accelerators, драйверы, службы, privacy tweaks, дефрагментация, schedule cleanup и duplicates не входят в v1.0.

## UX

- Решение владельца от 24.10.2026: палитра строго белая, чёрная и серая в обеих темах, включая акценты, focus ring, выделение и app icon. Без зелёного и других цветных акцентов. Риски/ошибки/успех различать текстом, иконками и контрастом, не цветным кодированием.
- Спокойный desktop UI, свои semantic color/spacing/radius/type tokens, light/dark/system, видимый keyboard focus, aria, reduced motion.
- Навигация PRD: Обзор, Умная очистка, Диски, Автозагрузка, Программы; нижняя зона — История, Настройки, О проекте.
- Не показывать demo-данные как реальные, fake health/speed/size, promises of acceleration/100% uninstall.
- Для настоящих flows обязательны empty/loading/error/partial/unsupported/offline и понятные RU-тексты.
- Целевые окна 1280×720 и 1920×1080, DPI 125/150%; overlays/scroll проверять в Windows WebView2.
- SPE-56 формирует полную дизайн-систему; tokens SPE-55 — начальная база.
- SPE-56: docs/SPE-56-design-system.md, примеры src/preview. Моковые экраны всегда явно помечены, не подключать их как настоящие системные результаты. В SPE-57 рабочий shell должен начинаться без mock findings.
- CSP остаётся без unsafe-inline: meta pc-care-style-nonce с Tauri STYLE_NONCE_TOKEN → get-nonce перед render; передавать nonce в новые Radix Viewport/style consumers. Native UI tests должны замечать CSP violations, а не только JS errors.

## Работа и проверки

- SPE-60: pnpm build:installer / pnpm test:installer собирают только unsigned draft NSIS + PE/hash smoke, не запускают installer. Для install/uninstall нужен snapshot VM; см. docs/SPE-60-testing.md и SECURITY.md. У main required windows/GitHub Actions 15368, strict/enforce admins; stacked feature branches правилом не защищены, не сливать без владельца.

1. Прочитать Linear, затронутый код/всех callers и локальные AGENTS; описать небольшой план и риски.
2. Работать в отдельной ветке codex/<issue>-<scope>. Маленький полный diff, без ненужных абстракций/рефакторинга.
3. Реализация, обработка ошибок, релевантные unit/integration/smoke тесты, документация; screenshots для UI.
4. pnpm check, pnpm lint, pnpm test, pnpm format:check, pnpm build:web, pnpm test:smoke, pnpm rust:check, pnpm build, pnpm test:native. Указывать фактически выполненные и заблокированные проверки.
5. Native smoke и системные эффекты — Windows/WebView2; destructive flows только в тестовой VM. Browser smoke не заменяет native smoke.
6. PR: issue, результат, tests/evidence, screenshots, permissions/threats, ограничения. Подготовленный шаблон .github/pull_request_template.md.
7. Обновить Linear до In Review после реализации, без Done до owner acceptance. Непройденные AC перечислить явно.

- GitHub репозиторий создавать/публиковать только после подтверждения владельца, названия и настроек. Не придумывать remote URL.
- Репозиторий: https://github.com/SpellbourneDZ/pc-care, Public, main, Issues on, Wiki off. Владелец разрешил продолжить с предложенными настройками 24.10.2026. Не менять visibility/owner и не сливать PR без разрешения.
- LICENSE — MIT, уведомления о скопированных shadcn-компонентах сохранять в THIRD_PARTY_NOTICES.md.
- Установку Rust и Microsoft C++ Build Tools + Windows SDK владелец разрешил в сессии SPE-55. Это не разрешение изменять пользовательские данные.
