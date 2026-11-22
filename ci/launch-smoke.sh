#!/usr/bin/env bash
# تشغيل حقيقي لـ Minecraft لأي loader/إصدار تحت شاشة افتراضية + Mesa software GL.
# معايير النجاح (كلها موضوعية): (1) Java من مجلد بيانات اللانشر نفسه (Runtime تلقائي) (2) ظهرت نافذة Minecraft
# (3) العملية بقيت حيّة بعد التحميل (4) لقطة الشاشة فيها محتوى فعلي لا شاشة فارغة (5) لا crash في السجل.
# الاستخدام: ci/launch-smoke.sh <loader|vanilla> <mc-version> [max_seconds]
set -u
LOADER="$1"; VER="$2"; MAXW="${3:-240}"
ROOT="$(pwd)"
export HRC_DATA="${RUNNER_TEMP:-/tmp}/hrc-$LOADER-$VER"
[ "$LOADER" = vanilla ] && unset HRC_LOADER || export HRC_LOADER="$LOADER"
OUT="$ROOT/ci-out"; mkdir -p "$OUT"
LOG="$OUT/$LOADER-$VER.log"; SHOT="$OUT/$LOADER-$VER.png"
export LIBGL_ALWAYS_SOFTWARE=1 DISPLAY=:99 MESA_GL_VERSION_OVERRIDE=4.5 MESA_GLSL_VERSION_OVERRIDE=450

start_server() {
  case "$1" in
    xvfb) Xvfb :99 -screen 0 1280x720x24 +extension GLX +extension RANDR +render -noreset >"$OUT/xserver-$1.log" 2>&1 & ;;
    xorg) sudo Xorg :99 -config "$ROOT/ci/xorg-dummy.conf" -noreset -ac -nolisten tcp -novtswitch -sharevts -logfile "$OUT/xorg.log" >"$OUT/xserver-$1.log" 2>&1 & ;;
  esac
  SRV=$!
  for _ in $(seq 20); do xdpyinfo >/dev/null 2>&1 && return 0; sleep 1; done
  return 1
}
stop_all() { pkill -f "net.minecraft" 2>/dev/null; pkill -f "bootstraplauncher|cpw.mods|net.minecraftforge|net.neoforged|KnotClient|QuiltLoader" 2>/dev/null; sleep 2; kill $GP 2>/dev/null; sudo pkill -f "Xorg :99" 2>/dev/null; kill $SRV 2>/dev/null; pkill -f hrc-cli 2>/dev/null; true; }

# مود اختياري يُثبَّت قبل التشغيل (من Modrinth) لإثبات أنه يُحمَّل داخل هذا الـ instance فقط
if [ -n "${SMOKE_MOD:-}" ]; then
  "$ROOT/target/release/hrc-cli" install "$VER" > "$OUT/$LOADER-$VER-install.log" 2>&1 || { echo "FAIL install"; tail -5 "$OUT/$LOADER-$VER-install.log"; exit 1; }
  "$ROOT/target/release/hrc-cli" mod "$VER" "$SMOKE_MOD" | tee "$OUT/$LOADER-$VER-mod.log"
fi
window_found=0
for server in xvfb xorg; do
  command -v "$(case $server in xorg) echo Xorg;; *) echo Xvfb;; esac)" >/dev/null || continue
  echo "=== attempt with $server ==="
  start_server "$server" || { echo "$server did not start"; kill $SRV 2>/dev/null; continue; }
  "$ROOT/target/release/hrc-cli" launch "$VER" Player 2048 > "$LOG" 2>&1 &
  GP=$!
  for i in $(seq 1 $((MAXW/5))); do
    sleep 5
    kill -0 $GP 2>/dev/null || break                       # اللعبة أنهت نفسها (crash أو خروج)
    if xwininfo -root -tree 2>/dev/null | grep -qi "minecraft"; then window_found=1; break; fi
  done
  if [ $window_found = 1 ]; then
    sleep 45                                               # وقت لإكمال التحميل على software GL
    import -window root "$SHOT" 2>"$OUT/import.log" || true
    ALIVE=0; kill -0 $GP 2>/dev/null && ALIVE=1
    cp "$LOG" "$OUT/$LOADER-$VER-$server.log"
    stop_all
    break
  fi
  cp "$LOG" "$OUT/$LOADER-$VER-$server.log"
  echo "$server: no Minecraft window"; grep -aE "Exception|Error|crash" "$LOG" | head -5
  stop_all; sleep 2
done

echo "---- log tail ----"; tail -n 40 "$LOG"
echo "---- checks ----"
ok=1
JL=$(grep -a "^Java runtime:" "$LOG" | head -1 | sed 's/^Java runtime: //')
case "$JL" in "$HRC_DATA"/*) echo "PASS java runtime auto-installed under launcher data ($JL)";; *) echo "FAIL java runtime path: '$JL'"; ok=0;; esac
[ $window_found = 1 ] && echo "PASS minecraft window appeared" || { echo "FAIL no minecraft window"; ok=0; }
[ "${ALIVE:-0}" = 1 ] && echo "PASS game still running after load" || { echo "FAIL game not running after load"; ok=0; }
if [ -s "$SHOT" ]; then
  COLORS=$(convert "$SHOT" -format %k info:)
  [ "${COLORS:-0}" -ge 100 ] && echo "PASS screenshot has content ($COLORS colors)" || { echo "FAIL screenshot looks empty ($COLORS colors)"; ok=0; }
else echo "FAIL no screenshot"; ok=0; fi
grep -qaE "Game crashed|This crash report|Exception in thread \"main\"|Failed to start|Could not create the Java Virtual Machine|Unrecognized option" "$LOG" && { echo "FAIL crash/startup error in log"; ok=0; } || echo "PASS no crash markers"
if [ -n "${SMOKE_MOD:-}" ]; then
  grep -qai "$SMOKE_MOD" "$LOG" && echo "PASS mod '$SMOKE_MOD' loaded by the game" || { echo "FAIL mod '$SMOKE_MOD' not seen in game log"; ok=0; }
fi
if [ -n "${SMOKE_ABSENT:-}" ]; then
  grep -qai "$SMOKE_ABSENT" "$LOG" && { echo "FAIL '$SMOKE_ABSENT' leaked into this instance"; ok=0; } || echo "PASS '$SMOKE_ABSENT' absent from this instance"
fi
grep -aE "Created: [0-9]+x[0-9]+|Sound engine started|Backend library|Reloading ResourceManager" "$LOG" | head -3 | sed 's/^/  info: /'
[ "$ok" = 1 ]
