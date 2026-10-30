//! Wireglass Android — Slint UI wired to `wireglass::run`.
//! Navigation, validation, back stack, and exit confirm live here.

slint::include_modules!();

#[cfg(target_os = "android")]
mod android_bridge;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use wireglass::Report;
/// Accept http(s) URLs or bare hosts / IPv4. Reject empty, spaces, other schemes.
fn normalize_target(raw: &str) -> Result<String, String> {
    let t = raw.trim();
    if t.is_empty() {
        return Err("Enter a domain or URL.".into());
    }
    if t.chars().any(|c| c.is_whitespace()) {
        return Err("Remove spaces from the target.".into());
    }
    if t.contains("://") {
        let lower = t.to_ascii_lowercase();
        if !(lower.starts_with("http://") || lower.starts_with("https://")) {
            return Err("Only http:// or https:// URLs are allowed.".into());
        }
        // Reject scheme-only / hostless inputs such as "https://" or "http://".
        let after_scheme = t.split_once("://").map(|x| x.1).unwrap_or("");
        let host_part = after_scheme.split('/').next().unwrap_or("");
        let host_part = host_part.split('?').next().unwrap_or(host_part);
        let host_part = host_part.split('#').next().unwrap_or(host_part);
        if host_part.trim().is_empty() {
            return Err("URL is missing a host.".into());
        }
        Ok(t.to_string())
    } else {
        Ok(format!("https://{}", t))
    }
}

fn stage_row(name: &str, status: &str, detail: impl Into<SharedString>) -> StageRow {
    StageRow {
        name: name.into(),
        status: status.into(),
        detail: detail.into(),
    }
}

