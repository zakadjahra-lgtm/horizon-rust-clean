package net.horizon.client.core;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/** مود اختياري. يبدأ معطّلاً دائماً؛ المستخدم يفعّله من قائمة Mods. */
public abstract class Mod {
    public final String id;
    public final String name;
    public final String description;
    /** "HUD" أو "MECHANIC". */
    public final String category;
    private boolean enabled;
    boolean lastEnabled; // يستخدمه HorizonClient لاكتشاف لحظة التعطيل
    private final List<Option> options = new ArrayList<Option>();

    protected Mod(String id, String name, String description, String category) {
        this.id = id; this.name = name; this.description = description; this.category = category;
    }

    protected final Option add(Option o) { options.add(o); return o; }
    public final List<Option> options() { return Collections.unmodifiableList(options); }

    public final Option option(String optId) {
        for (Option o : options) if (o.id.equals(optId)) return o;
        return null;
    }

    public final boolean enabled() { return enabled; }
    public final void setEnabled(boolean e) { enabled = e; }

    public void resetToDefaults() {
        enabled = false;
        for (Option o : options) o.reset();
    }

    /** يُستدعى كل tick (20Hz) والمود مفعّل. */
    public void onTick(Platform p) {}
    /** يُستدعى مرة واحدة عند التعطيل لتنظيف أي تأثير على اللعبة. */
    public void onDisable(Platform p) {}
}
