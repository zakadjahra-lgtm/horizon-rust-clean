package dev.horizon.client;

import java.io.File;
import java.io.FileInputStream;
import java.io.InputStream;
import java.util.Properties;

/**
 * إعدادات من ملف {@code horizon-client.properties} داخل مجلد اللعبة. كل المودات مطفأة افتراضياً
 * (Clean). يُعاد تحميل الملف تلقائياً عند تغيّره، فتسري تعديلات اللانشر أثناء اللعب.
 */
final class Config {
    private Config() {}

    private static File file;
    private static long loadedAt = -1;
    private static int tick;
    private static volatile Properties props = new Properties();

    static void init(String gameDir) {
        file = new File(gameDir == null || gameDir.isEmpty() ? "." : gameDir, "horizon-client.properties");
        load();
    }

    /** يُستدعى كل إطار؛ يفحص تعديل الملف كل ~60 إطاراً فقط. */
    static void poll() {
        if (file == null || ++tick % 60 != 0) return;
        long m = file.exists() ? file.lastModified() : -1;
        if (m != loadedAt) load();
    }

    private static void load() {
        Properties p = new Properties();
        loadedAt = file.exists() ? file.lastModified() : -1;
        if (loadedAt != -1) {
            try (InputStream in = new FileInputStream(file)) {
                p.load(in);
            } catch (Exception e) {
                Log.err("could not read " + file + ": " + e);
            }
        }
        props = p;
    }

    static boolean bool(String key, boolean def) {
        String v = props.getProperty(key);
        return v == null ? def : Boolean.parseBoolean(v.trim());
    }

    static int num(String key, int def) {
        try {
            String v = props.getProperty(key);
            return v == null ? def : Integer.parseInt(v.trim());
        } catch (NumberFormatException e) {
            return def;
        }
    }
}
