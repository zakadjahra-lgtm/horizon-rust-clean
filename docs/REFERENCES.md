# المراجع التقنية والتراخيص

آخر تحديث: 2026-11-22. هذه الوثيقة نتيجة فحص فعلي للملفات المرفقة وصفحات المستودعات الرسمية.
**ليست استشارة قانونية**: أي استخدام تجاري أو إعادة توزيع يحتاج مراجعة قانونية حقيقية.

## الخلاصة

1. **Apollo وBadlion ModAPI وBACPluginAPI كلها إضافات/مكتبات للسيرفر** (Bukkit/Bungee/Velocity…)، وليست مكتبات تُبنى بها عملاء. هي تدلّنا على *فكرة* التكامل بين السيرفر والعميل، لا على كود نعيد استخدامه في عميلنا.
2. **لا يوجد كود من Lunar أو Badlion في مشروعنا**، ولن يوجد. عميلنا (`client/`) مكتوب من الصفر ويعتمد على LWJGL نفسها بدل أي مكتبة منهم.
3. من هذه القائمة يمكن استخدام **Configurate (Apache-2.0)** و**Apollo وServerMappings (MIT)** قانونياً بشروط بسيطة، لكن لا حاجة تقنية لأي منها الآن (التفاصيل أدناه).
4. **Sodium** ترخيصه *PolyForm Shield 1.0.0* (مصدر متاح وليس مفتوحاً)، فلا ننسخ كوده ولا نضمّنه: نُنزّله اختيارياً من Modrinth بطلب المستخدم، ويبقى بترخيصه.

## جدول التراخيص والقرارات

