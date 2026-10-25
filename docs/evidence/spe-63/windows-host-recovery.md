# SPE-63: приватная карта восстановления на Windows-хосте

Проверена реализация `execution_recovery.rs` поверх `bad402b`. Production execute остаётся unsupported; IPC и capabilities не расширялись. Тесты используют новые UUID fixtures и искусственные receipts, не выполняют COM или перенос исходных файлов в staging.

## Проверенные случаи

- Карта сохраняется до создания staging-файла и использует удерживаемый backend snapshot, а не display path
- Запись переживает закрытие и повторное открытие журнала; исходное содержимое не меняется
- Simulated plan, чужой candidate ID и хранилище внутри/над cleanup root отклоняются
- Mixed-case extension учитывается; malformed/future schema, lock, hardlink и orphaned mapping не трактуются как пустой список
- После ошибки или panic адаптера pending-карта сохраняется, batch завершается с Unknown, новый план блокируется после перезапуска
- Закрытие карты требует ранее сохранённого success; после него тот же план может подготовить следующий элемент
- Ошибка закрытия карты сохраняет подтверждённый success и признаки journal_error/recovery_pending; повторное закрытие метаданных после перезапуска не вызывает файловый эффект

## Проверки

| Команда                                                                         | Результат                                               |
| ------------------------------------------------------------------------------- | ------------------------------------------------------- |
| `pnpm rust:check`                                                               | fmt/clippy и 72 Rust tests, 0 failed/ignored            |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Успешно                                                 |
| `cargo test --locked --example vm_safety_runner --features vm-recycle-probe`    | 1 regression test passed                                |
| `pnpm check`, `pnpm lint`, `pnpm test`                                          | Успешно; 26 frontend tests                              |
| `pnpm format:check`, `pnpm build:web`, `pnpm test:smoke`                        | Успешно; 30 browser tests                               |
| `pnpm build`, `pnpm test:native`                                                | Production EXE; HTTP regression + WebView2 smoke passed |
| `pnpm build:qa`, `pnpm test:settings`                                           | QA EXE; 2 real IPC tests passed                         |

## Границы

Карта содержит приватный путь и metadata, хранится отдельно от pathless result JSON и не является разрешением на restore/delete. Шифрование, явная DACL staging namespace и защита от процесса с полным доступом той же учётной записи не заявляются.

Последняя VM evidence относится к 64 тестам. Новые 8 recovery tests пока проверены только на хосте. Перенос по handle, проверка тома/identity назначения, актуальная policy/settings/process revalidation, recovery UI, полная COM-цепочка и потеря питания ещё не проверены этой реализацией.
