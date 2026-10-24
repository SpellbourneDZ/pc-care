# SPE-61: read-only анализ очистки

Backend анализирует фиксированные источники через существующий `operation_api`. Рабочий UI подключается в SPE-62; текущий «Найти мусор» остаётся отключённым. Реального удаления нет. Preview plan по реальным кандидатам остаётся simulated, `execute_plan` для cleanup возвращает unsupported до SPE-63.

## Закрытый registry

| Rule ID              | Источник относительно Known Folder LocalAppData | Класс  | Минимальный возраст |
| -------------------- | ----------------------------------------------- | ------ | ------------------- |
| user-temp-v1         | Temp                                            | Safe   | 168 часов           |
| chrome-http-cache-v1 | Google/Chrome/User Data/Default/Cache           | Review | 24 часа             |
| edge-http-cache-v1   | Microsoft/Edge/User Data/Default/Cache          | Review | 24 часа             |

В UserTemp v1 allowlist включает только .tmp/.temp; другие форматы получают unverified_type и не становятся кандидатами. Это сознательное ограничение покрытия, а не утверждение, что все старые файлы Temp безопасны.

Возраст проверяется по creation, last-write и change time; все три должны быть достоверными и старше порога. Недавние/future/неизвестные timestamps пропускаются. Это консервативное правило отбора, не доказательство ненужности файла; preview/подтверждение всё равно обязательны перед будущей очисткой.

App caches относятся к Review: ресурсы могут потребовать повторной загрузки и перестать быть доступны offline. Учитывается только стандартный Default profile. Другие профили, нестандартные UserDataDir/DiskCacheDir, сторонние браузеры и произвольные AppData каталоги не обнаруживаются и не используются как fallback. Registry отдаёт RU название, объяснение, класс и порог каждого правила.

Браузерные cookies/пароли/session/Network/Storage/IndexedDB находятся вне разрешённого Cache root. По известным компонентам секретных хранилищ и metadata markers профиля защищаются также переносимые browser profiles внутри Temp, включая WAL/SHM/journal/backup/temp sidecars. Имена используются только для запрета, не для разрешения очистки. Добавление profile marker после snapshot отзывает допуск при revalidation. Содержимое HTTP-кеша не читается и не разбирается. Root основан на документированном расположении, не на подстроке имени программы/файла.

## Scope и наблюдения

- Production roots берутся из Windows Known Folders текущего пользователя, не из frontend или TEMP environment. Перед открытием проверяются текущий profile, protected folders и родительские exclusions.
- Source policy расширена только на два фиксированных Cache root. Успешный probe остаётся backend read-only evidence, не разрешением на удаление.
- Directory probes проверяют каждый ancestor, final canonical path, fixed drive и case-sensitive/reparse/cloud признаки. Handles удерживают предков без write/delete sharing во время перечисления. Проверено, что это запрещает открытие directory writer, но не создание искусственного child file.
- Junction/symlink/mountpoints не обходятся; hidden/system/readonly/encrypted директории, специальные файлы, hardlinks, sparse/cloud/неизвестные file attributes пропускаются. Не запрашиваются UAC или смена ACL.
- Перед выдачей кандидата выполняется повторный probe: file ID/volume, canonical path, sizes, timestamps, attributes, link count. Ошибка/изменение возвращаются как причина пропуска.
- Перед каждым cache source production перечисляет process metadata через Toolhelp32. Обнаруженный chrome.exe/msedge.exe блокирует соответствующий source; ошибка перечисления также закрывает его. Процессы не останавливаются, командные строки/контент не читаются. Это наблюдение в начале source, не гарантия, что браузер не запустится позже; executor SPE-63 обязан повторить проверки.
- Exclusions фиксируются при старте анализа из подтверждённых settings. После изменения exclusions нужен новый scan; будущий executor обязан загрузить актуальные настройки перед эффектом.

Scan не создаёт/изменяет/удаляет файлов. Read probes запрашивают DELETE как проверку ACL/sharing, без вызова удаления. Финальная гонка check/use перед реальным mutator в этой задаче не решается; такой mutator отсутствует.

## Размеры и evidence

`Candidate.logical_bytes` — EndOfFile. `bytes` — allocation, сообщённая Windows: FileStandardInfo.AllocationSize для обычных файлов и GetCompressedFileSizeW для compressed. Path-based запрос compressed выполняется при удерживаемых ancestors/leaf, после запрета reparse. Hardlinks/sparse/неизвестные случаи не включаются в сумму.

