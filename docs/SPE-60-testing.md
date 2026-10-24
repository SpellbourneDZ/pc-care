# SPE-60: Windows CI и безопасные проверки

## CI contract

`Windows checks / windows` на `windows-2022` проверяет frozen pnpm lock, Prettier, generated TS drift, typecheck, lint, Vitest, web build, browser smoke, fmt/clippy/Rust tests, draft NSIS build, production EXE/native WebView2, отдельный QA EXE/settings smoke и PE/hash smoke. Ошибка шага завершает job ошибкой; `continue-on-error`, path filters и условия пропуска обязательного job не используются. Все сторонние Actions фиксированы commit SHA, token имеет `contents: read`. PR code не получает repository secrets; workflow не использует `pull_request_target` и не запускает installer на CI host.

В GitHub для `main` настроены required check `windows`, expected App `GitHub Actions` (15368), strict up-to-date, enforce admins, force-push/delete off. Проверено через REST GET после применения. Это настройка репозитория, не поле YAML. Не создавался намеренно падающий PR и не предпринималась попытка merge для проверки блокировки; конфигурация прочитана и проверена. Stacked feature branches не защищены этим правилом; их не следует сливать вручную в обход приёмки. После последовательного принятия PR проверка останется gate для main.

Не требуем дополнительного approval от того же аккаунта, который авторит PR: он не может одобрить собственный PR. Owner acceptance остаётся обязательным правилом проекта, автоматический merge выключен в рабочем процессе.

## Draft installer

```powershell
pnpm build:installer
pnpm test:installer
```

Обычная `pnpm build` по-прежнему создаёт EXE без установщика. Только `tauri.ci.conf.json` включает NSIS current-user и MIT license. QA feature не используется. В draft отключена установка WebView2: Runtime должен быть уже установлен. Это [официальный режим Tauri skip](https://v2.tauri.app/distribute/windows-installer/#skipping-installation); при отсутствии Runtime приложение не запустится. Настройка полной доставки WebView2/подписи относится к SPE-83.

`test:installer` проверяет DOS/PE headers, x64 machine приложения, непустой NSIS setup и записывает `draft-manifest.json` с SHA256/bytes обоих artifacts. NSIS setup stub может быть x86 при x64 payload. Malformed/truncated/wrong-architecture parser cases покрыты Node test. Проверка не запускает setup/uninstaller, не доказывает их успешную установку и не заменяет подпись. Manifest/hashes относятся к текущей сборке.

Успешный CI сохраняет setup и manifest как `pc-care-draft-nsis-<commit>` на 7 дней. Это Actions artifact для ревью, не GitHub Release. NSIS toolchain загружается Tauri во время build; runtime приложения остаётся offline. Установщик unsigned; SmartScreen bypass не предлагается.

## Fixtures и dry-run

- Rust policy/settings tests создают только свои UUID fixtures под `target/policy-fixtures`. Junction/hardlink/ACL относятся к искусственным данным. Исполнение удаления не подключено.
- `pnpm build:qa && pnpm test:settings` использует отдельный `target/native-qa` и разрешённый `target/native-fixtures/<UUID>/config`; production config в тестах не сохраняется. QA binary нельзя публиковать как production.
- `pnpm test:native` открывает production EXE с отдельным WebView profile, проверяет offline UI, CSP и отказ hostile IPC/release mock. Backend execution остаётся simulated=true и actual_reclaimed_bytes="0".
- Перед добавлением каждого executor сначала нужны assertions dry-run: unchanged fixture bytes/registry values/startup entries, отсутствие внешних effects, plan/outcomes и cancellation. Затем отдельная VM/snapshot. Глобального флага, который обещает обезвредить ещё не реализованные операции, нет.

## Локальный результат

На Windows 11 выполнены typecheck/lint/format, generated TS drift, 15 frontend tests, 20 browser smoke, fmt/clippy/33 Rust tests; production и QA builds, WebView2/native settings smoke, Node artifact-parser test и NSIS PE/hash smoke. После исправления focus в SPE-57 settings-сценарий дополнительно прошёл пять последовательных перезапусков EXE. Окончательный результат hosted Windows CI находится в Checks PR. Private vulnerability reporting включён и проверен GET.

## VM matrix — запланировано, не выполнено

Использовать одноразовые Windows 10/11 x64 VM с snapshot **до каждого сценария**, обычным локальным тестовым пользователем, отдельно повышенными правами по необходимости. В VM нет общих папок с личными данными, browser profiles и пользовательских credentials. После сценария экспортировать обезличенный отчёт и восстановить snapshot.

| Сценарий                                                                 | Ожидаемый результат / evidence                                                                                                                                  |
| ------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Win10 и Win11: clean clone → tools → install → dev → build               | Версии tools, команды, exit codes, EXE start без admin                                                                                                          |
| NSIS current-user install, launch, повторная установка/upgrade/uninstall | Нет обязательного UAC, корректные shortcuts/app registration; не удаляются пользовательские настройки без отдельного выбора; installer/uninstaller log и hashes |
| WebView2 установлен / отсутствует, сеть выключена                        | При наличии — offline запуск; draft skip без Runtime честно unsupported, без обещания self-repair                                                               |
| 1280×720 и 1920×1080, настоящий OS DPI 125/150%, обе темы                | Overlays/scroll, keyboard/focus, screen reader, reduced motion, screenshots; browser DPR не считается настоящим OS scaling                                      |
| Temp fixture safe/protected/excluded, deep/Unicode/long path             | Dry-run не изменяет данные, scope/default-deny и сумма плана соответствуют fixtures                                                                             |
| Junction/symlink/mount loop, hardlink, cloud placeholder, UNC/removable  | Нет traversal/cycle/double accounting; unsupported и per-item причины видимы                                                                                    |
| Locked/no-access/changed target, cancel до/во время bulk, crash/restart  | Нет silent retry/delete fallback, revalidation и durable outcomes; skipped/error не выдаются за success                                                         |
| Настоящий recycle executor после SPE-63                                  | Только fixture IDs, explicit preview/confirm, поддержка recycle проверена, permanent fallback отсутствует; snapshot restore после теста                         |
| Startup HKCU Run/per-user folder после соответствующих задач             | Искусственные записи, backup → off/on → logon verification; system/other-user read-only                                                                         |
| Test MSI/EXE/MSIX uninstall и остатки                                    | Exact identity, официальный flow, backup registry, shared/low confidence protected; никакого подстрочного удаления                                              |

Сейчас отсутствует готовая тестовая VM, поэтому install/uninstall и destructive сценарии не запускались ни на основном ПК, ни в CI host. Это явный остающийся release gate, а не зелёный пункт проверки.

Security baseline: [SECURITY.md](../SECURITY.md). Правила required checks соответствуют [GitHub protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches#require-status-checks-before-merging).
