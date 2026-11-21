package dev.horizon.client;

import java.lang.instrument.Instrumentation;

/**
 * نقطة الدخول (-javaagent). لا يعدّل شيئاً في Minecraft نفسها؛ يضيف فقط خطّافاً صغيراً
 * في حلقة رسم LWJGL (مستقلة عن أسماء Minecraft) لرسم طبقة HUD اختيارية.
 * أي خطأ هنا يُسجَّل ولا يوقف اللعبة أبداً.
 */
public final class Agent {
    private Agent() {}

    /** @param args مجلد اللعبة الخاص بالـ instance (يحوي horizon-client.properties) */
    public static void premain(String args, Instrumentation inst) {
        try {
            Config.init(args);
            inst.addTransformer(new Transformer());
            Log.info("agent loaded (game dir: " + args + ")");
        } catch (Throwable t) {
            Log.err("agent init failed, continuing without HUD: " + t);
        }
    }
}
