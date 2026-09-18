use crate::data::{browser_repository, shortcuts::LauncherSettings, store::Store};
use gtk4::{prelude::*, Align, Box as GtkBox, Button, Entry, Label, Orientation};

pub fn build_shortcut_settings() -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 10);
    let title = Label::new(Some("Browser Shortcuts"));
    title.add_css_class("heading");
    title.set_halign(Align::Start);
    container.append(&title);
    let hint = Label::new(Some("Use two letters. Search-engine aliases are reserved."));
    hint.set_halign(Align::Start);
    hint.add_css_class("dim-label");
    container.append(&hint);
    let status = Label::new(None);
    status.set_wrap(true);
    status.set_halign(Align::Start);
    let result = (|| -> anyhow::Result<_> {
        let store = Store::new()?;
        let config = LauncherSettings::load(&store)?;
        Ok(config)
    })();
    let config = match result {
        Ok(c) => c,
        Err(e) => {
            status.set_text(&e.to_string());
            container.append(&status);
            return container;
        }
    };
    let profile = shortcut_row(
        &container,
        "Chrome profiles",
        &config.chrome_profile_shortcut,
    );
    let browsers = browser_repository::get_installed_browsers();
    let mut entries = Vec::new();
    for browser in browsers {
        if let Some(shortcut) = config.browser_shortcuts.get(&browser.id) {
            let entry = shortcut_row(&container, &browser.name, shortcut);
            entries.push((browser.id, entry));
        }
    }
    let save = Button::with_label("Save Shortcuts");
    save.add_css_class("suggested-action");
    save.set_halign(Align::End);
    container.append(&save);
    container.append(&status);
    save.connect_clicked(move |_| {
        let result = (|| -> anyhow::Result<_> {
            let store = Store::new()?;
            let mut config = LauncherSettings::load(&store)?;
            config.chrome_profile_shortcut = profile.text().to_string();
            for (id, entry) in &entries {
                config
                    .browser_shortcuts
                    .insert(id.clone(), entry.text().to_string());
            }
            config.save(&store)
        })();
        match result {
            Ok(config) => {
                profile.set_text(&config.chrome_profile_shortcut);
                for (id, entry) in &entries {
                    entry.set_text(&config.browser_shortcuts[id]);
                }
                status.set_text("Shortcuts saved.");
            }
            Err(e) => status.set_text(&e.to_string()),
        }
    });
    container
}

fn shortcut_row(container: &GtkBox, name: &str, value: &str) -> Entry {
    let row = GtkBox::new(Orientation::Horizontal, 10);
    let label = Label::new(Some(name));
    label.set_halign(Align::Start);
    label.set_hexpand(true);
    row.append(&label);
    let entry = Entry::builder().text(value).width_chars(4).build();
    entry.set_tooltip_text(Some("Exactly two letters (a–z)"));
    row.append(&entry);
    container.append(&row);
    entry
}
