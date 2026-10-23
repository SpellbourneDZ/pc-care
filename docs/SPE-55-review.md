# SPE-55 — infrastructure review

Linear: https://linear.app/splbrn/issue/SPE-55/sozdat-github-repozitorij-i-bazovuyu-infrastrukturu-monorepozitoriya

## Изменения

- pnpm/Cargo workspace, Tauri 2/Rust + React/TS strict/Vite, Tailwind и официальный shadcn/Radix Button.
- apps/desktop/src/ui, src-tauri/domain и src-tauri/win/adapters; пока без системных операций, IPC-команд и FS/shell/registry plugins. Capabilities пустые, production CSP локальный, dev HMR разрешён отдельно.
- RU default, semantic tokens, Segoe UI, системные light/dark themes, keyboard disclosure, reduced motion. Полная дизайн-система — SPE-56.
- По решению владельца палитра монохромная: белый/чёрный/нейтральный серый, включая фокус, selection, destructive token и app icon. Скриншоты и native EXE обновлены и повторно проверены.
- Зафиксированы Node 24.22.1, pnpm 10.34.8, Rust stable release 1.99.1, Tauri 2.12.5 / CLI 2.12.4 и lockfiles.
- AGENTS.md, README и Windows setup, базовый Windows CI и PR template. Нет фиктивных packages и сетевого backend.
- main содержит bootstrap commit для первого PR; реализация в codex/spe-55-monorepo. GitHub: https://github.com/SpellbourneDZ/pc-care. Лицензия MIT, upstream notice shadcn сохранён отдельно.

## Проверка — 24.10.2026, Asia/Novosibirsk

Windows 11 Pro build 26200 x64, WebView2 155.0.4283.62. Приложение проверялось без административного токена. Rust и C++ Build Tools/SDK установлены с разрешения владельца.

| Проверка                                | Результат                                                                         |
| --------------------------------------- | --------------------------------------------------------------------------------- |
| pnpm install --frozen-lockfile          | passed                                                                            |
| check / lint / format:check / build:web | passed                                                                            |
| pnpm test                               | 2 passed: keyboard/disclosure, CSP/capabilities                                   |
| pnpm test:smoke                         | 3 passed: Edge light 1280×720, dark 1920×1080, compact 720×520; keyboard/offline  |
| cargo fmt / clippy -D warnings          | passed                                                                            |
| cargo test                              | harness passed, 0 Rust tests — domain logic ещё не реализована                    |
| pnpm build                              | passed, unsigned release EXE, 8 705 536 bytes                                     |
| pnpm test:native                        | 1 passed: release WebView2, offline reload, CSS, keyboard                         |
| pnpm dev --no-watch                     | passed: Vite + debug EXE + WebView2, работающая кнопка, без JS/CSP console errors |

Release SHA-256 монохромной версии: `EA6EAC5A11E9E28A7E0A5FC33E1DED96CA5693132CC630C23A689C7F5D7D5566`. Authenticode: NotSigned. Installer не собирался; NSIS current-user намечен в конфигурации, bundling выключен.

Скриншоты просмотрены: [light](screenshots/shell-light.png), [dark](screenshots/shell-dark.png), [compact](screenshots/shell-compact.png), [release WebView2](screenshots/shell-webview2.png), [dev WebView2](screenshots/shell-dev-webview2.png). При минимальной высоте есть вертикальная прокрутка, горизонтального переполнения нет.

Дополнительно: свежий локальный Git clone → install --frozen-lockfile → check → build:web — passed на том же Windows 11 хосте с уже установленными инструментами.

## Права и ограничения

Пользовательские/системные файлы не сканировались и не удалялись, реестр/автозагрузка не менялись. Тесты создают собственные profiles/artifacts; CDP включается только для дочернего процесса на loopback, тест закрывает только своё дерево процессов.

Не выполнены: branch protection (SPE-60), чистые Windows 10/11 VM clone→install→dev→build. Текущий ПК не заменяет обе VM. Installer/signing, DPI и screen reader также не подтверждены. Результаты удалённого CI проверять в PR по текущему commit.

Linear get_agent_context/record_agent_learning возвращают Tool not found; проект, все 30 issues/зависимости и три документа доступны и прочитаны через MCP. Особенность Windows proxy для shadcn описана в setup docs без адресов/секретов.

После приёмки SPE-55 следующая по приоритету и зависимостям — SPE-58 (typed API/plans/progress → SPE-59 policy). SPE-56 также разблокируется. Задача не переводится в Done до owner acceptance.
