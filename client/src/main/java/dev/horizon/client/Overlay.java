package dev.horizon.client;

import org.lwjgl.input.Keyboard;
import org.lwjgl.input.Mouse;
import org.lwjgl.opengl.Display;
import org.lwjgl.opengl.GL11;

import java.util.ArrayDeque;

/**
 * طبقة HUD فوق الإطار (LWJGL 2: الإصدارات 1.8 – 1.12.2). كل مود مطفأ افتراضياً؛ وإن كانت
 * كلها مطفأة فلا يُنفَّذ أي أمر GL إطلاقاً (تكلفة صفر).
 */
final class Overlay {
    private TextRenderer text;
    private boolean loggedFirst;

    // FPS
    private long fpsWindowStart = System.nanoTime();
    private int fpsFrames, fpsValue;

    // CPS
    private final ArrayDeque<Long> leftClicks = new ArrayDeque<>();
    private final ArrayDeque<Long> rightClicks = new ArrayDeque<>();
    private boolean prevLeft, prevRight;

    void render() {
        Config.poll();
        boolean fps = Config.bool("fps.enabled", false);
        boolean cps = Config.bool("cps.enabled", false);
        boolean keys = Config.bool("keystrokes.enabled", false);
        if (!fps && !cps && !keys) return;

        int w = Display.getWidth(), h = Display.getHeight();
        if (w <= 0 || h <= 0) return;

        updateCounters(cps || keys);

        GL11.glPushAttrib(GL11.GL_ALL_ATTRIB_BITS);
        GL11.glMatrixMode(GL11.GL_PROJECTION);
        GL11.glPushMatrix();
        GL11.glLoadIdentity();
        GL11.glOrtho(0, w, h, 0, -1, 1);
        GL11.glMatrixMode(GL11.GL_MODELVIEW);
        GL11.glPushMatrix();
        GL11.glLoadIdentity();
        try {
            GL11.glDisable(GL11.GL_DEPTH_TEST);
            GL11.glDisable(GL11.GL_LIGHTING);
            GL11.glDisable(GL11.GL_CULL_FACE);
            GL11.glDisable(GL11.GL_FOG);
            GL11.glDisable(GL11.GL_ALPHA_TEST);
            GL11.glEnable(GL11.GL_BLEND);
            GL11.glBlendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
            GL11.glEnable(GL11.GL_TEXTURE_2D);
            if (text == null) text = new TextRenderer();

            float scale = Math.max(1, Config.num("hud.scale", Math.max(1, h / 480)));
            if (fps) drawFps(scale);
            if (cps) drawCps(scale);
            if (keys) drawKeystrokes(scale);
        } finally {
            GL11.glMatrixMode(GL11.GL_PROJECTION);
            GL11.glPopMatrix();
            GL11.glMatrixMode(GL11.GL_MODELVIEW);
            GL11.glPopMatrix();
            GL11.glPopAttrib();
        }
        if (!loggedFirst) {
            loggedFirst = true;
            Log.info("first HUD frame drawn (" + w + "x" + h + ", glError=" + GL11.glGetError() + ")");
        }
    }

    private void updateCounters(boolean needClicks) {
        long now = System.nanoTime();
        fpsFrames++;
        if (now - fpsWindowStart >= 500_000_000L) {
            fpsValue = Math.round(fpsFrames * 1e9f / (now - fpsWindowStart));
            fpsFrames = 0;
            fpsWindowStart = now;
        }
        if (needClicks && Mouse.isCreated()) {
            long ms = System.currentTimeMillis();
            boolean l = Mouse.isButtonDown(0), r = Mouse.isButtonDown(1);
            if (l && !prevLeft) leftClicks.addLast(ms);
            if (r && !prevRight) rightClicks.addLast(ms);
            prevLeft = l;
            prevRight = r;
            prune(leftClicks, ms);
            prune(rightClicks, ms);
        }
    }

    private static void prune(ArrayDeque<Long> q, long now) {
        while (!q.isEmpty() && now - q.peekFirst() > 1000) q.removeFirst();
    }

    private void drawFps(float s) {
        label("FPS: " + fpsValue, Config.num("fps.x", 6), Config.num("fps.y", 6), s);
    }

    private void drawCps(float s) {
        label("CPS: " + leftClicks.size() + " | " + rightClicks.size(),
                Config.num("cps.x", 6), Config.num("cps.y", 26), s);
    }

    /** مربع نصي بخلفية شفافة. */
    private void label(String str, int x, int y, float s) {
        float pad = 4 * s, w = text.width(str, s) + pad * 2, h = text.height(s) + pad * 2;
        TextRenderer.rect(x * s, y * s, w, h, 0x66000000);
        text.draw(str, x * s + pad, y * s + pad * 0.6f, s, 0xFFFFFFFF);
    }

    private void drawKeystrokes(float s) {
        float ox = Config.num("keystrokes.x", 6) * s, oy = Config.num("keystrokes.y", 50) * s;
        float k = 26 * s, gap = 3 * s;
        key("W", Keyboard.isCreated() && Keyboard.isKeyDown(Keyboard.KEY_W), ox + k + gap, oy, k, k, s);
        key("A", Keyboard.isCreated() && Keyboard.isKeyDown(Keyboard.KEY_A), ox, oy + k + gap, k, k, s);
        key("S", Keyboard.isCreated() && Keyboard.isKeyDown(Keyboard.KEY_S), ox + k + gap, oy + k + gap, k, k, s);
        key("D", Keyboard.isCreated() && Keyboard.isKeyDown(Keyboard.KEY_D), ox + 2 * (k + gap), oy + k + gap, k, k, s);
        float bw = (3 * k + 2 * gap - gap) / 2f, y2 = oy + 2 * (k + gap);
        key("LMB " + leftClicks.size(), prevLeft, ox, y2, bw, k * 0.8f, s * 0.8f);
        key("RMB " + rightClicks.size(), prevRight, ox + bw + gap, y2, bw, k * 0.8f, s * 0.8f);
        key("SPACE", Keyboard.isCreated() && Keyboard.isKeyDown(Keyboard.KEY_SPACE),
                ox, y2 + k * 0.8f + gap, 3 * k + 2 * gap, k * 0.55f, s * 0.8f);
    }

    private void key(String label, boolean down, float x, float y, float w, float h, float s) {
        TextRenderer.rect(x, y, w, h, down ? 0xBBFFFFFF : 0x66000000);
        float tw = text.width(label, s), th = text.height(s);
        text.draw(label, x + (w - tw) / 2f, y + (h - th) / 2f - s, s, down ? 0xFF000000 : 0xFFFFFFFF);
    }
}
