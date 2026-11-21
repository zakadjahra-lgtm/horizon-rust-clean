package dev.horizon.client;

/** سجل بسيط يظهر في Console اللانشر. */
final class Log {
    private Log() {}

    static void info(String m) { System.out.println("[HorizonClient] " + m); }

    static void err(String m) { System.err.println("[HorizonClient] " + m); }
}
