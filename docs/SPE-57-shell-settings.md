# SPE-57: оболочка и локальные настройки

Рабочий запуск открывает Обзор без примеров файлов/программ. Sidebar ведёт к четырём разделам и Истории. Hash routes поддерживают прямой переход и назад/вперёд; неизвестный маршрут открывает Обзор. Поиск разделов доступен по кнопке и Ctrl+K, управление стрелками/Enter/Escape. О проекте открывает отдельный явно помеченный прототип SPE-56; его тема временная и не сохраняется.

CTA анализа и область томов показывают недоступность источника без придуманных размеров/результатов. Настоящий обзор free/used/total и даты анализа — SPE-69 (зависит от SPE-57 и сканера SPE-65); scan и операции подключаются в следующих задачах. Текущий shell не вызывает backend mock scan.

## Настройки и IPC

`operation_api` расширен закрытыми запросами `get_preferences` и `save_preferences {expected_revision, preferences}`; ответ `preferences` содержит `schema_version`, `revision`, `preferences`. Rust contracts.rs остаётся источником generated TS. Theme: system/light/dark; language: ru. Другие языки не заявляются как доступные. Исключения ограничивают область будущего анализа, не дают разрешения на удаление.

React Query хранит подтверждённый snapshot; локальный UI state — React state. Автоматических retry и зависимости от network-online нет. Controls блокируются во время загрузки/сохранения; тема/список меняются после ответа backend. Ошибки storage, конфликта, версии, пути и transport видимы; введённый путь сохраняется при ошибке. После timeout результат записи неизвестен: интерфейс предлагает перечитать настройки. Удаление исключения требует отдельного подтверждения, поскольку расширяет область будущего анализа.

Production storage — `app_config_dir()/settings.json`, для текущего identifier на Windows обычно `%APPDATA%/local.pccare.desktop/settings.json`. UI не выбирает каталог/имя хранения. Чтение отсутствующего config возвращает defaults без создания каталогов/файлов. Изменение настроек — единственный новый дисковый эффект; доступа UI к filesystem/shell/registry и новых capabilities нет.

## Формат и защита записи

- Версия 1: `{schema_version:1, revision:UUID, preferences:{theme, language, exclusions}}`; максимум 256 KiB и 128 исключений.
- Поддерживаемый migration fixture версии 0: `{schema_version:0, theme, exclusions?}`. При чтении добавляется ru, revision nil; исходный файл не переписывается до явного сохранения. Старой опубликованной версии приложения с этим форматом пока нет.
- Неизвестная будущая версия/повреждение/неизвестные поля не сбрасываются и не перезаписываются. Необходимы исправление файла владельцем или совместимая версия приложения; скрытого reset нет.
- Исключения проходят существующий backend LocalPath/Exclusions: абсолютные DOS paths, без traversal/UNC/ADS, сравнение с учётом Windows casing, дедупликация. Пути могут не существовать: добавление исключения не сканирует их.
- Mutex и межпроцессный file lock сериализуют сотрудничающие экземпляры. Revision check предотвращает потерю обновления от устаревшего UI. Оставшийся lock-файл не означает занятый lock.
- Предки config удерживаются открытыми без share-delete; reparse/cloud/необычные файлы и hardlinks отклоняются. JSON записывается в новый UUID temporary file в том же каталоге, flush/sync, затем atomic rename. При неудаче старый JSON сохраняется, ошибка возвращается; свой incomplete temp может остаться для диагностики. Посторонние файлы не удаляются.

Это защита app-owned настроек, не универсальная гарантия от процесса с теми же/большими правами, произвольно меняющего файлы. Изменения пользовательских/системных данных приложением пока не реализованы.

## Проверка

Выполнены check/lint, 14 frontend tests, 20 browser smoke (light/dark/compact/DPR 1.25/1.5), fmt/clippy/33 Rust tests, generated TS drift, web/release build и обычный native smoke. Дополнительный native settings smoke проверяет сохранение через настоящий IPC, offline, полный перезапуск EXE с новым WebView profile, отмену/подтверждение удаления исключения и сохранность повреждённого JSON. CSP/JS errors считаются провалом.

```powershell
pnpm build
pnpm test:native
pnpm build:qa
pnpm test:settings
```

`build:qa` изолирует Cargo output в `target/native-qa`, не заменяя production EXE. Feature `qa-fixtures` выключена по умолчанию. QA host принимает `PC_CARE_FIXTURE_ROOT` только как непосредственный UUID child внутри compiled-workspace `target/native-fixtures`; при отсутствии/неверном значении storage unavailable, без fallback к пользовательским настройкам. Тест создаёт там только свои искусственные файлы. Этот EXE нельзя публиковать как обычный релиз.

Rust fixtures также проверяют миграцию, bad/future JSON, лимиты, ошибки, старые revisions/concurrent saves, lock, отказ atomic replacement и реальный junction/hardlink. Никакие реальные пользовательские файлы/registry/startup не изменяются тестами.

Галерея: [native WebView2](screenshots/spe-57/README.md). Чистые Windows 10/11 VM и настоящие OS scaling 125/150% ещё не проверены. Browser DPR не заменяет их. Vite сообщает размер общего JS chunk ~519 kB (159 kB gzip); отдельного performance budget/замера cold start пока нет. EXE unsigned, installer не опубликован. Статус задачи — In Review после PR, без Done до приёмки владельцем.
