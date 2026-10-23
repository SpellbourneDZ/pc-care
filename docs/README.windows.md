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

Склонируйте согласованный GitHub-репозиторий и перейдите в его корень. До публикации используйте текущую локальную папку проекта. URL ещё не утверждён.

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

Нет пользовательских IPC-команд и системных plugins; у окна пустой список permissions. Production CSP разрешает только локальные ресурсы; dev CSP отдельно допускает локальный HMR. Предварительный app identifier `local.pccare.desktop` не является утверждённым публичным именем.

NSIS current-user намечен конфигурацией, bundling выключен. Установщик, подпись, clean Windows 10/11 VM и merge protection относятся к последующим задачам/настройке GitHub. Файл CI подготовлен, но удалённый запуск и branch protection нельзя подтвердить без remote.

## Неполадки

- `Unsupported engine`: убедитесь, что консоль использует Node 24 и pnpm 10.34.8, а не старые системные версии.
- `link.exe not found` / отсутствуют SDK libraries: проверьте C++ workload и SDK в Visual Studio Installer; перезапустите консоль.
- Порт 1420 занят: завершите собственный предыдущий dev-сервер; Vite намеренно не выбирает другой порт.
- Registry timeout при shadcn: CLI использует HTTPS_PROXY/HTTP_PROXY, а не автоматически прокси Windows. При наличии системного прокси передайте его в окружении процесса. Не сохраняйте адреса/учётные данные прокси в Git.
- Ошибка запуска WebView2: проверьте Evergreen Runtime; UI тест нельзя считать пройденным только по успешной компиляции.

Лицензия ожидает выбора владельца. MIT пока только предложение; installer/release не публикуется.
