# Сборка QIDIR

## Требования

| Компонент | Версия |
|---|---|
| Rust (rustup, stable) | ≥ 1.80 |
| Node.js | ≥ 20 (в CI используется 22) |
| Windows | Visual Studio Build Tools 2022 с рабочей нагрузкой «Разработка классических приложений на C++», WebView2 Runtime |
| Linux (только для разработки UI и тестов ядра) | `libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf` |

## Ядро и CLI (любая ОС)

```bash
cargo test --workspace --exclude qidir-desktop
cargo clippy -p qidir-core -p qidir-cli --all-targets -- -D warnings
cargo run -p qidir-cli -- --help
```

## Приложение

```bash
cd apps/desktop
npm ci
npm run check        # svelte-check
npm run test         # vitest
npm run build        # dist/
npm run tauri dev    # окно приложения с горячей перезагрузкой UI
npm run tauri build  # установщики
```

Результат `tauri build` на Windows:

```text
target/release/bundle/nsis/QIDIR_1.0.0_x64-setup.exe
target/release/bundle/msi/QIDIR_1.0.0_x64_ru-RU.msi
target/release/qidir-desktop.exe
```

Утилита командной строки: `cargo build --release -p qidir-cli` → `target/release/qidir.exe`.

## Фронтенд без Tauri

`npm run dev` открывает интерфейс в браузере с мок-бэкендом (реалистичные данные на трёх языках, имитация индексации). Параметры URL: `?theme=dark`, `?lang=kk`, `?empty=1` (первый запуск). `npm run screenshots` делает скриншоты основных экранов в headless Chromium.

## Проверка сборки Windows без Windows

Крейт приложения можно проверить типами под целевую платформу:

```bash
rustup target add x86_64-pc-windows-msvc
cargo check -p qidir-desktop --target x86_64-pc-windows-msvc
```

Полная сборка установщиков делается на Windows — локально или в GitHub Actions (`.github/workflows/ci.yml`, артефакт `QIDIR-windows-installers`).

## Релиз

1. Обновите версию в `Cargo.toml` (workspace), `apps/desktop/src-tauri/tauri.conf.json`, `apps/desktop/package.json` и `CHANGELOG.md`.
2. Создайте тег `vX.Y.Z` и отправьте его: `git tag v1.0.0 && git push origin v1.0.0`.
3. Workflow `release.yml` соберёт установщики и создаст черновик релиза с ними.

### Подпись кода

Установщики не подписаны по умолчанию; SmartScreen может показать предупреждение при первом запуске. Для подписи задайте в настройках Tauri (`bundle.windows.certificateThumbprint`, `timestampUrl`) сертификат, установленный на сборочной машине, либо используйте внешний подписыватель через `bundle.windows.signCommand`.

## Устранение неполадок

- **«Индекс QIDIR уже используется другим процессом»** — запущена вторая копия приложения или `qidir.exe` работает с тем же `--data-dir`. Закройте её.
- **Пустые результаты после обновления** — при смене версии схемы индекс пересобирается автоматически; дождитесь окончания индексирования.
- **Логи**: `%LOCALAPPDATA%\QIDIR\logs\`. Уровень задаётся переменной `RUST_LOG` (например, `RUST_LOG=qidir_core=debug`).
