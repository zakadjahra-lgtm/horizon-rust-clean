package net.horizon.client.core;

/** وصف عنصر للعرض. `handle` كائن غامض يفهمه المحوّل (ItemStack في كل إصدار). */
public final class ItemInfo {
    public static final ItemInfo EMPTY = new ItemInfo(null, 0, 0, 0);

    public final Object handle;
    public final int count;
    public final int damage;
    public final int maxDamage;

    public ItemInfo(Object handle, int count, int damage, int maxDamage) {
        this.handle = handle;
        this.count = count;
        this.damage = damage;
        this.maxDamage = maxDamage;
    }

    public boolean isEmpty() { return handle == null || count <= 0; }
    public boolean hasDurability() { return maxDamage > 0; }
    public int durabilityLeft() { return Math.max(0, maxDamage - damage); }
    public int percent() {
        if (maxDamage <= 0) return 100;
        return Math.max(0, Math.min(100, (maxDamage - damage) * 100 / maxDamage));
    }
}