| المرجع | ما هو فعلاً | الترخيص (مصدر التحقق) | القرار |
|---|---|---|---|
| [LunarClient/Apollo](https://github.com/LunarClient/Apollo) (مرفق: `apollo-bukkit-1.3.0.jar`) | Server API لـ Lunar: Bukkit/Folia/Minestom/Bungee/Velocity | **MIT** ("Apollo is licensed under the MIT license" — README) | دراسة المفاهيم فقط. لا نعتمده: هو جهة السيرفر، ومنفذه الفعلي داخل Lunar Client المغلق. لا ننفّذ بروتوكوله لننتحل هوية Lunar. |
| [LunarClient/ServerMappings](https://github.com/LunarClient/ServerMappings) | جدول مفتوح: عنوان السيرفر ↔ اسم عرض جميل | **MIT** (بيانات المستودع على ecosyste.ms) | مفيد لاحقاً لقائمة السيرفرات. إن استُخدم يجب حفظ إشعار MIT وحقوق النشر. غير مطلوب الآن. |
| lunarclient.dev (مرفق: docs) | موقع توثيق: deep links، client brand، Maven، شراكات السيرفرات | **غير معروف**: لا ملف ترخيص داخل الأرشيف المرفق، ولم أتمكن من فتح صفحة المستودع للتحقق | للقراءة فقط. لا ننسخ نصوصه. |
| [BadlionClientModAPI](https://github.com/BadlionClient/BadlionClientModAPI) (مرفق: `Badlion-ModApi-Bukkit.jar` v2.1.9) | إضافة سيرفر تعطّل مودات/ميزات في Badlion وتحدد CPS | **لا ترخيص**: لا في الـ README ولا داخل الـ jar (فحصت `META-INF` والملفات) ⇒ كل الحقوق محفوظة افتراضياً | **لا نستخدم الكود ولا نفككه ولا نعيد توزيعه.** نستفيد فقط من الفكرة الموصوفة علناً (سيرفر يقيّد مودات). |
| [BACPluginAPI](https://github.com/BadlionClient/BACPluginAPI) (مرفق) | API لمضاد غش Badlion على السيرفر (`HeartbeatApi`) | **MIT** © ESL Gaming Online (ملف LICENSE في الأرشيف) | غير ذي صلة بعميل. غير مستخدم. |
| [Mojang/LegacyLauncher](https://www.github.com/Mojang/LegacyLauncher) | وصفه: "Hacky code to launch our old versions from the new launcher!" | **لا ترخيص ظاهر** في صفحة المستودع | لا ننسخ. لانشرنا يبني أمر التشغيل من Version JSON الرسمي أصلاً (راجع `launch.rs`) ولا يحتاج هذا الكود. |
| [Sodium](https://github.com/CaffeineMC/sodium) | مود تحسين رسوميات (Fabric/NeoForge/Quilt)، client-side | **PolyForm Shield 1.0.0** (README + [Modrinth](https://modrinth.com/project/sodium)) | لا ننسخ ولا نضمّن. تنزيل اختياري من Modrinth للإصدارات 1.16.3 فما فوق فقط. لا يدعم 1.8.9 ولا 1.12.2. |
| [Lithium](https://modrinth.com/project/lithium) | مود تحسين منطق اللعبة (Fabric/NeoForge/Quilt) | **LGPL-3.0-only** (Modrinth) | نفس أسلوب Sodium (تنزيل اختياري، لا تضمين). 1.16.2 فما فوق. |
| [Configurate](https://www.GitHub.com/SpongePowered/Configurate) | مكتبة Java للإعدادات (JSON/HOCON/YAML/XML) | **Apache-2.0** (README) | مسموحة قانونياً، لكن غير لازمة الآن (انظر القسم التالي). |
| [Modrinth API](https://docs.modrinth.com/api/) | مصدر المودات للـ Mod Manager | شروط الـ API: `User-Agent` فريد إلزامي، 300 طلب/دقيقة/IP | سنلتزم بها في وحدة Mod Manager. |

### ملاحظة عن PolyForm Shield (Sodium)
ترخيص "مصدر متاح": يسمح بالاستخدام والتعديل بشروط، ويمنع استخدام البرنامج لتقديم منتج ينافس صاحب الترخيص. لذلك أبسط وأسلم مسار: لا نأخذ كوداً منه، ونترك المستخدم يثبّته من الموقع الرسمي كما تفعل أغلب اللانشرات. (التراخيص تتغير، فيُعاد فحص الصفحة قبل أي ربط تلقائي.)

## دراسة كل نقطة طلبتها

**Configurate (الإعدادات).** فكرتها مفيدة: ملف إعدادات بنوع مُعرَّف، تحميل/حفظ، وإعادة تحميل. لدينا هذا:
- Rust يملك الإعدادات (JSON عبر serde): `settings.json`، و`profiles/*.json` الجديدة.
- الـ agent الجافا يقرأ ملف `horizon-client.properties` مسطّحاً ويعيد تحميله عند تغيّره (`Config.java`).
- ندخل Configurate **فقط** إن احتجنا إعدادات متداخلة معقدة داخل الـ agent، وعندها تُضمَّن مع relocation وإشعار Apache-2.0.

**Sodium (الأداء).** يعمل بإعادة كتابة مسار رسم الـ chunks عبر Fabric/NeoForge ويحتاج OpenGL 3.3. غير قابل للتطبيق على 1.8–1.12.2. لذلك استراتيجيتنا طبقتان:
- **قديم (1.8–1.12.2):** إعدادات JVM مدروسة (`perf.rs`) + HUD بتكلفة صفر عند الإطفاء. لم تُقَس مكاسبها بعد، ولن أدّعي رقماً قبل القياس.
- **حديث (1.16.3+):** Sodium/Lithium اختيارياً عبر Mod Manager.

**LegacyLauncher (تشغيل Java).** لا ترخيص ظاهر، فلا نقرأ منه إلا الوصف العام. ما نحتاجه موجود عندنا ومُختبَر على CI: Version JSON القديم والحديث، `inheritsFrom`، natives، وJava Runtime الرسمي.

**Apollo (تكاملات Minecraft).** الدرس: السيرفر يرسل رسائل (plugin messages) تضبط الميزات في العميل، والعميل يعلن هويته. ما نأخذه كفكرة:
- قناة قيود من السيرفر إلى عميلنا (مستقبلاً وبقناة **خاصة بنا**، مثل `horizon:restrictions`).
- نرسل هوية صادقة (`horizon`) ولا ننتحل `lunarclient` أو `badlion`. Lunar نفسها تحذّر أن brand المنتحَل ليس آمناً في [client-brand](https://lunarclient.dev/client-brand)، وانتحاله تحايل على السيرفرات.

**BadlionClientModAPI (إدارة المودات).** الدرس: كل مود له معرّف، ويمكن للسيرفر تعطيل المود كله أو ميزة داخله. نموذجنا (مكتوب بأيدينا): كل مود في `client/` له `id` ثابت (`fps`, `cps`, `keystrokes`)، وكل **Profile** يحدد المفعّل منها ومواقعها.

## معمارية نظام الـ Client عندنا

```
Rust Launcher (hrc-core)
 ├─ profile.rs   Profiles: مودات HUD مفعّلة + مواقعها + preset الأداء        ← جديد
 ├─ perf.rs      JVM flags حسب preset وإصدار Java + أهلية Sodium/Lithium    ← جديد
 ├─ instance.rs  كل instance = إصدار + مجلد مستقل + profile_id             ← محدَّث
 └─ launch.rs    يبني الأمر: JVM الخاصة بالـ preset + -javaagent
          │  يكتب  <game>/horizon-client.properties
          ▼
Java Agent (client/)  يقرأ الملف، يرسم HUD، لا يغيّر Minecraft
```
"إعدادات مستقلة لكل إصدار" تتحقق لأن كل instance له مجلد وملفات خاصة وprofile خاص، ومنها `RAM` وJava ومسار الـ agent.

## ما بُني وما لم يُبنَ (انظر `STATUS.md` للحالة المُتحقَّق منها)
- مبني: Profiles، presets الـ JVM، أهلية Sodium/Lithium حسب الإصدار، توليد ملف إعدادات الـ agent.
- غير مبني بعد: Mod Manager (Modrinth + Fabric loader)، واجهة Profiles في اللانشر، قائمة داخل اللعبة، دعم LWJGL 3.

## المصادر
- [Apollo (GitHub)](https://github.com/LunarClient/Apollo) · [ServerMappings](https://github.com/LunarClient/ServerMappings) · [lunarclient.dev: client brand](https://lunarclient.dev/client-brand)
- [BadlionClientModAPI (readme)](https://github.com/BadlionClient/BadlionClientModAPI/blob/master/readme.md) · [BACPluginAPI](https://github.com/BadlionClient/BACPluginAPI)
- [Mojang/LegacyLauncher](https://www.github.com/Mojang/LegacyLauncher)
- [Sodium (GitHub)](https://github.com/CaffeineMC/sodium) · [Sodium (Modrinth)](https://modrinth.com/project/sodium) · [Lithium (Modrinth)](https://modrinth.com/project/lithium)
- [Configurate](https://www.GitHub.com/SpongePowered/Configurate) · [Modrinth API](https://docs.modrinth.com/api/)
