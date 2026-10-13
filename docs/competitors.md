# docs/competitors.md — پژوهش رقبا و شکاف بازار (WIREGLASS)

> **گیت اجباری فاز ۰.** این سند پیش از نوشتن کد تعیین می‌کند WIREGLASS دقیقاً چه چیزی را بهتر نشان می‌دهد و چه چیزی را بهتر نشان نمی‌دهد.
> تاریخ پژوهش: ۲۰۲۶-۱۰. داده‌ها از مستندات رسمی، مخازن عمومی و سایت ابزارها. شماره‌ها «شاهد»اند، نه تعهد دائمی.

---

## ۰. خلاصه‌ی مدیریتی (نتیجه‌ی صادقانه)

**تمایز اصلی که در طرح محصول (خصوصی) ادعا شده بود — «تایم‌لاین یکپارچه از مراحل اتصال + گزارش قابل‌بازتولید» — از قبل توسط دو ابزار فعال اشغال شده است، از جمله یک ابزار Rust:**

- **`http-stat-rs`** (Rust، MIT، خالص، تک‌باینری، صفر-وابستگی): HTTP/1.1 + HTTP/2 + HTTP/3، تایم‌لاین DNS/TCP/TLS/Server/Transfer، تجزیه‌ی `Server-Timing`، دیاگ‌های TCP و TLS. (~۲۱ ستاره، فعال، ساخت ۲۰۲۵-۰۴)
- **`httptap`** (Python، Apache-2.0): محوشدن waterfall، زنجیره‌ی ریدایرکت با زمان‌بندی هر گام، خروجی JSON، بررسی آستانه‌ی SLO. (~۸۰۹ ستاره)

**نتیجه:** اگر WIREGLASS فقط «یک تایم‌لاین دیگر» باشد، نسخه‌ی دیگری از ابزار موجود است و شرط شروع خودمان را رد می‌کند.
**شکاف واقعی، نه در «داشتن تایم‌لاین»، بلکه در سه چیز است:**

1. **عمق جمع‌شده در یک گزارش:** ابزارهای تایم‌لاین، DNS را فقط تا حد «resolve شدن» می‌بینند (زنجیره‌ی DNS/DNSSEC/CAA/TTL را دنبال نمی‌کنند)؛ و ابزارهای TLS/DNS جدا هستند. هیچ‌کدام «مسیر واقعی از DNS تا ALPN نهایی» را در یک خروجی واحد و قابل‌diff نمی‌دهند.
2. **صداقت درباره‌ی شکست:** مسئله‌ی واقعی و مستند این است که وقتی HTTP/3 کار نمی‌کند، همه بی‌صدا به HTTP/2 برمی‌گردند و «همه‌چیز خوب به نظر می‌رسد» — در حالی که هزینه‌ی latency یک پروتکل تبلیغ‌شده‌ی بی‌فایده را می‌دهی. ابزارهای کمی این «افت بی‌صدا» را به زبان ساده نشان می‌دهند.
3. **گزارش تصویریِ آفلاین و قابل‌اشتراک:** خروجی اکثر این ابزارها متن ترمینال است؛ DNSViz تصویری است ولی فقط DNS/DNSSEC و وابسته به وب.

**توصیه:** MVP را روی این سه باریک کنیم، نه روی «تایم‌لاین» به‌عنوان ارزش اصلی.

---

## ۱. روش پژوهش

دسته‌های بررسی‌شده: (الف) تایم‌لاین درخواست HTTP، (ب) اسکن/امتیازدهی TLS، (پ) تحلیل و نمایش DNS/DNSSEC، (ت) تشخیص مسیر شبکه و پکت، (ث) بررسی HTTP/3/QUIC.
هر ابزار از نظر: قابلیت، مدل توزیع (باینری/وابستگی)، خروجی (متن/تصویری/JSON)، پوشش پروتکل، و محدودیت ارزیابی شد.

