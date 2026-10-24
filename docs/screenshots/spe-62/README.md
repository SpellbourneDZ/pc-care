# SPE-62: визуальная проверка

Все изображения показывают **искусственные файлы**; метка происхождения присутствует на экране и в overlays. Это не результаты сканирования пользовательских каталогов.

| Сценарий                                       | Evidence                                                                                                       |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Настоящий QA EXE / Windows WebView2 / 1280×720 | [Результат](native-ready.png), [список файлов](native-drawer.png), [backend preview](native-preview.png)       |
| Browser IPC fixture / dark / 1920×1080         | [Категории](dark-ready.png), [предпросмотр](dark-preview.png)                                                  |
| Browser IPC fixture / 720×520                  | [Preview с доступными кнопками и отдельным scroll](compact-preview.png), [длинный путь](compact-long-path.png) |
| Browser IPC fixture / light / 1280×720         | [Partial/truncated](partial.png), [ноль находок](empty.png), [ошибка](error.png)                               |
| Browser IPC fixture / scale 150% / dark        | [Категории](scale-150.png)                                                                                     |

Визуально проверены native ready/preview, light/dark desktop, compact preview и длинный путь. После обнаружения обрезанной CTA в 720×520 footer отделён от прокручиваемого содержимого, а у списка устранено сжатие grid-строки; сценарий повторно пройден. Browser tests проверяют оба device scale 125/150%, modal focus и закрытие с клавиатуры. Это не проверка системного DPI Windows; VM matrix остаётся открытой.
