# SPE-63: удержание дескрипторов на Windows-хосте

Проверена рабочая версия после `b48f315`, 25 октября 2026. Изменены `files.rs`, `cleanup.rs` и общий `preferences::directory_guard`; добавлены 4 regression tests. Это read-only проверки новых guards на искусственных fixtures, не VM-проверка полного executor.

## Найденная ошибка

Первый запуск `cargo test --locked --lib reservation`: 2 passed, 1 failed. Тест ожидал отказ открытия родительского каталога с правом `DELETE`, но получил дескриптор. В `open_path` каталоги-предки открывались только с `FILE_READ_ATTRIBUTES`, что не обеспечивало ожидаемый share-deny.

После добавления `FILE_LIST_DIRECTORY` ко всем каталогам пути: 3 passed. То же исправление внесено в общий guard настроек и журнала; отдельный regression test подтвердил блокировку до `drop` и освобождение после него.

Новые тесты открывают дескрипторы с правами записи/удаления для проверки sharing violation, но не выполняют запись, перенос или удаление через них. Содержимое fixtures проверяется после освобождения guards.

## Результаты

| Проверка                                                                        | Результат                                                   |
| ------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `pnpm rust:check`                                                               | fmt, clippy default targets; 64 lib tests, 0 failed/ignored |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Успешно                                                     |
| `pnpm check`, `pnpm lint`, `pnpm test`                                          | Успешно, 26 frontend tests                                  |
| `pnpm format:check`, `pnpm build:web`                                           | Успешно                                                     |
| `pnpm test:smoke`                                                               | 30 passed                                                   |
| `pnpm build`, `pnpm test:native`                                                | Production EXE, 1 WebView2 smoke passed                     |
| `pnpm build:qa`, `pnpm test:settings`                                           | QA EXE, 2 real IPC tests passed                             |

## Границы

Production cleanup execute остаётся unsupported. Удержание дескрипторов не запрещает все изменения атрибутов, создание дочерних файлов или действия через ранее открытые handles/mappings. Проверка текущих правил, настроек и процессов, durable staging mapping и полная VM-цепочка исполнения ещё не реализованы.

Предыдущий VM-отчёт относится к 60 тестам. Новые 4 теста в VM пока не запускались. Полная матрица Windows 10/11, стандартного пользователя, DPI и installer остаётся непроверенной.
