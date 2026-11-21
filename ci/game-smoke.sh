#!/usr/bin/env bash
# تشغيل حقيقي لـ Minecraft تحت شاشة افتراضية (Xvfb + Mesa software GL) مع Horizon Client،
# ثم لقطة شاشة وسجل. الاستخدام: ci/game-smoke.sh <version> [seconds]
set -u
VER="$1"; WAIT="${2:-100}"
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
export LIBGL_ALWAYS_SOFTWARE=1 MESA_GL_VERSION_OVERRIDE=4.5 SHOT="$OUT/$VER.png" LOG="$OUT/$VER.log" WAIT VER

xvfb-run -a -s "-screen 0 1280x720x24" bash -c '
  "$0/target/release/hrc-cli" launch "$VER" Player 1536 > "$LOG" 2>&1 &
  sleep "$WAIT"
  import -window root "$SHOT" 2>>"$LOG.import" || true
  pkill -f "net.minecraft" || true
  sleep 2
' "$ROOT"
pkill -f hrc-cli || true

echo "---- log tail ----"; tail -n 40 "$OUT/$VER.log"
echo "---- checks ----"
ok=1
grep -q "\[HorizonClient\] hooked LWJGL 2" "$OUT/$VER.log" && echo "PASS agent hooked" || { echo "FAIL agent hooked"; ok=0; }
grep -q "\[HorizonClient\] first HUD frame drawn" "$OUT/$VER.log" && echo "PASS hud frame drawn" || { echo "FAIL hud frame drawn"; ok=0; }
grep -q "overlay disabled after error" "$OUT/$VER.log" && { echo "FAIL overlay error"; ok=0; } || echo "PASS no overlay error"
[ -s "$OUT/$VER.png" ] && echo "PASS screenshot" || { echo "FAIL screenshot"; ok=0; }
[ "$ok" = 1 ]
