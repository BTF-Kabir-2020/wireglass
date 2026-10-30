# Android permissions policy (Wireglass)

**Goal:** never become the app that dies in permission hell.

## Hard rules

1. **Default allow-list is only** `android.permission.INTERNET`.
2. **No runtime dangerous permissions** in v1 (storage, camera, mic, location, phone, SMS, contacts, nearby devices, notifications — none).
3. **Do not** add a permission “just in case”.
4. Every new permission needs: product reason, Android docs link, Gate report, and owner OK.
5. After every APK build, run the permission audit script. Unexpected entries = **fail**.

## Why INTERNET only

- Wireglass is a network diagnostic tool to public hosts.
- `INTERNET` is **normal** (install-time). Users do **not** get a scary runtime popup for it on modern Android.
- Pain in communities usually comes from: missing INTERNET, over-requesting dangerous perms, cleartext flags, or LAN/nearby permissions.

## Explicitly out of scope (for now)

| Permission / flag | Why we avoid |
| --- | --- |
| `WRITE/READ_EXTERNAL_STORAGE` / media | Export stays in-app / Share later without broad storage |
| `ACCESS_FINE_LOCATION` / coarse | Not needed for DNS/TCP/TLS of a typed host |
| `NEARBY_WIFI_DEVICES` / local-network | Only if we later prove LAN/mDNS need; document first |
| `POST_NOTIFICATIONS` | No background scanning |
| `QUERY_ALL_PACKAGES` | Never |
| Global `usesCleartextTraffic=true` | Prefer HTTPS; show honest errors for http |

## Audit

```bash
./scripts/linux/audit-apk-permissions.sh path/to/app.apk
```

Allowed names today:

- `android.permission.INTERNET`
