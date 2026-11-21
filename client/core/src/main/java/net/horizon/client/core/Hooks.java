package net.horizon.client.core;

/** حالة ساكنة تقرأها Mixins في المحوّلات (مثل معامل الـ Zoom). لا اعتماد على Minecraft. */
public final class Hooks {
    /** 1.0 = بلا تكبير؛ أقل من 1.0 = تكبير (يُضرب في FOV). */
    public static volatile double zoomFactor = 1.0;

    private Hooks() {}
}
