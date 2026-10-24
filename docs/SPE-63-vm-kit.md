# SPE-63: воспроизведение VM-проверок

Это dev-набор для одноразовой Windows VM, не часть production-приложения. Сначала dry-run/unit tests, затем отдельный checkpoint перед каждым сценарием. Не подключать личные папки, профили браузеров, физические диски или credentials хоста.

## Сборка

Для текущего набора использована отдельная копия исходников в `C:\Users\Public\Documents\PC Care VM build`. Она нужна, чтобы compile-time пути Rust-тестов совпадали с доступным для записи fixture-каталогом внутри VM. Исходники, `dist` и lockfiles копируются для сборки; `.git`, `.tools`, `target`, `node_modules` и личные данные в VM не передаются.

В этой копии:

```powershell
cargo test --locked --lib --no-run --message-format=json
cargo build --locked --example vm_recycle_probe --example vm_safety_runner --features vm-recycle-probe
```

Первый вызов только собирает тесты. Поле `executable` соответствующего `compiler-artifact` с `profile.test=true` указывает на lib test EXE. Production и QA/test артефакты не следует смешивать.

Создать read-only ISO с Joliet-именами и следующим содержимым:

| Имя на ISO             | Источник                                           |
| ---------------------- | -------------------------------------------------- |
| `pc-care-tests.exe`    | Собранный lib test EXE                             |
| `vm_recycle_probe.exe` | Одноимённый example EXE                            |
| `vm-test-runner.exe`   | Копия `vm_safety_runner.exe`                       |
| `vm-probe-runner.exe`  | Вторая копия того же `vm_safety_runner.exe`        |
| `*.ts`                 | `apps/desktop/src/generated/*.ts`                  |
| `manifest.json`        | SHA-256 каждого EXE/type и снимок хэшей исходников |

GUI-обработчик выбирает ровно один из двух режимов по собственному имени. Он не принимает пути или shell-команды, проверяет VM-маркер до записи, требует явного запуска, перенаправляет вывод собственного test/probe EXE в отчёт и синхронизирует его перед показом результата. При ошибке возвращает ненулевой exit code. Автозапуска и изменений реестра нет.

Новые generated types записываются без переноса read-only атрибута ISO. Совпадающие существующие файлы переиспользуются, отличающиеся сохраняются с явной ошибкой: для другого набора types нужен чистый fixture workspace. Это проверяет отдельный unit test обработчика, который запускается и в CI; COM probe на CI host не запускается. Скопируйте собранные EXE в отдельный payload-каталог сразу после сборки этой копии: другая сборка с общим `target-dir` может заменить артефакты, содержащие compile-time пути.

## VM

Текущая VM: QEMU 11.1.0, отдельный qcow2, SMBIOS manufacturer `PC Care VM`, product `Disposable test VM`. Маркер — защита от случайного запуска, не аттестация изоляции. Параметры гипервизора нужно проверить отдельно.

Первоначальные internal snapshots включали RAM. После восстановления одного из них следующий `savevm` завершил запись snapshot, но монитор QMP перестал отвечать. Изолированная VM остановлена; `qemu-img check` без repair сообщил `No errors were found`. Из `pccare-before-unit-58` создана и отдельно проверена дисковая база. Она используется только как backing file для нового overlay, с TCG single thread, без сети и общих папок. Исходный образ со snapshots сохранён. Проверка qcow2 не заменяет проверку гостевой файловой системы.

Для новых сценариев использовать отдельный overlay от этой базы; после экспорта отчёта остановить гостя и вернуть исходную дисковую точку новым overlay. Не выполнять `qemu-img` repair/convert над образом, пока использующий его QEMU работает. Не восстанавливать snapshot ради обхода ограничений Evaluation и не менять часы, licensing services или настройки безопасности.

## Запуск и отчёты

В гостевой Windows открыть DVD и явно запустить нужный GUI-обработчик. Он работает только на новых искусственных fixtures. Unit tests используют UUID-папки `target/policy-fixtures`; probe — `target/vm-probes` внутри compile-time workspace.

- `vm-test-runner.exe`: отчёт `%TEMP%\PCcare-VM-test-results.txt`
- `vm-probe-runner.exe`: отчёт `%TEMP%\PCcare-VM-recycle-probe.json`

Не считать код 0 у экспериментального probe доказательством безопасности executor: нужно проверить каждый HRESULT, callbacks, сохранение file identity и sentinels. Нет полного native cleanup flow, пока это прямо не подтверждено в [результатах SPE-63](SPE-63-execution.md). Текущая Evaluation показывает истёкшую лицензию; активация и полная матрица Windows 10/11 не подтверждены.
