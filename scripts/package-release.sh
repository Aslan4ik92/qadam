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
# The GNU (mingw) build links WebView2Loader dynamically; MSVC builds embed it.
if [ -f "$BIN_DIR/WebView2Loader.dll" ]; then
  cp "$BIN_DIR/WebView2Loader.dll" "$OUT/QIDIR-portable/WebView2Loader.dll"
fi
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
1. Распакуйте папку QIDIR-portable целиком в любое место (например, C:\QIDIR).
   Файлы QIDIR.exe и WebView2Loader.dll должны лежать рядом.
2. Запустите QIDIR.exe. Установка не требуется, прав администратора не нужно.
3. При первом запуске нажмите «Добавить папку…» и выберите папки или диски
   для индексирования. Поиск доступен уже во время индексирования.

Требования
----------
* Windows 10 (1809+) или Windows 11, 64-bit.
* Больше ничего: ни .NET, ни Java, ни Visual C++ Redistributable, ни WebView2.
  Если среда Microsoft Edge WebView2 на компьютере есть (Windows 11 — всегда),
  QIDIR открывается в собственном окне. Если её нет, QIDIR сам откроет тот же
  интерфейс во вкладке вашего браузера (встроенный сервер на 127.0.0.1).
  Принудительно открыть в браузере: QIDIR.exe --browser
* Копируйте папку целиком: WebView2Loader.dll должен лежать рядом с QIDIR.exe.

Первый запуск и SmartScreen
---------------------------
EXE не подписан сертификатом, поэтому Windows может показать окно
«Система Windows защитила ваш компьютер». Нажмите «Подробнее» →
«Выполнить в любом случае».

Если QIDIR.exe не запускается (ничего не происходит)
----------------------------------------------------
1. Правой кнопкой по QIDIR.exe → Свойства → галочка «Разблокировать» → OK
   (Windows блокирует файлы, распакованные из скачанного архива).
   В PowerShell в этой папке: Get-ChildItem | Unblock-File
2. Запустите QIDIR.exe --browser — интерфейс откроется в браузере
   (WebView2 не нужен). Адрес вида http://127.0.0.1:НОМЕР/ виден в консоли:
   QIDIR.exe --browser --console
3. Запустите QIDIR.exe --console из PowerShell: откроется консоль с журналом.
   Ошибки также показываются в диалоговом окне и пишутся в
   %LOCALAPPDATA%\QIDIR\logs.
4. Проверьте антивирус (карантин для неподписанных EXE).

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
  sha256sum QIDIR-portable/*.exe QIDIR-portable/*.dll *.zip *.exe *.msi 2>/dev/null > SHA256SUMS.txt || true
)
echo "release folder ready:"
ls -la "$OUT" "$OUT/QIDIR-portable"
