// ينسخ الملفات الثابتة بجانب main.js المُصرَّف إلى dist/ (الذي يضمّنه Tauri داخل الـ EXE).
import { copyFileSync, mkdirSync } from 'node:fs';

mkdirSync('dist', { recursive: true });
for (const f of ['index.html', 'style.css']) copyFileSync(f, `dist/${f}`);
copyFileSync('../assets/logo.png', 'dist/logo.png');
