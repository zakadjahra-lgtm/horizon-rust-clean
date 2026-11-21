package dev.horizon.client;

import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassVisitor;
import org.objectweb.asm.ClassWriter;
import org.objectweb.asm.MethodVisitor;
import org.objectweb.asm.Opcodes;

import java.lang.instrument.ClassFileTransformer;
import java.security.ProtectionDomain;

/**
 * يضيف استدعاءً واحداً {@code Hooks.frame()} في بداية {@code Display.update(boolean)} في LWJGL 2
 * (أي قبل تبديل الـ buffers مباشرة، فيُرسم الـ HUD فوق الإطار). Minecraft نفسها غير مُعدَّلة.
 */
final class Transformer implements ClassFileTransformer {
    private static final String DISPLAY = "org/lwjgl/opengl/Display";

    @Override
    public byte[] transform(ClassLoader loader, String name, Class<?> redefined,
                            ProtectionDomain pd, byte[] bytes) {
        if (!DISPLAY.equals(name)) return null;
        try {
            ClassReader cr = new ClassReader(bytes);
            ClassWriter cw = new ClassWriter(cr, ClassWriter.COMPUTE_MAXS);
            final boolean[] patched = {false};
            cr.accept(new ClassVisitor(Opcodes.ASM9, cw) {
                @Override
                public MethodVisitor visitMethod(int access, String n, String desc, String sig, String[] ex) {
                    MethodVisitor mv = super.visitMethod(access, n, desc, sig, ex);
                    if (mv != null && "update".equals(n) && "(Z)V".equals(desc)) {
                        patched[0] = true;
                        return new MethodVisitor(Opcodes.ASM9, mv) {
                            @Override
                            public void visitCode() {
                                super.visitCode();
                                super.visitMethodInsn(Opcodes.INVOKESTATIC,
                                        "dev/horizon/client/Hooks", "frame", "()V", false);
                            }
                        };
                    }
                    return mv;
                }
            }, 0);
            if (!patched[0]) {
                Log.err("Display.update(Z) not found; HUD disabled for this LWJGL version");
                return null;
            }
            Log.info("hooked LWJGL 2 Display.update");
            return cw.toByteArray();
        } catch (Throwable t) {
            Log.err("transform failed: " + t);
            return null; // اترك الصنف الأصلي كما هو
        }
    }
}
