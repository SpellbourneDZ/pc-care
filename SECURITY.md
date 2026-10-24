# Security policy

PC Care — предварительная версия бесплатного offline-приложения. Поддерживаемый код сейчас — актуальная ветка разработки; опубликованной стабильной версии пока нет. Черновые CI artifacts не являются рекомендацией устанавливать приложение на основной ПК.

## Сообщить об уязвимости

Используйте GitHub **Security → Report a vulnerability** в [репозитории PC Care](https://github.com/SpellbourneDZ/pc-care/security). Не публикуйте приватные пути, дампы, содержимое дисков, токены или воспроизводимый способ потери данных в публичном issue. Приложите commit/version, Windows/WebView2 version и минимальный пример на искусственных файлах. Обещаний фиксированного срока ответа нет.

## Текущая граница безопасности

- Приложение запускается без администратора. UI имеет только закрытый типизированный `operation_api`; прямого filesystem/registry/shell API нет.
- Production CSP разрешает локальные ресурсы; runtime backend, аккаунтов и телеметрии нет. Загрузка зависимостей/инструментов во время build требует сети.
- Read-only анализ cleanup доступен через typed API с фиксированными roots и policy; рабочий экран подключается в SPE-62. Очистка/startup/uninstall не подключены. Debug mock scan и frontend prototype явно отделены; native cleanup execute возвращает unsupported, mock execution остаётся симуляцией с нулём освобождённых байтов.
- Единственная реализованная запись приложения — собственные локальные настройки с валидацией, schema/revision, lock и atomic replacement. Содержимое файлов пользователя не пишется в логи.
- Защитная политика backend default-deny и read-only revalidation не являются разрешением на удаление. Реальные effects потребуют SPE-63 и durable outcome recovery.

## Обязательный baseline для изменений

- [x] Protected roots/Windows/Program Files/ProgramData/другие пользователи — default-deny; Downloads, документы, browser secrets не входят в очистку по умолчанию.
- [x] Reparse/junction, hardlinks, cloud attributes, Unicode/casing/long paths, locks и ACL представлены искусственными Windows fixtures; неизвестные источники запрещены.
- [x] Закрытая serde-модель IPC, generated TS drift, минимальные capabilities; hostile extra fields отклоняются.
- [x] UI показывает ограничения и ошибки, без fake health/ускорения/освобождённых байтов. Settings меняются после backend acknowledgement.
- [x] Windows CI: frontend/Rust/native/settings checks, NSIS draft build и PE/hash smoke, без запуска installer или удаления данных.
- [x] `main` требует `windows` от GitHub Actions, strict/up-to-date, включая администраторов; force-push/delete запрещены. Самостоятельно merge не выполняется.
- [ ] Перед реальными мутациями: snapshot → selected stable IDs → backend plan → preview/confirmation → revalidation → per-item outcomes; устранить финальный check/use race.
- [ ] Recycle API без permanent fallback, recovery/history, source-specific backup и подтверждённые uninstall mechanisms — в соответствующих задачах.
- [ ] Clean Windows 10/11 VM с snapshots, installer install/upgrade/uninstall, настоящий DPI и destructive scenarios — [VM strategy](docs/SPE-60-testing.md), ещё не выполнены.
- [ ] Подпись/SmartScreen/release provenance и официальный релиз — SPE-83; не выдавать SHA256 за подпись или гарантию безопасности.

Проверки CI и branch protection уменьшают риск ошибки, но не являются sandbox для недоверенного кода или защитой от процесса с правами пользователя/администратора. Actions PR исполняются на одноразовых hosted runners с `contents: read`, без secrets и `pull_request_target`. Опасные системные операции разрешаются только отдельным подтверждённым сценарием, сначала dry-run и VM fixtures.
