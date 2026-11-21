#!/usr/bin/env bash
# تشغيل حقيقي لـ Minecraft تحت شاشة افتراضية + Mesa software GL مع Horizon Client، ثم لقطة وسجل.
# LWJGL 2 يفشل على Xvfb العادي (لا أوضاع RandR)، لذا نجرّب عدة خوادم عرض بالترتيب.
# الاستخدام: ci/game-smoke.sh <version> [max_seconds_per_attempt]
set -u
VER="$1"; MAXW="${2:-100}"
ROOT="$(pwd)"
export HRC_DATA="${RUNNER_TEMP:-/tmp}/hrc-$VER"
GAME="$HRC_DATA/instances/cli-$VER/game"
OUT="$ROOT/ci-out"; mkdir -p "$OUT" "$GAME"
cat > "$GAME/horizon-client.properties" <<EOP
fps.enabled=true
cps.enabled=true
keystrokes.enabled=true
EOP
export HRC_EXTRA_JVM="-javaagent:$ROOT/client/target/horizon-client.jar=$GAME"
export LIBGL_ALWAYS_SOFTWARE=1 DISPLAY=:99
LOG="$OUT/$VER.log"; SHOT="$OUT/$VER.png"

start_server() {
  case "$1" in
    xvfb) Xvfb :99 -screen 0 1280x720x24 +extension GLX +extension RANDR +render -noreset >"$OUT/xserver-$1.log" 2>&1 & ;;
    xorg) sudo Xorg :99 -config "$ROOT/ci/xorg-dummy.conf" -noreset -ac -nolisten tcp -novtswitch -sharevts -logfile "$OUT/xorg.log" >"$OUT/xserver-$1.log" 2>&1 & ;;
    xvnc) Xvnc :99 -geometry 1280x720 -depth 24 -SecurityTypes None +extension GLX >"$OUT/xserver-$1.log" 2>&1 & ;;
  esac
  SRV=$!
  for _ in $(seq 20); do xdpyinfo >/dev/null 2>&1 && return 0; sleep 1; done
  return 1
}

reached=0
for server in xorg xvfb xvnc; do
  command -v "$(case $server in xorg) echo Xorg;; xvfb) echo Xvfb;; *) echo Xvnc;; esac)" >/dev/null || { echo "skip $server (not installed)"; continue; }
  echo "=== attempt with $server ==="
  start_server "$server" || { echo "$server did not start"; kill $SRV 2>/dev/null; continue; }
  "$ROOT/target/release/hrc-cli" launch "$VER" Player 1536 > "$LOG" 2>&1 &
  GP=$!
  for i in $(seq 1 $((MAXW/5))); do
    sleep 5
    grep -q "first HUD frame drawn" "$LOG" && { reached=1; break; }
    grep -qE "Game crashed|Game exited|panicked" "$LOG" && break
  done
  if [ $reached = 1 ]; then
    sleep 12   # بضع ثوانٍ إضافية ليستقر القائمة الرئيسية ويظهر FPS
    import -window root "$SHOT" 2>"$OUT/import.log" || true
  fi
  cp "$LOG" "$OUT/$VER-$server.log"
  pkill -f "net.minecraft" 2>/dev/null; sleep 2; kill $GP 2>/dev/null; sudo pkill -f "Xorg :99" 2>/dev/null; kill $SRV 2>/dev/null; pkill -f hrc-cli 2>/dev/null
  [ $reached = 1 ] && { echo "$server: HUD frame drawn"; break; }
  echo "$server: did not reach HUD frame"; grep -aE "Exception|Error" "$LOG" | head -3
  sleep 2
done

echo "---- log tail ----"; tail -n 30 "$LOG"
echo "---- checks ----"
ok=1
grep -q "\[HorizonClient\] hooked LWJGL 2" "$LOG" && echo "PASS agent hooked" || { echo "FAIL agent hooked"; ok=0; }
grep -q "\[HorizonClient\] first HUD frame drawn" "$LOG" && echo "PASS hud frame drawn" || { echo "FAIL hud frame drawn"; ok=0; }
grep -q "overlay disabled after error" "$LOG" && { echo "FAIL overlay error"; ok=0; } || echo "PASS no overlay error"
[ -s "$SHOT" ] && echo "PASS screenshot" || { echo "FAIL screenshot"; ok=0; }
[ "$ok" = 1 ]
