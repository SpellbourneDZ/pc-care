# UI foundation — SPE-55

Это стартовая основа, а не полная дизайн-система SPE-56.

- React + TS strict, Tailwind 4 через официальный Vite plugin.
- shadcn/ui (Radix), официальный CLI, компонент Button хранится в src/ui. Alias и components.json находятся в app workspace.
- Иконки — Lucide; шрифт — системный Segoe UI, без сетевых загрузок.
- Семантические цвета, radius и общая типографика задаются src/index.css. Светлая/тёмная палитра следует системной теме; пользовательский выбор/сохранение — SPE-56/57.
- Базовый ритм 4/8/16/24/32 px, кнопки desktop-плотности, фокус через shadcn, reduced motion в CSS.
- Единственная интеракция текущего экрана — «О проекте», disclosure с aria-expanded/aria-controls, доступный Enter и Space. Это не имитация сканирования.
- Пока нет custom dialogs/scroll/overlays и полноценной sidebar/navigation. Их проверка WebView2 обязательна при реализации соответствующих компонентов.
- app-icon.svg — простой локальный значок для dev-сборки; финальный брендинг не утверждён.
