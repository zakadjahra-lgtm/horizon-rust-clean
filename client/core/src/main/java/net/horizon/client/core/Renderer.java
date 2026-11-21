package net.horizon.client.core;

/** واجهة الرسم التي ينفذها كل إصدار. الإحداثيات بوحدات GUI المقيّسة (scaled). */
public interface Renderer {
    int width();
    int height();
    void fill(int x, int y, int w, int h, int argb);
    void text(String s, int x, int y, int argb, boolean shadow);
    int textWidth(String s);
    int fontHeight();
    /** يرسم أيقونة العنصر 16x16 عند (x,y). */
    void item(ItemInfo item, int x, int y);
    void clip(int x, int y, int w, int h);
    void unclip();
}
