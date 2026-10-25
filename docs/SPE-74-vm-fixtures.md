# SPE-74: искусственные MSI для изолированной VM

## Что готово

`pnpm build:uninstall-fixture` создаёт пакет и `manifest.json` в новом UUID-каталоге `target/uninstall-fixtures`. `pnpm test:uninstall-fixture` создаёт два пакета с одинаковыми именем, издателем и версией, проверяет разные identities и сохраняет их manifests в `target/uninstall-fixtures/last-tested.json`.

Это **сборка и чтение базы MSI**, без установки, advertisement, repair или uninstall. Скрипт не вызывает msiexec/InstallProduct. Windows Installer ProductState нового GUID проверяется до/после: продукт должен оставаться неизвестным. Тест включён в Windows CI; пакеты не входят в release/installer artifacts приложения.

Состав каждого пакета:

- Имя `PC Care VM Fixture`, издатель `PC Care Fixture Lab`, версия `1.0.0`. ProductCode, PackageCode, ComponentId, UpgradeCode и instance UUID создаются заново.
- Один обычный текстовый файл `fixture.txt` с instance/ProductCode. Нет executable payload, custom actions, Registry/ServiceInstall/Environment/RemoveFile tables. Только стандартные MSI file/registration actions.
- Per-user scope, без ALLUSERS; summary WordCount=8, минимальный Installer 4.0. Назначение: `LocalAppDataFolder/PC Care VM Fixtures/<instance UUID>`. Два пакета используют разные папки и компоненты.
- Uncompressed source рядом с MSI, чтобы не требовать WiX, SDK packaging tools или новых зависимостей.
- LaunchCondition требует отдельное свойство с instance UUID для **первой установки**. Это защита от случайного двойного клика, **не доказательство изоляции VM**. Удаление уже установленного fixture не требует дополнительных аргументов и может использовать штатный `/x ProductCode`.
- Manifest фиксирует точную identity, имена package/payload, destination suffix, требуемое свойство и SHA-256 обоих файлов. Node test независимо проверяет hashes/CFB header и наличие ownership-маркеров в payload. База повторно открывается read-only: проверяются разрешённые таблицы, payload count, launch guard, отсутствие ALLUSERS и summary flags.

Полная ICE validation, installation и uninstall работоспособность **ещё не доказаны**. Создание/чтение MSI не заменяет тест на Windows VM. Не запускать полученные MSI на хосте; не публиковать их как установщик PC Care.

На Windows 11 x64 локальный `pnpm test:uninstall-fixture` прошёл: один integration test создал и прочитал два пакета, проверил разные identities, hashes и отсутствие регистрации обоих продуктов. Lint, format check и offline install с frozen lockfile также прошли. Runtime приложения после `3509ad5` не менялся; его полный hosted Windows CI [40186120921](https://github.com/SpellbourneDZ/pc-care/actions/runs/40186120921) прошёл. Новый CI отдельно проверяет добавленный builder.

## Сценарий будущей проверки — только внутри VM

1. Подтвердить принадлежность VM/диска тестовому окружению, действительную Windows, отсутствие host shares/raw disks/network и наличие checkpoint до сценария. Не считать SMBIOS или fixture property доказательством изоляции. Сохранённый RAM state из прошлых экспериментов не использовать; активные диски не ремонтировать/конвертировать.
2. Перенести только два искусственных пакета с manifests/payload через изолированный носитель. Сверить hashes до запуска, убедиться, что ProductCode обоих отсутствуют и все пути относятся к своим UUID. Записать исходное состояние; сначала read-only dry-run.
3. В госте, обычным пользователем, явно разрешить установку **только этих** fixtures. Не запрашивать elevation скрытно и не обходить ограничения. Проверить фактическое назначение файла, MSI context и зарегистрированную identity; обнаружить два одинаковых DisplayName как две разные программы в PC Care.
4. Пройти read-only MSI/signature check. Он не разрешает удаление: нынешний production executor остаётся disabled. Реальный тест PC Care uninstall возможен только после отдельной реализации/проверки confirmation, rights, process outcome и повторной валидации цели.
5. Для будущего разрешённого uninstall проверить точный ProductCode, права/показанный UAC, неизменность второго fixture и искусственных sentinels вне первого UUID. После завершения проверить MSI state и файлы независимо от exit code, не объявлять успех по одному коду процесса.
6. Проверить отказ/отмену там, где она поддерживается, повторный запрос и изменённую identity. Leftovers — отдельное действие с evidence/confidence, без автовыбора и без поиска по одному имени. Никаких реальных пользовательских/системных объектов.

Текущая сохранённая Windows Evaluation отмечена истёкшей; install/uninstall и reboot/logon proof не выполнялись. Этот неизменный блокер повторно не диагностировался. Лицензирование/дату/защиту не обходить. Full Windows 10/11 / обычный пользователь / DPI / per-machine / MSIX матрица остаётся открытой.

## Файлы и команда

Builder: `apps/desktop/scripts/build-uninstall-fixture.ps1`. Проверка: `apps/desktop/scripts/uninstall-fixture.test.mjs`. Повторный запуск создаёт новые UUID, не перезаписывает прежние пакеты и не удаляет fixtures. Выходной JSON содержит только искусственные identities и относительные имена, без путей профиля владельца.

```powershell
pnpm test:uninstall-fixture
```

Команда выше **ничего не устанавливает**. Она не является разрешением выполнять MSI на хосте.

Источники: [Windows Installer database](https://learn.microsoft.com/en-us/windows/win32/msi/using-the-installer-database), [Directory layout](https://learn.microsoft.com/en-us/windows/win32/msi/using-the-directory-table), [package privilege summary](https://learn.microsoft.com/en-us/windows/win32/msi/word-count-summary), [MSI SQL syntax](https://learn.microsoft.com/en-us/windows/win32/msi/sql-syntax).
