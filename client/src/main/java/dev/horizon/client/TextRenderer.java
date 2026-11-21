package dev.horizon.client;

import org.lwjgl.BufferUtils;
import org.lwjgl.opengl.GL11;

import java.awt.Color;
import java.awt.Font;
import java.awt.FontMetrics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.image.BufferedImage;
import java.nio.ByteBuffer;

/**
 * نص بسيط: يرسم ASCII 32..127 في atlas واحد عبر Java AWT (خط النظام) ثم يعرضه كـ quads.
 * مستقل عن خطوط Minecraft، لذا يعمل في كل الإصدارات.
 */
final class TextRenderer {
    private static final int COLS = 16, CELL_W = 32, CELL_H = 32, TEX_W = 512, TEX_H = 256;
    /** حجم الخط الظاهر = حجم الـ atlas × هذا المعامل × scale. */
    private static final float BASE = 0.5f;

    private final int tex;
    private final float[] advance = new float[96];

    /** يجب أن يُنشأ والـ GL context نشط (داخل الإطار). */
    TextRenderer() {
        BufferedImage img = new BufferedImage(TEX_W, TEX_H, BufferedImage.TYPE_INT_ARGB);
        Graphics2D g = img.createGraphics();
        g.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, RenderingHints.VALUE_TEXT_ANTIALIAS_ON);
        g.setRenderingHint(RenderingHints.KEY_FRACTIONALMETRICS, RenderingHints.VALUE_FRACTIONALMETRICS_ON);
        g.setFont(new Font(Font.SANS_SERIF, Font.BOLD, 24));
        g.setColor(Color.WHITE);
        FontMetrics fm = g.getFontMetrics();
        for (int i = 0; i < 96; i++) {
            char c = (char) (32 + i);
            g.drawString(String.valueOf(c), (i % COLS) * CELL_W + 2, (i / COLS) * CELL_H + fm.getAscent());
            advance[i] = fm.charWidth(c);
        }
        g.dispose();

        int[] px = img.getRGB(0, 0, TEX_W, TEX_H, null, 0, TEX_W);
        ByteBuffer buf = BufferUtils.createByteBuffer(TEX_W * TEX_H * 4);
        for (int p : px) {
            buf.put((byte) 255).put((byte) 255).put((byte) 255).put((byte) (p >>> 24));
        }
        buf.flip();

        tex = GL11.glGenTextures();
        GL11.glBindTexture(GL11.GL_TEXTURE_2D, tex);
        GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_MIN_FILTER, GL11.GL_LINEAR);
        GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_MAG_FILTER, GL11.GL_LINEAR);
        GL11.glTexImage2D(GL11.GL_TEXTURE_2D, 0, GL11.GL_RGBA, TEX_W, TEX_H, 0,
                GL11.GL_RGBA, GL11.GL_UNSIGNED_BYTE, buf);
    }

    float width(String s, float scale) {
        float w = 0;
        for (int i = 0; i < s.length(); i++) w += advance[idx(s.charAt(i))] * BASE * scale;
        return w;
    }

    float height(float scale) { return CELL_H * BASE * scale * 0.75f; }

    private static int idx(char c) {
        return (c < 32 || c > 127) ? '?' - 32 : c - 32;
    }

    /** يرسم نصاً مع ظل خفيف. (x,y) = أعلى يسار النص. يفترض تفعيل TEXTURE_2D والـ blend. */
    void draw(String s, float x, float y, float scale, int argb) {
        GL11.glBindTexture(GL11.GL_TEXTURE_2D, tex);
        pass(s, x + scale, y + scale, scale, 0x99000000);
        pass(s, x, y, scale, argb);
    }

    private void pass(String s, float x, float y, float scale, int argb) {
        GL11.glColor4f(((argb >> 16) & 255) / 255f, ((argb >> 8) & 255) / 255f,
                (argb & 255) / 255f, ((argb >>> 24) & 255) / 255f);
        float cw = CELL_W * BASE * scale, ch = CELL_H * BASE * scale;
        GL11.glBegin(GL11.GL_QUADS);
        for (int i = 0; i < s.length(); i++) {
            int k = idx(s.charAt(i));
            float u0 = (k % COLS) * CELL_W / (float) TEX_W, v0 = (k / COLS) * CELL_H / (float) TEX_H;
            float u1 = u0 + CELL_W / (float) TEX_W, v1 = v0 + CELL_H / (float) TEX_H;
            GL11.glTexCoord2f(u0, v0); GL11.glVertex2f(x, y);
            GL11.glTexCoord2f(u1, v0); GL11.glVertex2f(x + cw, y);
            GL11.glTexCoord2f(u1, v1); GL11.glVertex2f(x + cw, y + ch);
            GL11.glTexCoord2f(u0, v1); GL11.glVertex2f(x, y + ch);
            x += advance[k] * BASE * scale;
        }
        GL11.glEnd();
    }

    /** مستطيل ملوّن (يعطّل الـ texture مؤقتاً). */
    static void rect(float x, float y, float w, float h, int argb) {
        GL11.glDisable(GL11.GL_TEXTURE_2D);
        GL11.glColor4f(((argb >> 16) & 255) / 255f, ((argb >> 8) & 255) / 255f,
                (argb & 255) / 255f, ((argb >>> 24) & 255) / 255f);
        GL11.glBegin(GL11.GL_QUADS);
        GL11.glVertex2f(x, y);
        GL11.glVertex2f(x + w, y);
        GL11.glVertex2f(x + w, y + h);
        GL11.glVertex2f(x, y + h);
        GL11.glEnd();
        GL11.glEnable(GL11.GL_TEXTURE_2D);
    }
}
