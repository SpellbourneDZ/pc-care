# SPE-59 — защитная политика

## Доступная граница

`domain/policy.rs` задаёт typed причины отказа, file identity/metadata и проверку выбранного ID. `win/adapters/path.rs` и `files.rs` собирают Windows evidence и применяют правила. Новых IPC-команд, capabilities или UI нет. `execute_plan` SPE-58 по-прежнему выполняет только симуляцию; реальные пути не принимаются из frontend.

`FilePolicy::for_current_user()` получает профиль, LocalAppData и защищённые расположения через Shell Known Folders текущего токена, без доверия к строкам UI или переменным окружения. Начальный поддержанный источник SPE-59 — `FileSource::UserTemp`; [SPE-61](SPE-61-cleanup-analysis.md) расширяет read-only policy только фиксированными HTTP-cache roots и защитой browser state. Допуск ограничен обычными файлами в `LocalAppData/Temp` внутри текущего профиля. Это проверка границы, а не правило «всё в Temp безопасно удалить»: возраст, активность, принадлежность и правила кешей реализуются в SPE-61. При ошибке Known Folders или перенаправлении Temp за пределы профиля — отказ.

Корни, Windows/Installer/WinSxS, Program Files, ProgramData, Recovery, System Volume Information, корзина, системные/другие профили отклоняются. Downloads, Documents, Desktop, браузерные credentials/session state и произвольные AppData не входят в допуск. Перенаправленные Downloads/Documents/Desktop остаются protected, даже если попали внутрь Temp. Ручные сценарии дискового анализатора, корзины и остатков потребуют отдельных source-specific правил в соответствующих issue.

## Пути и Windows evidence

- Только абсолютный локальный DOS path, включая extended-length `\\?\C:\…`. UNC/device/GLOBALROOT, ADS, dot segments, зарезервированные DOS имена, неоднозначные окончания и wildcards отклоняются. Лимиты одного probe: 32 000 UTF-16 units и 256 компонентов.
- Сравнение по компонентам через Windows `CompareStringOrdinal(ignoreCase=true)`, без языкового преобразования и Unicode normalization. Частичное совпадение имени не даёт допуск. Case-sensitive каталоги — unsupported.
- Только fixed drive. Каждый префикс от корня открывается `OPEN_EXISTING`, `OPEN_REPARSE_POINT`, `BACKUP_SEMANTICS`; предки проверяются до открытия ребёнка. Reparse/junction/symlink/mountpoint и offline/recall flags дают отказ. Короткие имена/неожиданное разрешение пути также отклоняются.
- Handles открываются без share-write/share-delete и удерживаются до окончания probe. Для файла запрашиваются `FILE_READ_ATTRIBUTES | DELETE`: это проверяет доступ текущего токена, не вызывает удаление и не повышает права. ACL/privilege, sharing/lock и неизвестные ошибки возвращаются как typed denial. Чтение содержимого не требуется.
- По handle читаются final normalized path, volume serial + 128-bit file ID, size, creation/write/change time, attributes и link count. Каталоги, pending delete, hardlinks и неподдержанные атрибуты отклоняются. На драйвере/Windows без нужной информации нет permissive fallback.

`snapshot(id, source, path)` создаёт неизменяемый backend snapshot с приватными полями. `revalidate(selectedId, snapshot)` заново проверяет источник, scope, актуальные исключения, путь и все перечисленные metadata. Неизвестный ID/изменённая цель дают отказ. Snapshot не сериализуется из frontend.

Проверка не устраняет весь TOCTOU: handles закрываются при возврате, а ACL/атрибуты могут меняться конкурентно. **Её результат нельзя использовать как разрешение выполнить отдельное удаление по пути.** SPE-63 должен связать последний check с поддержанным способом исполнения и outcome recovery; до этого реальных effects нет. Запрос/функция generic registry delete отсутствует; source-specific registry ownership/backup/confirmation относятся к SPE-71/78.

## Исключения

`Exclusions::add/remove` валидируют локальный путь; дубликаты учитывают Unicode/casing Windows. Исключение каталога распространяется на потомков с границей компонента, включая изменения после snapshot. Исключения никогда не расширяют допуск. UI и локальное сохранение/миграции настроек — SPE-57; в этой задаче механизм в памяти, без записи пользовательских настроек.

## Проверки и AC

Windows fixtures создаются только внутри `target/policy-fixtures/<UUID>`. Реальные пользовательские/системные файлы не удаляются, реестр/автозапуск не меняются. ACL изменяются и восстанавливаются только у новых искусственных файла/родителя; junctions и hardlinks связывают только fixtures. Fixtures остаются как test artifacts: рекурсивной очистки через junction нет.

Автоматические проверки охватывают отказ всех protected path fixtures, namespaces/ADS/traversal, redirected documents, parent exclusions/casing, selected ID, каждое metadata поле, подмену файла, реальные hardlinks/locks/ACL, junction escape/loops, Unicode и длинные пути. Cloud flags проверены искусственными metadata, без настоящего облачного провайдера. Закрытая IPC-схема отклоняет generic file/registry delete и frontend path в execute.

Реальная очистка, IFileOperation, UAC, поведение на чистых Windows 10/11 VM и настоящий cloud provider здесь не проверяются. AC повторной проверки реализован для backend dry-run на fixtures; включение этого gate в реальное исполнение относится к SPE-61/63 и остаётся обязательным до любых удалений.

24.10.2026 локально на Windows 11 прошли: `pnpm rust:check` (24 Rust tests, fmt/clippy), `types:check`, `check`, `lint`, `test` (6 frontend tests), `format:check`, `build:web`, `test:smoke` (3 профиля), `build`, `test:native` (настоящий release EXE/WebView2, offline и IPC denial). UI не менялся; прежние монохромные screenshots сохраняют актуальность.

Собранный `target/release/pc-care.exe` не подписан и не является установщиком. SHA-256: `1488D5A98B628B60C71874E97E9EAE419DCBDA51B1239BD09C0CC221BD0019E2`. Статус удалённого Windows CI — в Checks PR; задача остаётся на owner review, без Done/merge.

## Первичные источники

- [CreateFileW: flags, sharing, access](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
- [GetFinalPathNameByHandleW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew)
- [GetFileInformationByHandleEx](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getfileinformationbyhandleex)
- [CompareStringOrdinal](https://learn.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-comparestringordinal)
- [SHGetKnownFolderPath](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shgetknownfolderpath)
