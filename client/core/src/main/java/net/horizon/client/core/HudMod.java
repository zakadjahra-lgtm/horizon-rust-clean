package net.horizon.client.core;

/** مود يرسم عنصراً على الشاشة. الموضع بنسبة مئوية من الشاشة (مستقل عن الدقة). */
public abstract class HudMod extends Mod {
    protected final Option posX;
    protected final Option posY;
    protected final Option background;

    protected HudMod(String id, String name, String description, String category, int defX, int defY) {
        super(id, name, description, category);
        posX = add(Option.integer("posX", "Position X (%)", 0, 100, 1, defX));
        posY = add(Option.integer("posY", "Position Y (%)", 0, 100, 1, defY));
        background = add(Option.bool("background", "Background", true));
    }

    public abstract int width(Renderer r, Platform p);
    public abstract int height(Renderer r, Platform p);
    public abstract void draw(Renderer r, Platform p, int x, int y);

    /** يُستدعى كل إطار رسم قبل width/height؛ للمودات التي تقرأ حالة كل إطار (مثل CPS). */
    public void frame(Platform p) {}

    final int screenX(Renderer r, int w) { return posX.get() * Math.max(0, r.width() - w) / 100; }
    final int screenY(Renderer r, int h) { return posY.get() * Math.max(0, r.height() - h) / 100; }

    protected final void panel(Renderer r, int x, int y, int w, int h) {
        if (background.on()) r.fill(x, y, w, h, 0x80000000);
    }
}
