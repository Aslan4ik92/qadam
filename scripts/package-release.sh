#!/usr/bin/env bash
# Assemble the `release/` folder with ready-to-run Windows x64 binaries.
#
# Usage:
#   scripts/package-release.sh [TARGET_DIR]
# TARGET_DIR defaults to target-win/x86_64-pc-windows-gnu/release (the
# cross-compiled GNU build) and falls back to target/x86_64-pc-windows-msvc/release
# and target/release (native Windows build).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="$(grep -m1 '^version' "$ROOT/Cargo.toml" | sed -E 's/.*"([^"]+)".*/\1/')"
OUT="$ROOT/release"

pick() {
  for d in "$@"; do
    if [ -f "$d/qidir-desktop.exe" ]; then echo "$d"; return; fi
  done
  echo "no qidir-desktop.exe found in: $*" >&2; exit 1
}
BIN_DIR="${1:-$(pick "$ROOT/target-win/x86_64-pc-windows-gnu/release" "$ROOT/target/x86_64-pc-windows-msvc/release" "$ROOT/target/release")}"

rm -rf "$OUT"
mkdir -p "$OUT/QIDIR-portable"
cp "$BIN_DIR/qidir-desktop.exe" "$OUT/QIDIR-portable/QIDIR.exe"
cp "$BIN_DIR/qidir.exe"         "$OUT/QIDIR-portable/qidir.exe"
cp "$ROOT/LICENSE"              "$OUT/QIDIR-portable/LICENSE.txt"
cp "$ROOT/docs/SEARCH_SYNTAX.md" "$OUT/QIDIR-portable/SYNTAX.md"

# Installers, when present (built by `tauri build`).
for f in "$ROOT"/target*/*/release/bundle/nsis/*.exe "$ROOT"/target*/release/bundle/nsis/*.exe \
         "$ROOT"/target*/*/release/bundle/msi/*.msi  "$ROOT"/target*/release/bundle/msi/*.msi; do
  [ -f "$f" ] && cp "$f" "$OUT/"
done

cat > "$OUT/QIDIR-portable/README.txt" <<TXT
QIDIR $VERSION — портативная версия для Windows 10/11 x64
=========================================================

Запуск
------
1. Распакуйте папку QIDIR-portable в любое место (например, C:\QIDIR).
2. Запустите QIDIR.exe. Установка не требуется, прав администратора не нужно.
3. При первом запуске нажмите «Добавить папку…» и выберите папки или диски
   для индексирования. Поиск доступен уже во время индексирования.

Требования
----------
* Windows 10 (1809+) или Windows 11, 64-bit.
* Среда выполнения Microsoft Edge WebView2. В Windows 11 она есть всегда.
  В Windows 10 обычно установлена вместе с Microsoft Edge; если QIDIR.exe
  сообщает об отсутствии WebView2, установите её отсюда:
  https://developer.microsoft.com/microsoft-edge/webview2/#download
  (Evergreen Bootstrapper, ~2 МБ) — или используйте установщик
  QIDIR_${VERSION}_x64-setup.exe, который делает это автоматически.
* Никакие другие компоненты (Java, .NET, Python, Visual C++ Redistributable)
  не нужны: EXE полностью самодостаточный.

Первый запуск и SmartScreen
---------------------------
EXE не подписан сертификатом, поэтому Windows может показать окно
«Система Windows защитила ваш компьютер». Нажмите «Подробнее» →
«Выполнить в любом случае».

Где хранятся данные
-------------------
Индекс и настройки: %LOCALAPPDATA%\QIDIR
Логи:               %LOCALAPPDATA%\QIDIR\logs
Ничего не отправляется в интернет.

Командная строка
----------------
qidir.exe — тот же движок без окна. Примеры:
  qidir.exe add-root D:\Документы
  qidir.exe index
  qidir.exe search "договор аренды"
  qidir.exe --help
Не запускайте qidir.exe одновременно с QIDIR.exe над одним индексом.

Удаление
--------
Удалите папку QIDIR-portable и, при желании, %LOCALAPPDATA%\QIDIR.
TXT

(
  cd "$OUT"
  rm -f "QIDIR-${VERSION}-windows-x64-portable.zip"
  zip -q -r "QIDIR-${VERSION}-windows-x64-portable.zip" QIDIR-portable
  sha256sum QIDIR-portable/*.exe *.zip *.exe *.msi 2>/dev/null > SHA256SUMS.txt || true
)
echo "release folder ready:"
ls -la "$OUT" "$OUT/QIDIR-portable"