Это размер, учтённый файловой системой для кандидата, без обещания фактически освобождённого места, уникальных физических блоков или учёта всех filesystem metadata. Файл может измениться после scan, а shared storage/compression/recycle влияют на результат. Native compressed fixture подтверждает отличие physical от logical. Оба счётчика передаются десятичными строками без потери u64 в JS.

`identity` — непрозрачная строка полного backend metadata snapshot, включая volume/file ID. Клиент не должен её разбирать или использовать как разрешение. Backend хранит immutable candidate по случайному stable ID; произвольные frontend paths не принимаются.

## API, progress и лимиты

```json
{"type":"get_cleanup_rules"}
{"type":"start_scan","data":{"kind":"cleanup","scenario":"complete","timeout_ms":300000}}
```

`scenario` сохранён ради совместимости mock API; для cleanup разрешён только complete. `get_task`, `cancel_task`, `get_scan_result` и Channel остаются общими. Cleanup поддерживается и в release; mock по-прежнему выключен там. Capabilities не расширены.

Progress: processed — просмотренные entries, включая директории; total=null, поскольку предварительного полного обхода нет. Первое/терминальное событие обязательны, промежуточные обновления ограничены примерно 20 в секунду. Timeout 1–300000 мс, cancel кооперативный между probes. Долгий kernel call нельзя прервать этим timeout; IPC acknowledgement timeout также не доказывает отмену работы. Автоматических повторов нет.

CandidatePage включает issues и truncated. Issues группируются по rule/reason, содержат count и первый sample_path. Count — наблюдённые события/объекты, включая недоступные directories/sources; число их неперечисленных потомков неизвестно. Недавний/исключённый объект не следует называть ошибкой удаления. При ошибках других entries scan продолжается; весь результат нельзя выдавать за полный, если есть препятствия или truncated.

Одновременно работает один cleanup worker. Общая retention — до 32 задач/snapshots; вытесняются только завершившие worker. Snapshot ограничен 10000 кандидатами, 100000 просмотренными entries, 128 группами issues и 4 MiB бюджетом учитываемых строк/структур. Это бюджет сохранённых данных, не точный общий heap процесса. При достижении лимита сохраняется частичный snapshot с truncated и limit_reached. Cancel/fatal timeout не публикуют snapshot. Native execute остаётся unsupported; восстановление после crash и durable outcomes — SPE-63.

## Проверки и границы

Локально прошли 47 Rust tests, включая TS drift, NTFS compression, portable browser protection и отказ process probe; frontend — 15 tests, browser smoke — 20 сценариев. Проверены check/lint/format, обе конфигурации clippy, web/production/QA builds и оба native smoke. Hosted CI перечислен в PR.

Все изменяемые данные тестов — новые UUID fixtures под target/policy-fixtures или target/native-fixtures. Тесты сравнивают исходные bytes после анализа, проверяют age/exclusions, running/unavailable app state, lock/ACL, настоящий junction/hardlink, изменившийся файл, NTFS compression, directory pinning и bounded snapshots. ABI/TypeScript проверяются генератором Rust; frontend callers компилируются с nullable total и новым logical_bytes.

Native QA использует только искусственный profile и process state. Через настоящий IPC проверяются rules, progress и анализ свежего sentinel без изменений, затем сохранение/перезапуск settings. Production process enumeration проверена компилятором; реальная матрица браузерных процессов/версий и clean Windows 10/11 VM ещё не пройдены. Реальные пользовательские Temp/cache не используются тестами. Нового UI нет, дополнительные screenshots не требуются.

## Первичные источники

- [Chromium user data/cache directories](https://chromium.googlesource.com/chromium/src/+/show/master/docs/user_data_dir.md).
- [Chromium HTTP cache path отдельно от Network/cookie databases](https://chromium.googlesource.com/chromium/src/+/main/chrome/browser/net/profile_network_context_service.cc).
- [Chromium constants: Default, Cache, Cookies, Network, Sessions-related data](https://raw.githubusercontent.com/chromium/chromium/main/chrome/common/chrome_constants.h).
- [Chromium disk cache](https://www.chromium.org/developers/design-documents/network-stack/disk-cache/) и [Microsoft Edge DiskCacheDir](https://learn.microsoft.com/en-us/deployedge/microsoft-edge-browser-policies/diskcachedir): кеш управляется браузером, нестандартный root не является разрешением анализировать произвольный путь.
- [FILE_STANDARD_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_standard_info), [GetCompressedFileSizeW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getcompressedfilesizew).
- [Mozilla: содержимое browser profile](https://support.mozilla.org/en-US/kb/profiles-where-firefox-stores-user-data) — известные credential/cookie/session/form stores используются для защитного запрета, не как источники очистки.
