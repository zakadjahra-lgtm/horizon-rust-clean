package net.horizon.client.core;

/** ما تحتاجه المودات من اللعبة. كل إصدار ينفذه (طبقة رقيقة). */
public interface Platform {
    String gameVersion();
    int fps();
    long nowMs();
    boolean inWorld();
    /** الحالة الفعلية لمفتاح اللعبة (تشمل أي override وضعناه). */
    boolean isDown(KeyId k);
    /** حالة لوحة المفاتيح الحقيقية للمفتاح المرتبط، دون أي override. */
    boolean isPhysicalDown(KeyId k);
    /** يفرض أن يُعامَل المفتاح كأنه مضغوط (أو يلغي الفرض). الحالة النهائية = فعلي || override. */
    void setHeldOverride(KeyId k, boolean held);
    /**
     * هل المفتاح المسمّى (بأسماء LWJGL مثل "RSHIFT" أو "C") مضغوط الآن؟
     * يجب أن يعيد false إذا كانت هناك شاشة مفتوحة (حتى لا يعمل Zoom أثناء الكتابة في الشات).
     */
    boolean isNamedKeyDown(String keyName);
    /** الخوذة، الصدرية، البنطال، الحذاء (4 عناصر، قد تكون EMPTY). */
    ItemInfo[] armor();
    ItemInfo mainHand();
}
