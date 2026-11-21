package net.horizon.client.core;

/** خيار قابل للضبط لمود: منطقي (BOOL) أو رقمي بمدى (INT). */
public final class Option {
    public enum Type { BOOL, INT }

    public final String id;
    public final String label;
    public final Type type;
    public final int min, max, step, def;
    private int value;

    private Option(String id, String label, Type type, int min, int max, int step, int def) {
        this.id = id; this.label = label; this.type = type;
        this.min = min; this.max = max; this.step = Math.max(1, step); this.def = def;
        this.value = def;
    }

    public static Option bool(String id, String label, boolean def) {
        return new Option(id, label, Type.BOOL, 0, 1, 1, def ? 1 : 0);
    }

    public static Option integer(String id, String label, int min, int max, int step, int def) {
        return new Option(id, label, Type.INT, min, max, step, def);
    }

    public int get() { return value; }
    public boolean on() { return value != 0; }

    public void set(int v) {
        if (type == Type.INT && step > 1) {
            v = min + Math.round((float) (v - min) / step) * step;
        }
        value = Math.max(min, Math.min(max, v));
    }

    public void setBool(boolean b) { value = b ? 1 : 0; }
    public void reset() { value = def; }
}
