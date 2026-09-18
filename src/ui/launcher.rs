use crate::data::{
    browser_repository::{self, Browser},
    input,
    shortcuts::LauncherSettings,
    store::Store,
};
use gtk4::{prelude::*, ApplicationWindow};

/// Shared by Enter, row clicks and the Chrome-profile shortcut.
pub fn launch(
    window: &ApplicationWindow,
    selected: &Browser,
    browsers: &[Browser],
    text: &str,
    keep_open: bool,
) {
    let result = (|| -> anyhow::Result<()> {
        let store = Store::new()?;
        let config = LauncherSettings::load(&store)?;
        let request = input::resolve_request(
            text,
            &selected.id,
            browsers,
            &config,
            &store.list_engines()?,
            &store
                .get_setting("search_engine")?
                .unwrap_or_else(|| "g".into()),
        )?;
        let browser = browsers
            .iter()
            .find(|b| b.id == request.browser_id)
            .ok_or_else(|| anyhow::anyhow!("Browser is no longer available."))?;
        if browser_repository::is_google_chrome(browser) {
            crate::ui::chrome_profile_dialog::show_profile_picker(
                window,
                browser.id.clone(),
                request.target,
                "",
            );
            return Ok(());
        }
        browser_repository::launch_browser(&browser.id, &request.target)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        store.increment_usage(&browser.id)?;
        if keep_open {
            let weak = window.downgrade();
            gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
                if let Some(win) = weak.upgrade() {
                    win.present();
                }
            });
        } else {
            window.close();
        }
        Ok(())
    })();
    if let Err(error) = result {
        gtk4::AlertDialog::builder()
            .message("Unable to open this request")
            .detail(error.to_string())
            .build()
            .show(Some(window));
    }
}
