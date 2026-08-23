# whipnext

[English](README.en.md)

## Что это

Знакомо иногда писать «продолжай» в Claude Code? Вот это приложение про это.

whipnext — маленький desktop-оверлей над coding harnesses. Вы выбираете персонажа, а по клику он бьёт по экрану, проигрывает звук и отправляет фразу в активную сессию. Можно задать аниме-девочку, которая будет бить плёткой ваш Codex и говорить: «дальше, мальчик».

Поддерживаются Claude Code, Codex CLI, Antigravity, Grok Build, OpenCode и другие обнаруживаемые harnesses. Для каждого можно выбрать свою модель, звук и фразу.

<table>
<tr>
<td align="center">
<img src="docs/claude-code-overlay.png" width="520" alt="Аниме-девушка whipnext в Claude Code">
<br><sub>whipnext: аниме-девушка с плёткой</sub>
</td>
<td align="center">
<a href="https://x.com/blended_jpeg/status/2041108141266653325"><img src="docs/bad-claude-tweet.png" width="520" alt="Оригинальный твит BadClaude с плёткой над Claude Code"></a>
<br><sub><a href="https://x.com/blended_jpeg/status/2041108141266653325">Оригинальный твит @blended_jpeg</a></sub>
</td>
</tr>
</table>

## Как это работает

1. Приложение определяет окно и процесс в фокусе.
2. Находит активный harness и показывает назначенного персонажа.
3. Клик запускает анимацию и звук.
4. Фраза отправляется в текущую coding-сессию.

Фраза по умолчанию — `next`. Это не slash-команд launcher.

## История и вдохновение

Это не попытка решить большую инженерную задачу. Это способ сжечь лимиты Codex перед ресетом, пока он не начнёт писать «готово» раньше времени.

> Тупая поделка выходного дня, сделанная по промпту: «Сделай приложение на расте которое будет детектить харнессы и позволять создавать персонажей, которые будут бить по экрану и передавать команды по клику».

Идея выросла из вирусной шутки с цифровым кнутом для Claude Code:

- [Видео на YouTube про BadClaude](https://www.youtube.com/watch?v=6JnbXtbsKeg)
- [Pull requests OpenWhip](https://github.com/GitFrog1111/OpenWhip/pulls)

[![Открыть видео про цифровой кнут для Claude Code](https://img.youtube.com/vi/6JnbXtbsKeg/hqdefault.jpg)](https://www.youtube.com/watch?v=6JnbXtbsKeg)

Это внешние материалы и не являются частью whipnext.

## Запуск

Нужны [Rust](https://rustup.rs/), Cargo и `just`. Node.js + npm нужны только для проверки settings UI; `ffmpeg` — для видео-персонажей.

```text
just build
target/release/whipnext
```

Флаги:

```text
whipnext                 # окно настроек + оверлей
whipnext --detect        # JSON установленных/запущенных агентов
whipnext --demo          # только оверлей, без инжекта
```

Настройки живут в `%USERPROFILE%\.whipnext\settings.json` на Windows и `~/.whipnext/settings.json` на macOS/Linux.

## Платформы

- Windows: Win32-ввод и WebView2.
- macOS: ввод через CoreGraphics. Выдай приложению доступ Accessibility: System Settings → Privacy & Security → Accessibility.
- Linux: сейчас поддерживается X11. Нужны `xdotool`, `ffmpeg`, GTK 3 и WebKitGTK 4.1; Wayland пока не поддержан текущим backend.

Сборка пакетов: `bash scripts/package_linux.sh` или `bash scripts/package_macos.sh`.

## Персонажи

| id | кто | клик |
|---|---|---|
| `default` | девушка с плёткой | удар |
| `commissar` | комиссар | удар хлыстом |
| `cat` | котик | лапка |
| `parrot` | попугай | «уже готово?» |
| `capybara` | капибара | лапка |
| `feather` | перо | лапка |
| `lash` | плеть | удар хлыстом |
| `manager` | эффективный менеджер | готово |

Файлы персонажей лежат в `assets/pack/models/<id>/`, звуки — в `assets/pack/sounds/`.

## Разработка

```text
just setup
just check
just detect       # опциональный live smoke test
```

Тесты запускаются последовательно: часть проверок временно подменяет process-wide переменные окружения. CI собирает и тестирует Windows, Linux/X11 и macOS.

## Снимок языков репозитория

Исторический снимок GitHub до очистки служебных скриптов:

![Снимок языков репозитория](docs/language-stats.png)

## Участие и безопасность

- [CONTRIBUTING.md](CONTRIBUTING.md)
- [SECURITY.md](SECURITY.md)
- [LICENSE](LICENSE)
- [Ветки и правила разработки](docs/branching.md)

## GIF-демо

![GIF-демо whipnext](docs/demo.gif)
