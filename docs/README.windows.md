# Разработка PC Care в Windows

Цель MVP: Windows 10/11 x64. Для запуска приложения не нужны права администратора. Установка инструментов разработки может требовать UAC; это отдельный шаг, а не требование к PC Care.

## Инструменты

1. Node.js **24.22.1** (24.x), pnpm **10.34.8**. Проверить `node --version` и `pnpm --version`.
2. Microsoft Visual Studio 2022 Build Tools: workload **Desktop development with C++**, MSVC x64/x86 и Windows SDK.
3. Rust через rustup, host **x86_64-pc-windows-msvc**. `rust-toolchain.toml` фиксирует стабильный релиз **1.99.1**, rustfmt и clippy.
4. Microsoft Edge WebView2 Evergreen Runtime. Browser smoke использует установленный Microsoft Edge.

Официальные инструкции: [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), [Node.js](https://nodejs.org/en/download), [pnpm](https://pnpm.io/installation).

После установки Node 24:

```powershell
npm install --global pnpm@10.34.8
```

Rust/MSVC можно установить из официальных установщиков или winget:

```powershell
winget install --id Rustlang.Rustup --exact --source winget
winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --source winget --override "--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

Откройте новую консоль после установки. Если cargo отсутствует в PATH (в сессии SPE-55 rustup установлен с `--no-modify-path`), добавьте его для текущего процесса:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
rustc --version
cargo --version
```

## Установка зависимостей и запуск

Склонируйте [GitHub-репозиторий](https://github.com/SpellbourneDZ/pc-care) и перейдите в его корень. До слияния SPE-55 код находится в ветке codex/spe-55-monorepo.

```powershell
pnpm install --frozen-lockfile
pnpm dev
```

`pnpm dev` запускает Vite на `127.0.0.1:1420` и нативное окно Tauri. При первом запуске Cargo скачивает/собирает зависимости. Порт должен быть свободен: он фиксирован, чтобы Tauri не открыл посторонний адрес.

Frontend отдельно, без Rust:

```powershell
pnpm dev:web
```

Приложение не обращается в сеть в runtime. Первичная установка инструментов, pnpm/Cargo dependencies и CI требуют сети.

## Проверки

```powershell
pnpm check
pnpm lint
pnpm test
pnpm format:check
pnpm build:web
pnpm test:smoke
pnpm rust:check
pnpm build
pnpm test:native
pnpm build:qa
pnpm test:settings
```

- `build:web` проверяет TS strict и собирает frontend в `apps/desktop/dist`.
- `rust:check` выполняет cargo fmt, clippy с warnings-as-errors и cargo test, используя корневой Cargo workspace.
- `build` создаёт unsigned `target/release/pc-care.exe`, без установщика и публикации.
- `test:smoke` поднимает Vite и проверяет light/dark/минимальное окно в Edge, offline после загрузки и клавиатуру.
- `test:native` запускает **release EXE** в WebView2 с отдельным профилем под `test-results`, отключает сеть и проверяет кнопку/отрисовку. CDP включён только через environment дочернего процесса, loopback-порт выбирается тестом; настройка приложения его не включает. Тест закрывает только запущенный им процесс и его дерево.
- Скриншоты тестов — `apps/desktop/test-results`; согласованные примеры для ревью — `docs/screenshots`.
- Эти проверки не сканируют и не изменяют пользовательские файлы, реестр или автозагрузку. Системные тесты будущих функций — только fixtures/VM.

## UI и границы

Следуем [официальной инструкции shadcn для Vite](https://ui.shadcn.com/docs/installation/vite): Tailwind Vite plugin, alias `@/*`, `components.json`, локальные компоненты `src/ui`. Источник и изменения описаны в [UI foundation](ui-foundation.md).

Системных plugins нет; main имеет только allow-operation-api для закрытого typed IPC. Mock scan доступен в debug, release возвращает unsupported. Исполнение планов пока только симуляция, без Windows effects. Production CSP разрешает только локальные ресурсы; dev CSP отдельно допускает локальный HMR. Предварительный app identifier `local.pccare.desktop` не является утверждённым публичным именем. Контракт и генерация типов: [SPE-58 API](SPE-58-api.md).

Обычный build не включает bundling. `pnpm build:installer` создаёт черновой NSIS current-user, `pnpm test:installer` проверяет PE/размеры и пишет SHA256 manifest без запуска installer. WebView2 в draft не устанавливается и должен быть уже доступен. Подробности, ограничения VM и настройки branch protection — [SPE-60](SPE-60-testing.md). Подпись и полноценный release workflow — SPE-83.

## Неполадки

- `Unsupported engine`: убедитесь, что консоль использует Node 24 и pnpm 10.34.8, а не старые системные версии.
- `link.exe not found` / отсутствуют SDK libraries: проверьте C++ workload и SDK в Visual Studio Installer; перезапустите консоль.
- Порт 1420 занят: завершите собственный предыдущий dev-сервер; Vite намеренно не выбирает другой порт.
- Registry timeout при shadcn: CLI использует HTTPS_PROXY/HTTP_PROXY, а не автоматически прокси Windows. При наличии системного прокси передайте его в окружении процесса. Не сохраняйте адреса/учётные данные прокси в Git.
- Для native smoke не направляйте loopback CDP через HTTP_PROXY/HTTPS_PROXY: ограничьте proxy процессом build или добавьте `127.0.0.1,localhost` к `NO_PROXY`. Иначе соединение Playwright с локальным WebView2 может завершиться ECONNRESET.
- Ошибка запуска WebView2: проверьте Evergreen Runtime; UI тест нельзя считать пройденным только по успешной компиляции.

Лицензия MIT находится в корне репозитория; installer/release не публикуется на этапе foundation.
