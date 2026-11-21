package dev.horizon.client;

/** يُستدعى من داخل LWJGL كل إطار. لا يرمي استثناءً أبداً. */
public final class Hooks {
    private Hooks() {}

    private static boolean broken;
    private static Overlay overlay;

    public static void frame() {
        if (broken) return;
        try {
            if (overlay == null) overlay = new Overlay();
            overlay.render();
        } catch (Throwable t) {
            broken = true; // أوقف الـ HUD بدل أن نكرر الخطأ كل إطار
            Log.err("overlay disabled after error: " + t);
            t.printStackTrace();
        }
    }
}
