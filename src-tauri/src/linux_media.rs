//! WebKitGTK ships media streams disabled by default and never shows a
//! native permission prompt for `getUserMedia` — every call is silently
//! auto-denied (`NotAllowedError`) unless the embedding app explicitly
//! enables media-stream support on the WebView's settings and answers
//! the `permission-request` signal itself. Desktop-Linux-only: WebView2
//! (Windows) prompts natively, and WKWebView (macOS) has its own
//! separate story that isn't solved by this.

#[cfg(target_os = "linux")]
pub fn allow_user_media(window: &tauri::WebviewWindow) {
    use gtk::glib::Cast;
    use webkit2gtk::{PermissionRequestExt, SettingsExt, WebViewExt};

    let _ = window.with_webview(|webview| {
        let webview = webview.inner();

        if let Some(settings) = WebViewExt::settings(&webview) {
            settings.set_enable_media_stream(true);
            settings.set_enable_webaudio(true);
        }

        webview.connect_permission_request(|_, request| {
            if request
                .downcast_ref::<webkit2gtk::UserMediaPermissionRequest>()
                .is_some()
            {
                request.allow();
                true
            } else {
                false
            }
        });
    });
}

#[cfg(not(target_os = "linux"))]
pub fn allow_user_media(_window: &tauri::WebviewWindow) {}