fn map_report(report: &Report) -> Vec<StageRow> {
    use wireglass::Stage;
    let mut rows = Vec::new();

    match &report.dns {
        Stage::Ok(d) => rows.push(stage_row(
            "DNS",
            "ok",
            format!(
                "{} ms · {} IPv4 · {} IPv6 · resolver {}",
                d.duration_ms,
                d.a.len(),
                d.aaaa.len(),
                d.resolver
            ),
        )),
        Stage::Failed { error } => rows.push(stage_row("DNS", "failed", error.clone())),
        Stage::NotAttempted { reason } => {
            rows.push(stage_row("DNS", "unsupported", reason.clone()))
        }
    }

    match &report.tcp {
        Stage::Ok(d) => rows.push(stage_row(
            "TCP",
            "ok",
            format!("Connected to {} in {} ms", d.addr, d.connect_ms),
        )),
        Stage::Failed { error } => rows.push(stage_row("TCP", "failed", error.clone())),
        Stage::NotAttempted { reason } => {
            rows.push(stage_row("TCP", "unsupported", reason.clone()))
        }
    }

    match &report.tls {
        Stage::Ok(d) => rows.push(stage_row(
            "TLS",
            "ok",
            format!(
                "{} · {} · {} ms · {}",
                d.protocol_version, d.cipher_suite, d.handshake_ms, d.cert_subject
            ),
        )),
        Stage::Failed { error } => rows.push(stage_row("TLS", "failed", error.clone())),
        Stage::NotAttempted { reason } => {
            if reason != "not applicable for DNS port" {
                rows.push(stage_row("TLS", "unsupported", reason.clone()))
            }
        }
    }

    match &report.http {
        Stage::Ok(d) => rows.push(stage_row(
            "HTTP",
            "ok",
            format!(
                "{} · status {:?} · {} B · {} ms",
                d.version, d.http_status, d.body_bytes, d.duration_ms
            ),
        )),
        Stage::Failed { error } => rows.push(stage_row("HTTP", "failed", error.clone())),
        Stage::NotAttempted { reason } => {
            if reason != "not applicable for DNS port" {
                rows.push(stage_row("HTTP", "unsupported", reason.clone()))
            }
        }
    }

    match &report.http3 {
        Stage::Ok(d) => {
            let status = if d.negotiated {
                "ok"
            } else if d.failure_category.as_deref() == Some("timeout") {
                "timeout"
            } else {
                "unsupported"
            };
            rows.push(stage_row(
                "HTTP/3",
                status,
                format!(
                    "advertised={} · negotiated={} — {}",
                    d.advertised, d.negotiated, d.note
                ),
            ));
        }
        Stage::Failed { error } => rows.push(stage_row("HTTP/3", "failed", error.clone())),
        Stage::NotAttempted { reason } => {
            if reason != "not applicable for DNS port" {
                rows.push(stage_row("HTTP/3", "unsupported", reason.clone()))
            }
        }
    }

    match &report.dot {
        Stage::Ok(d) => rows.push(stage_row(
            "DoT (853)",
            "ok",
            format!(
                "{} · {} · {} ms",
                d.protocol_version, d.cipher_suite, d.handshake_ms
            ),
        )),
        Stage::Failed { error } => rows.push(stage_row("DoT (853)", "failed", error.clone())),
        Stage::NotAttempted { reason } => {
            rows.push(stage_row("DoT (853)", "unsupported", reason.clone()))
        }
    }

    if !report.ports.is_empty() {
        let open: Vec<u16> = report
            .ports
            .iter()
            .filter(|p| p.status.starts_with("open"))
            .map(|p| p.port)
            .collect();
        let detail = if open.is_empty() {
            report
                .ports
                .iter()
                .map(|p| format!("{}:{}", p.port, p.status))
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            format!(
                "open: {:?} · {}",
                open,
                report
                    .ports
                    .iter()
                    .map(|p| format!("{}:{}", p.port, p.status))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        rows.push(stage_row(
            "Ports",
            if open.is_empty() { "unsupported" } else { "ok" },
            detail,
        ));

        // Surface UDP DNS (port 53) explicitly for dnstt diagnostics.
        if let Some(dns) = report
            .ports
            .iter()
            .find(|p| p.port == 53 && p.protocol == "udp")
        {
            let status = if dns.status.starts_with("open") {
                "ok"
            } else if dns.status.contains("timeout") {
                "timeout"
            } else {
                "unsupported"
            };
            rows.push(stage_row("UDP DNS (53)", status, dns.status.clone()));
        }
    }

    rows
}

fn apply_report(ui: &MainWindow, report: &Report) {
    ui.set_is_mock(false);
    ui.set_about_core_version(format!("wireglass {}", report.version).into());
    ui.set_field_error(SharedString::from(""));
    let ip = report.resolved_ip.as_deref().unwrap_or("—");
    // Short banner — long UTC strings ate vertical space on phones
    ui.set_banner(format!("Done · {} · ip {}", report.target, ip).into());
    ui.set_stages(ModelRc::new(VecModel::from(map_report(report))));
    let json =
        serde_json::to_string_pretty(report).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"));
    ui.set_report_json(json.into());
    ui.set_run_state("done".into());
}

fn clear_run_output(ui: &MainWindow) {
    ui.set_run_state("empty".into());
    ui.set_banner(SharedString::from(""));
    ui.set_field_error(SharedString::from(""));
    ui.set_is_mock(false);
    ui.set_report_json(SharedString::from(""));
    ui.set_show_json(false);
    ui.set_stages(ModelRc::new(VecModel::<StageRow>::from(vec![])));
}

static CANCEL_FLAG: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

fn compute_verdict(ui: &MainWindow) -> (String, slint::Color) {
    let stages = ui.get_stages();
    let mut core_ok = true;
    let mut any_failed = false;
    let mut cancelled = false;
    for i in 0..stages.row_count() {
        if let Some(r) = stages.row_data(i) {
            if r.detail == "cancelled" {
                cancelled = true;
            }
            if r.status == "failed" {
                any_failed = true;
                core_ok = false;
            }
            // Primary stages that must succeed for "all clear".
            if (r.name == "DNS" || r.name == "TCP" || r.name == "UDP DNS (53)") && r.status != "ok"
            {
                core_ok = false;
            }
        }
    }
    if cancelled {
        return ("Cancelled".into(), slint::Color::from_rgb_u8(139, 154, 171));
    }
    if any_failed {
        ("Failed".into(), slint::Color::from_rgb_u8(227, 93, 106))
    } else if core_ok {
        ("All clear".into(), slint::Color::from_rgb_u8(61, 214, 140))
    } else {
        ("Mixed".into(), slint::Color::from_rgb_u8(230, 180, 80))
    }
}

fn recent_path() -> std::path::PathBuf {
    #[cfg(target_os = "android")]
    {
        if let Some(dir) = android_bridge::files_dir() {
            return std::path::PathBuf::from(dir).join("recent.json");
        }
    }
    std::path::PathBuf::from("recent.json")
}

fn load_recent() -> Vec<String> {
    let p = recent_path();
    std::fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_recent(targets: &[String]) {
    let p = recent_path();
    if let Ok(json) = serde_json::to_string(targets) {
        let _ = std::fs::write(&p, json);
    }
}

fn wire_ui(ui: &MainWindow) {
    let ui_weak = ui.as_weak();
    // Back stack: Help opened from Check returns to Check, not always Home.
    let nav_stack = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));

    let navigate = {
        let ui_weak = ui_weak.clone();
        let nav_stack = nav_stack.clone();
        move |to: &str| {
            let Some(ui) = ui_weak.upgrade() else {
                return;
            };
            ui.set_show_exit_dialog(false);
            let from = ui.get_screen().to_string();
            if from != to {
                if let Ok(mut stack) = nav_stack.lock() {
                    stack.push(from);
                    // keep stack shallow
                    if stack.len() > 8 {
                        stack.remove(0);
                    }
                }
            }
            ui.set_screen(to.into());
        }
    };

    ui.on_go_home({
        let navigate = navigate.clone();
        let nav_stack = nav_stack.clone();
        move || {
            if let Ok(mut stack) = nav_stack.lock() {
                stack.clear();
            }
            navigate("home");
        }
    });

    // Navigate to Check without wiping prior results.
    ui.on_go_run({
        let navigate = navigate.clone();
        move || navigate("run")
    });

    ui.on_go_about({
        let navigate = navigate.clone();
        move || navigate("about")
    });

    ui.on_go_help({
        let navigate = navigate.clone();
        move || navigate("help")
    });

    ui.on_fill_example({
        let ui_weak = ui_weak.clone();
        move |example| {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_target_input(example);
                ui.set_field_error(SharedString::from(""));
            }
        }
    });

    ui.on_clear_results({
        let ui_weak = ui_weak.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                clear_run_output(&ui);
            }
        }
    });

    // Recent targets initial load
    ui.set_recent_targets(ModelRc::new(VecModel::from(
        load_recent()
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    )));

    ui.on_cancel_requested({
        let _ = ();
        move || {
            if let Ok(guard) = CANCEL_FLAG.lock() {
                if let Some(flag) = guard.as_ref() {
                    flag.store(true, Ordering::SeqCst);
                }
            }
        }
    });

    ui.on_share_requested({
        let ui_weak = ui_weak.clone();
        move || {
            println!("UI: share-requested fired");
            if let Some(ui) = ui_weak.upgrade() {
                let target = ui.get_target_input().to_string();
                let json = ui.get_report_json().to_string();
                let summary = format!("Wireglass result for {target}\\n{json}");
                #[cfg(target_os = "android")]
                {
                    match android_bridge::share_text(&summary) {
                        Ok(()) => ui.set_banner(SharedString::from("Share opened")),
                        Err(e) => {
                            // Fallback so the button is never a dead end.
                            match android_bridge::set_clipboard(&summary) {
                                Ok(()) => ui.set_banner(SharedString::from(format!(
                                    "Share unavailable ({e}); copied to clipboard"
                                ))),
                                Err(e2) => ui.set_banner(SharedString::from(format!(
                                    "Share failed: {e}; copy failed: {e2}"
                                ))),
                            }
                        }
                    }
                }
                #[cfg(not(target_os = "android"))]
                let _ = summary;
            }
        }
    });

    ui.on_copy_json({
        let ui_weak = ui_weak.clone();
        move || {
            println!("UI: copy-json fired");
            if let Some(ui) = ui_weak.upgrade() {
                let json = ui.get_report_json().to_string();
                #[cfg(target_os = "android")]
                {
                    match android_bridge::set_clipboard(&json) {
                        Ok(()) => ui.set_banner(SharedString::from("Copied JSON to clipboard")),
                        Err(e) => ui.set_banner(SharedString::from(format!("Copy failed: {e}"))),
                    }
                }
                #[cfg(not(target_os = "android"))]
                let _ = json;
            }
        }
    });

    ui.on_copy_target({
        let ui_weak = ui_weak.clone();
        move || {
            println!("UI: copy-target fired");
            if let Some(ui) = ui_weak.upgrade() {
                let t = ui.get_target_input().to_string();
                #[cfg(target_os = "android")]
                {
                    match android_bridge::set_clipboard(&t) {
                        Ok(()) => ui.set_banner(SharedString::from("Copied target to clipboard")),
                        Err(e) => ui.set_banner(SharedString::from(format!("Copy failed: {e}"))),
                    }
                }
                #[cfg(not(target_os = "android"))]
                let _ = t;
            }
        }
    });

    ui.on_save_requested({
        let ui_weak = ui_weak.clone();
        move || {
            println!("UI: save-requested fired");
            if let Some(ui) = ui_weak.upgrade() {
                let json = ui.get_report_json().to_string();
                if json.is_empty() {
                    ui.set_banner(SharedString::from("Nothing to save yet"));
                    return;
                }
                let path = {
                    #[cfg(target_os = "android")]
                    {
                        android_bridge::files_dir()
                            .map(|d| std::path::PathBuf::from(d).join("report.json"))
                    }
                    #[cfg(not(target_os = "android"))]
                    {
                        Some(std::path::PathBuf::from("report.json"))
                    }
                };
                match path {
                    Some(p) => match std::fs::write(&p, &json) {
                        Ok(()) => {
                            ui.set_banner(SharedString::from(format!("Saved to {}", p.display())))
                        }
                        Err(e) => ui.set_banner(SharedString::from(format!("Save failed: {e}"))),
                    },
                    None => ui.set_banner(SharedString::from("Save failed: files dir unavailable")),
                }
            }
        }
    });

    ui.on_open_github({
        move || {
            #[cfg(target_os = "android")]
            android_bridge::open_url("https://github.com/BTF-Kabir-2020/wireglass");
        }
    });

    ui.on_open_url({
        move |url| {
            #[cfg(target_os = "android")]
            android_bridge::open_url(&url);
            #[cfg(not(target_os = "android"))]
            let _ = url;
        }
    });

    ui.on_copy_text({
        let ui_weak = ui_weak.clone();
        move |text| {
            #[cfg(target_os = "android")]
            {
                let msg = match android_bridge::set_clipboard(&text) {
                    Ok(()) => format!("Copied: {text}"),
                    Err(e) => format!("Copy failed: {e}"),
                };
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_banner(SharedString::from(msg));
                }
            }
            #[cfg(not(target_os = "android"))]
            {
                let _ = text;
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_banner(SharedString::from("Copy is Android-only"));
                }
            }
        }
    });

    ui.on_recent_target_clicked({
        let ui_weak = ui_weak.clone();
        move |t| {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_target_input(t.clone());
                ui.set_field_error(SharedString::from(""));
            }
        }
    });

    ui.on_clear_recent({
        let ui_weak = ui_weak.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_recent_targets(ModelRc::new(VecModel::from(Vec::<SharedString>::new())));
                save_recent(&[]);
            }
        }
    });

    // Android Back / ← Back button
    ui.on_back_pressed({
        let ui_weak = ui_weak.clone();
        let nav_stack = nav_stack.clone();
        move || {
            let Some(ui) = ui_weak.upgrade() else {
                return;
            };
            if ui.get_show_exit_dialog() {
                ui.set_show_exit_dialog(false);
                return;
            }
            let prev = nav_stack.lock().ok().and_then(|mut s| s.pop());
            match prev {
                Some(screen) => ui.set_screen(screen.into()),
                None if ui.get_screen() == "home" => ui.set_show_exit_dialog(true),
                None => ui.set_screen("home".into()),
            }
        }
    });

    ui.on_exit_cancelled({
        let ui_weak = ui_weak.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_show_exit_dialog(false);
            }
        }
    });

    ui.on_exit_confirmed({
        move || {
            slint::quit_event_loop().ok();
        }
    });

    ui.on_run_requested({
        let ui_weak = ui_weak.clone();
        move || {
            let Some(ui) = ui_weak.upgrade() else {
                return;
            };
            if ui.get_run_state() == "loading" {
                return;
            }

            let target = match normalize_target(&ui.get_target_input()) {
                Ok(t) => {
                    ui.set_field_error(SharedString::from(""));
                    t
                }
                Err(msg) => {
                    ui.set_run_state("error".into());
                    ui.set_field_error(msg.clone().into());
                    ui.set_banner(msg.into());
                    ui.set_stages(ModelRc::new(VecModel::<StageRow>::from(vec![])));
                    ui.set_report_json(SharedString::from(""));
                    ui.set_show_json(false);
                    ui.set_verdict("".into());
                    return;
                }
            };

            // seed pending stages for live progress
            let pending = vec![
                stage_row("DNS", "pending", "Pending…"),
                stage_row("TCP", "pending", "Pending…"),
                stage_row("TLS", "pending", "Pending…"),
                stage_row("HTTP", "pending", "Pending…"),
                stage_row("HTTP/3", "pending", "Pending…"),
            ];
            ui.set_run_state("loading".into());
            ui.set_is_mock(false);
            ui.set_show_json(false);
            ui.set_banner("Checking…".into());
            ui.set_stages(ModelRc::new(VecModel::from(pending)));
            ui.set_report_json(SharedString::from(""));
            ui.set_verdict("".into());

            let cancel = Arc::new(AtomicBool::new(false));
            if let Ok(mut g) = CANCEL_FLAG.lock() {
                *g = Some(cancel.clone());
            }

            let ui_weak2 = ui_weak.clone();
            #[allow(clippy::type_complexity)]
            let on_stage: Option<Box<dyn FnMut(&str) + Send>> =
                Some(Box::new(move |stage: &str| {
                    let ui_weak = ui_weak2.clone();
                    let stage = stage.to_string();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            let idx = match stage.as_str() {
                                "dns" => 0,
                                "tcp" => 1,
                                "tls" => 2,
                                "http" => 3,
                                "http3" => 4,
                                _ => return,
                            };
                            let name = ["DNS", "TCP", "TLS", "HTTP", "HTTP/3"][idx];
                            ui.get_stages().set_row_data(
                                idx,
                                StageRow {
                                    name: name.into(),
                                    status: "running".into(),
                                    detail: "Running…".into(),
                                },
                            );
                        }
                    });
                }));

            let ui_weak = ui_weak.clone();
            thread::spawn(move || {
                let _ = rustls::crypto::ring::default_provider().install_default();
                let rt = match tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        let msg = format!("Could not start runtime: {e}");
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_run_state("error".into());
                                ui.set_banner(msg.into());
                            }
                        });
                        return;
                    }
                };

                let report = rt.block_on(wireglass::run_with_progress(
                    &target,
                    10,
                    1,
                    on_stage,
                    Some(cancel.clone()),
                ));
                let cancelled = cancel.load(Ordering::SeqCst);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        apply_report(&ui, &report);
                        let (v, c) = compute_verdict(&ui);
                        let verdict = if cancelled {
                            "Cancelled".to_string()
                        } else {
                            v
                        };
                        let color = if cancelled {
                            slint::Color::from_rgb_u8(139, 154, 171)
                        } else {
                            c
                        };
                        ui.set_verdict(verdict.clone().into());
                        ui.set_verdict_color(color);
                        if cancelled {
                            ui.set_banner("Cancelled".into());
                        }
                        // update last-result-line
                        let ip = report
                            .resolved_ip
                            .clone()
                            .unwrap_or_else(|| "—".to_string());
                        let line = format!("{} · {} · {}", report.target, ip, verdict);
                        ui.set_last_result_line(line.clone().into());
                        // recent targets
                        let mut recent: Vec<String> = (0..ui.get_recent_targets().row_count())
                            .filter_map(|i| ui.get_recent_targets().row_data(i))
                            .map(|s| s.to_string())
                            .collect();
                        if !recent.iter().any(|x| x == &report.target) {
                            recent.insert(0, report.target.clone());
                        }
                        recent.truncate(5);
                        ui.set_recent_targets(ModelRc::new(VecModel::from(
                            recent
                                .iter()
                                .cloned()
                                .map(SharedString::from)
                                .collect::<Vec<_>>(),
                        )));
                        save_recent(&recent);
                    }
                });
            });
        }
    });
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    #[cfg(target_os = "android")]
    android_bridge::store_app(app.clone());
    slint::android::init(app).expect("slint android init");
    let ui = MainWindow::new().expect("MainWindow");
    ui.set_about_core_version("wireglass 0.2.1".into());
    ui.set_is_mock(false);
    wire_ui(&ui);
    ui.run().expect("ui run");
}

#[cfg(not(target_os = "android"))]
pub fn run_desktop_shell() {
    let ui = MainWindow::new().expect("MainWindow");
    ui.set_about_core_version("wireglass 0.2.1".into());
    ui.set_is_mock(false);
    wire_ui(&ui);
    ui.run().expect("ui run");
}

#[cfg(test)]
mod tests {
    use super::normalize_target;

    #[test]
    fn accepts_domain() {
        assert_eq!(
            normalize_target("example.com").unwrap(),
            "https://example.com"
        );
    }

    #[test]
    fn accepts_https() {
        assert!(normalize_target("https://example.com/path").is_ok());
    }

    #[test]
    fn rejects_spaces() {
        assert!(normalize_target("exa mple.com").is_err());
    }

    #[test]
    fn rejects_ftp() {
        assert!(normalize_target("ftp://example.com").is_err());
    }

    #[test]
    fn accepts_ipv4() {
        assert_eq!(normalize_target("1.1.1.1").unwrap(), "https://1.1.1.1");
    }
}