---

## ۲. ماتریس رقبا (بر اساس شواهد)

| ابزار | زبان/توزیع | پوشش | خروجی | قوت اصلی | محدودیت کلیدی | شاهد |
|---|---|---|---|---|---|---|
| **http-stat-rs** | Rust، MIT، صفر-وابستگی، تک‌باینری | HTTP/1.1, /2, /3؛ DNS/TCP/TLS/Server/Transfer | تایم‌لاین ترمینال + (JSON از طریق نسخه‌های مشابه) | هم‌زبان پروژه؛ عمق فازها؛ `Server-Timing`؛ مباحث TCP (RTT/MSS/cwnd) | DNS سطحی (بدون زنجیره/DNSSEC/CAA)؛ تمرکز بر یک درخواست؛ خروجی تصویری ندارد | [GitHub](https://github.com/vicanso/http-stat-rs) / [crates.io](https://crates.io/crates/http-stat) |
| **httptap** | Python، Apache-2.0 | DNS/TCP/TLS/TTFB/Transfer | waterfall، compact، `--metrics-only`، JSON | زنجیره‌ی ریدایرکت با زمان هر گام؛ بررسی SLO؛ بینش TLS | وابسته به پایتون؛ DNS سطحی؛ تصویری قابل‌share نیست | [GitHub](https://github.com/ozeranskii/httptap) / [docs](https://docs.httptap.dev/) |
| **curl -w / httpstat** | C / shell | فازهای یک درخواست | متن | همه‌جا نصب است؛ استاندارد دی‌فکتو | بدون DNS عمیق؛ بدون HTTP/3 قابل‌اعتماد؛ بدون تصویر | [stackharbor](https://stackharbor.com/en/knowledge-base/curl-request-timing-breakdown) |
| **testssl.sh** | Bash + OpenSSL | TLS: پروتکل‌ها، cipher‌ها، آسیب‌پذیری‌ها، امتیاز (پیاده‌سازی SSL Labs) | متن رنگی + JSON | عمق واقعی TLS؛ امتیازدهی؛ ALPN/HTTP2 | فقط TLS؛ نه مسیر اتصال؛ نه DNS chain؛ نه تصویری | [testssl.sh](https://testssl.sh/) |
| **sslyze / tlsx** | Python / Go | پیکربندی TLS | متن/JSON | اسکن سریع TLS؛ مناسب اتوماسیون | فقط TLS؛ نه تایم‌لاین؛ نه گزارش اشتراکی | [pentest-book](https://www.pentest-book.com/enumeration/ssl-tls) |
| **DNSViz** | Python | DNS/DNSSEC: زنجیره‌ی تأیید، خطاهای پیکربندی | **تصویری (SVG/PNG)** + JSON | نمایش بصری زنجیره‌ی DNSSEC | فقط DNS؛ وب-محور؛ نه TCP/TLS/HTTP؛ نه latency | [dnsviz.net](https://dnsviz.net/) / [GitHub](https://github.com/dnsviz/dnsviz) |
| **quicprobe** | CLI، صفر-وابستگی | HTTP/3: Alt-Svc، handshake واقعی QUIC v1، 0-RTT، fallback | متن به‌زبان ساده | دقیقاً «افت بی‌صدای H3» را روشن می‌کند | فقط H3؛ نه تایم‌لاین کامل؛ نه DNS/TLS عمیق | [GitHub](https://github.com/JaydenCJ/quicprobe) |
| **http3verify / nettools.fi** | وب | HTTP/3: probe واقعی + Alt-Svc/ALPN | وب | تأکید درست: «Alt-Svc صرف کافی نیست» | آنلاین، نه محلی/OSS؛ فقط H3 | [http3verify](https://www.http3verify.com/)، [nettools](https://nettools.fi/en/tools/http3-quic-check) |
| **mtr** | C | مسیر hop‌ها + loss/latency | TUI/متن/JSON/XML/CSV | استاندارد عیب‌یابی مسیر | ICMP/TCP/UDP traceroute؛ نه DNS chain/ALPN/HTTP؛ نه تصویری | [man mtr](https://manpages.ubuntu.com/manpages/focal/man8/mtr.8.html) |
| **trippy** | Rust، MIT | traceroute+ping، GeoIP، ASN | TUI + JSON/CSV/dot | هم‌زبان؛ نقشه‌ی GeoIP؛ OSS Rust | مسیر شبکه، نه لایه‌ی DNS/TLS/HTTP app؛ نه گزارش اشتراکی | [trippy.rs](https://trippy.rs/start/features) |
| **termshark / tshark** | Go + tshark | تحلیل پکت/pcap کامل | TUI + متن | عمیق‌ترین سطح (پکت) | نیازمند tshark؛ پیچیده؛ نه خلاصه‌ی قابل‌فهم برای غیر-متخصص | [termshark.io](https://termshark.io/) |

---

## ۳. شکاف واقعی بازار (بر اساس ماتریس)

**آنچه پوشش داده شده:** فازهای یک درخواست (http-stat-rs، httptap)، عمق TLS (testssl.sh)، نمایش DNS/DNSSEC (DNSViz)، مسیر شبکه (mtr/trippy)، بررسی H3 (quicprobe/http3verify).

**آنچه هیچ ابزار واحدی ندارد:**

1. **مسیر کامل، یک‌جا:** از زنجیره‌ی DNS (chain/TTL/DNSSEC/CAA) → TCP → handshake واقعی TLS (ALPN) → **مذاکره‌ی واقعی HTTP/3 و رفتار fallback** → هدرها، همه در یک گزارش واحدِ قابل‌diff.
2. **صداقت ساختاری درباره‌ی شکست:** علامت‌گذاری شفاف هر اندازه‌گیری ناموفق (بدون حدس)، به‌ویژه «H3 تبلیغ شده ولی برقرار نشد».
3. **گزارش تصویری محلی و قابل‌اشتراک، OSS و تک‌باینری Rust:** ایران و همه‌جا بدون وابستگی به وب‌سرویس و بدون فرستادن داده به سرور ثالث.

---

## ۴. تمایزهای نامزد — تأیید یا رد (مهم)

| تمایز نامزد | وضعیت | دلیل (شاهد) |
|---|---|---|
| «تایم‌لاین یکپارچه‌ی مراحل اتصال» | ❌ **رد** | http-stat-rs و httptap همین را دارند |
| «گزارش قابل‌بازتولید (همان ورودی → همان خروجی)» | ⚠️ **ضعیف** | ابزارهای موجود به‌طور طبیعی قطعی‌اند (به‌جز DNS/شبکه‌ی متغیر)؛ به‌تنهایی تمایز نیست |
| «DNS کامل + TLS + H3 در یک گزارش» | ✅ **نامزد قوی** | هیچ ابزار واحدی این ترکیب را ندارد (DNSViz فقط DNS، testssl فقط TLS، http-stat-rs DNS سطحی) |
| «صداقت درباره‌ی افت بی‌صدای HTTP/3» | ✅ **نامزد قوی** | quicprobe همین مسئله را مطرح می‌کند ولی فقط H3؛ ترکیب با DNS/TLS باز است |
| «گزارش تصویری محلی + OSS + تک‌باینری Rust» | ✅ **نامزد قوی** | خروجی رقبا عمدتاً متن؛ DNSViz تصویری ولی وب-محور و فقط DNS |
| «کارت طنز / نمره‌ی /۱۰۰» | ❌ **رد به‌عنوان تمایز** | قابل‌کپی، اعتبار فنی نمی‌سازد؛ می‌تواند لایه‌ی اختیاری بماند |

**درصدِ تأیید: ۳ نامزد قوی، ۲ رد، ۱ ضعیف.** MVP باید روی سه‌تای قوی سوار شود، نه روی تایم‌لاین.

---

## ۵. دامنه‌ی پیشنهادی MVP (بر اساس شواهد)

**در دامنه (باریک و قابل‌دفاع):**
- ورودی یک URL → یک گزارش **واحد**: زنجیره‌ی DNS (chain/TTL/DNSSEC/CAA) + TCP + TLS(ALPN) + **probe واقعی HTTP/3 + وضعیت fallback** + هدرهای کلیدی.
- **وضعیت صریح هر مرحله:** موفق/ناموفق/نامشخص — بدون حدس.
- خروجی **JSON قابل‌diff** + یک **تصویر تک‌صفحه‌ای محلی** (SVG→PNG) از مسیر.
- تک‌باینری Rust، اجرای کاملاً محلی، بدون ارسال داده به سرور.

**خارج از دامنه (تا اثبات تقاضا):**
- لیدربورد، حساب کاربری، سرویس عمومی آنلاین، امتیاز /۱۰۰ به‌عنوان محور، طنز به‌عنوان محصول.

**پرسش‌های باز که باید پیش از کد پاسخ داده شوند:**
1. آیا کسی برای «مسیر کامل DNS→H3 در یک گزارش» حاضر است ابزار را اجرا/ستاره کند؟ (باید با دموی کوچک تست شود.)
2. تمایز «گزارش تصویری محلی» چقدر ارزش دارد وقتی DNSViz تصویری دارد (هرچند فقط DNS)؟
3. نسبت به http-stat-rs، آیا زیادکردن DNS عمیق + H3 fallback، «کافی» است یا باید عمق دیگری (مثلاً vantage point چندگانه) اضافه شود؟

---

## ۶. ریسک رقابتی و لحن

- **ریسک کپی‌شدن:** http-stat-rs فعال است؛ اگر WIREGLASS فقط چند فیلد اضافه کند، سریع جبران می‌شود. تمایز باید در **عمق DNS + رفتار H3 + گزارش تصویری** هم‌زمان باشد.
- **ریسک «چرخ فلک دوباره‌سازی»:** ساختن چیزی که ~۸۰٪ آن موجود است. اگر سه‌تای قوی قابل‌دفاع نبودند، پروژه باید متوقف یا تغییر جهت دهد.
- **درس از quicprobe:** زبان ساده درباره‌ی یک مسئله‌ی واقعی و پنهان («پروتکلی که تبلیغ می‌کنی ولی نمی‌دهی»)، داستان فنی قوی‌تری از «نمره» می‌سازد.

---

## ۷. منابع

- http-stat-rs: https://github.com/vicanso/http-stat-rs ، https://crates.io/crates/http-stat ، https://docs.rs/crate/http-stat/0.7.1/source/README.md
- httptap: https://github.com/ozeranskii/httptap ، https://docs.httptap.dev/
- testssl.sh: https://testssl.sh/ ، https://testssl.sh/doc/testssl.1.html
- sslyze / tlsx (فهرست ابزارها): https://www.pentest-book.com/enumeration/ssl-tls
- DNSViz: https://dnsviz.net/ ، https://github.com/dnsviz/dnsviz
- quicprobe: https://github.com/JaydenCJ/quicprobe
- HTTP/3 verify: https://www.http3verify.com/ ، https://nettools.fi/en/tools/http3-quic-check
- HTTP/3 و Alt-Svc (RFC): https://www.rfc-editor.org/rfc/rfc9114.html ، https://http.dev/alt-svc
- mtr: https://manpages.ubuntu.com/manpages/focal/man8/mtr.8.html
- trippy: https://trippy.rs/start/features
- termshark/tshark: https://termshark.io/ ، https://www.wireshark.org/docs/man-pages/tshark
- curl timing: https://stackharbor.com/en/knowledge-base/curl-request-timing-breakdown
