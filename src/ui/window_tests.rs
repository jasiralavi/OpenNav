use super::*;

fn descendants(widget: &gtk4::Widget) -> Vec<gtk4::Widget> {
    let mut result = vec![widget.clone()];
    let mut child = widget.first_child();
    while let Some(w) = child {
        result.extend(descendants(&w));
        child = w.next_sibling();
    }
    result
}

fn press(widget: &impl IsA<gtk4::Widget>, key: gdk::Key) {
    press_with_modifiers(widget, key, gdk::ModifierType::empty());
}

fn press_with_modifiers(
    widget: &impl IsA<gtk4::Widget>,
    key: gdk::Key,
    modifiers: gdk::ModifierType,
) {
    let controllers = widget.observe_controllers();
    for i in 0..controllers.n_items() {
        if let Some(c) = controllers
            .item(i)
            .and_downcast::<gtk4::EventControllerKey>()
        {
            if c.emit_by_name::<bool>("key-pressed", &[&key, &0u32, &modifiers]) {
                break;
            }
        }
    }
}

fn titled_window(title: &str) -> gtk4::Window {
    gtk4::Window::list_toplevels()
        .into_iter()
        .filter_map(|w| w.downcast::<gtk4::Window>().ok())
        .find(|w| w.title().as_deref() == Some(title) && w.is_visible())
        .unwrap()
}

fn button(window: &gtk4::Window, label: &str) -> gtk4::Button {
    descendants(window.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk4::Button>().ok())
        .find(|b| b.label().as_deref() == Some(label))
        .unwrap()
}

fn tooltip_button(window: &gtk4::Window, tooltip: &str) -> gtk4::Button {
    descendants(window.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk4::Button>().ok())
        .find(|b| b.tooltip_text().as_deref() == Some(tooltip))
        .unwrap()
}

fn entries(window: &gtk4::Window) -> Vec<gtk4::Entry> {
    descendants(window.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk4::Entry>().ok())
        .collect()
}

fn labeled_entry(window: &gtk4::Window, text: &str) -> gtk4::Entry {
    descendants(window.upcast_ref())
        .iter()
        .find_map(|w| {
            w.downcast_ref::<Label>()
                .filter(|l| l.text() == text)
                .and_then(|l| l.next_sibling().and_downcast::<gtk4::Entry>())
        })
        .unwrap()
}

fn assert_capture(path: &str, expected: &str) {
    for _ in 0..100 {
        if std::fs::read_to_string(path).ok().as_deref() == Some(expected) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(std::fs::read_to_string(path).unwrap_or_default(), expected);
}

fn screenshot(window: &impl IsA<gtk4::Window>, name: &str) {
    let Some(directory) = std::env::var_os("OPENNAV_TEST_SCREENSHOTS") else {
        return;
    };
    let window = window.as_ref();
    for _ in 0..20 {
        while gtk4::glib::MainContext::default().pending() {
            gtk4::glib::MainContext::default().iteration(false);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let paintable = gtk4::WidgetPaintable::new(Some(window));
    let snapshot = gtk4::Snapshot::new();
    paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
    let node = snapshot.to_node().expect("Window snapshot");
    let texture = window.renderer().unwrap().render_texture(&node, None);
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).unwrap();
    texture
        .save_to_png(directory.join(format!("{name}.png")))
        .unwrap();
}

#[test]
#[ignore = "Requires a display and isolated XDG_DATA_HOME; run explicitly"]
fn keyboard_navigation_preserves_selection() {
    assert!(
        std::env::var_os("OPENNAV_TEST_CAPTURE").is_some(),
        "Run through tests/run-ui.sh"
    );
    gtk4::init().unwrap();
    let app = Application::builder()
        .application_id("com.opennav.keyboard-test")
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.register(None::<&gtk4::gio::Cancellable>).unwrap();
    build_ui(&app, Some("g rust gtk"));
    let win = app.active_window().unwrap();
    let widgets = descendants(win.upcast_ref());
    let list = widgets
        .iter()
        .find_map(|w| w.clone().downcast::<ListView>().ok())
        .unwrap();
    let entry = widgets
        .iter()
        .find_map(|w| w.clone().downcast::<gtk4::Entry>().ok())
        .unwrap();
    let selection = list.model().unwrap().downcast::<SingleSelection>().unwrap();
    assert!(selection.n_items() >= 2, "Need two browser fixtures");
    selection.set_selected(1);
    list.grab_focus();
    press(&list, gdk::Key::Left);
    assert_eq!(selection.selected(), 1);
    let focus = gtk4::prelude::GtkWindowExt::focus(&win).unwrap();
    assert!(focus == *entry.upcast_ref::<gtk4::Widget>() || focus.is_ancestor(&entry));
    assert_eq!(entry.text(), "g rust gtk");
    press(&entry, gdk::Key::Down);
    assert_eq!(selection.selected(), 1);
    screenshot(&win, "main");
    // Each configured shortcut selects its browser even when its first character filters it out.
    let store = Store::new().unwrap();
    let config = crate::data::shortcuts::LauncherSettings::load(&store).unwrap();
    let browsers = browser_repository::get_installed_browsers();
    for (id, shortcut) in &config.browser_shortcuts {
        list.grab_focus();
        for c in shortcut.chars() {
            press(&win, gdk::Key::from_name(c.to_string()).unwrap());
        }
        let name = selection
            .selected_item()
            .unwrap()
            .downcast::<gtk4::StringObject>()
            .unwrap()
            .string();
        assert_eq!(
            name.as_str(),
            browsers.iter().find(|b| &b.id == id).unwrap().name
        );
    }

    // Settings edits are applied immediately and uppercase values are normalized.
    let settings_button = widgets
        .iter()
        .find_map(|w| {
            w.clone()
                .downcast::<gtk4::Button>()
                .ok()
                .filter(|b| b.tooltip_text().as_deref() == Some("Settings"))
        })
        .unwrap();
    settings_button.emit_clicked();
    let settings = titled_window("Settings");
    screenshot(&settings, "settings");
    let profile = labeled_entry(&settings, "Chrome profiles");
    profile.set_text("PP");
    button(&settings, "Save Shortcuts").emit_clicked();
    assert_eq!(profile.text(), "pp");
    assert_eq!(
        crate::data::shortcuts::LauncherSettings::load(&store)
            .unwrap()
            .chrome_profile_shortcut,
        "pp"
    );
    profile.set_text("gh");
    button(&settings, "Save Shortcuts").emit_clicked();
    assert_eq!(
        crate::data::shortcuts::LauncherSettings::load(&store)
            .unwrap()
            .chrome_profile_shortcut,
        "pp"
    );
    profile.set_text("pp");

    // Add a bookmark through the actual dialog, then reject a duplicate.
    tooltip_button(&settings, "Add Bookmark").emit_clicked();
    let editor = titled_window("Add Bookmark");
    let fields = entries(&editor);
    fields[0].set_text("Dashboard & Reports");
    fields[1].set_text("DP");
    fields[2].set_text("https://example.com/dashboard?a=1&b=2");
    screenshot(&editor, "bookmark-editor");
    button(&editor, "Save Bookmark").emit_clicked();
    assert_eq!(
        crate::data::shortcuts::LauncherSettings::load(&store)
            .unwrap()
            .bookmarks[0]
            .keyword,
        "dp"
    );
    tooltip_button(&settings, "Add Bookmark").emit_clicked();
    let editor = titled_window("Add Bookmark");
    let fields = entries(&editor);
    fields[0].set_text("Duplicate");
    fields[1].set_text("dp");
    fields[2].set_text("https://example.com");
    button(&editor, "Save Bookmark").emit_clicked();
    assert!(editor.is_visible());
    assert_eq!(
        crate::data::shortcuts::LauncherSettings::load(&store)
            .unwrap()
            .bookmarks
            .len(),
        1
    );
    editor.close();
    settings.close();

    list.grab_focus();
    press(&win, gdk::Key::period);
    assert_eq!(entry.text(), ".");
    let focus = gtk4::prelude::GtkWindowExt::focus(&win).unwrap();
    assert!(focus.is_ancestor(&entry));

    // Use fixture desktop launchers, never a real browser, and inspect exact arguments.
    let capture = std::env::var("OPENNAV_TEST_CAPTURE").expect("Run through tests/run-ui.sh");
    for (text, expected) in [
        (
            "g rust gtk -fx",
            "firefox\nhttps://www.google.com/search?q=rust+gtk\n",
        ),
        (
            ".dp -fx",
            "firefox\nhttps://example.com/dashboard?a=1&b=2\n",
        ),
        (
            ".dp -me",
            "microsoft-edge\nhttps://example.com/dashboard?a=1&b=2\n",
        ),
    ] {
        let _ = std::fs::remove_file(&capture);
        entry.set_text(text);
        entry.grab_focus();
        press_with_modifiers(&win, gdk::Key::Return, gdk::ModifierType::CONTROL_MASK);
        assert_capture(&capture, expected);
    }
    // The configured profile shortcut reaches the picker immediately.
    entry.set_text("g rust gtk");
    list.grab_focus();
    press(&win, gdk::Key::p);
    press(&win, gdk::Key::p);
    let profile_dialog = gtk4::Window::list_toplevels()
        .into_iter()
        .filter_map(|w| w.downcast::<gtk4::Window>().ok())
        .find(|w| w.transient_for().as_ref() == Some(&win))
        .expect("Profile dialog must open");
    profile_dialog.close();
    let _ = std::fs::remove_file(&capture);
    browser_repository::launch_chrome_profile(
        "google-chrome-test.desktop",
        "Profile 1",
        "g rust & gtk",
    )
    .unwrap();
    assert_capture(&capture, "google-chrome\n--profile-directory=Profile 1\nhttps://www.google.com/search?q=rust+%26+gtk\n");

    // Rhymezone's ampersands remain literal text in the engine settings row.
    let engine = crate::data::store::SearchEngine {
        keyword: "rz".into(),
        name: "RhymeZone & More".into(),
        url: "https://www.rhymezone.com/r/rhyme.cgi?Word={}&typeofrhyme=perfect".into(),
        icon_path: None,
    };
    store.add_engine(&engine).unwrap();
    let engines_ui = crate::ui::engines_dialog::build_engine_management_ui();
    assert!(descendants(engines_ui.upcast_ref())
        .iter()
        .filter_map(|w| w.downcast_ref::<Label>())
        .any(|label| label.text().contains(&engine.url)));

    // Editing and deleting bookmarks operate on persisted entries.
    settings_button.emit_clicked();
    let settings = titled_window("Settings");
    tooltip_button(&settings, "Edit Bookmark").emit_clicked();
    let editor = titled_window("Edit Bookmark");
    entries(&editor)[1].set_text("work");
    button(&editor, "Save Bookmark").emit_clicked();
    assert_eq!(
        crate::data::shortcuts::LauncherSettings::load(&store)
            .unwrap()
            .bookmarks[0]
            .keyword,
        "work"
    );
    let firefox_shortcut = labeled_entry(&settings, "firefox Test");
    firefox_shortcut.set_text("FF");
    button(&settings, "Save Shortcuts").emit_clicked();
    assert_eq!(firefox_shortcut.text(), "ff");
    assert_eq!(
        crate::data::shortcuts::LauncherSettings::load(&store)
            .unwrap()
            .browser_shortcuts["firefox-test.desktop"],
        "ff"
    );
    tooltip_button(&settings, "Delete Bookmark").emit_clicked();
    assert!(crate::data::shortcuts::LauncherSettings::load(&store)
        .unwrap()
        .bookmarks
        .is_empty());
    settings.close();

    list.grab_focus();
    press(&win, gdk::Key::f);
    press(&win, gdk::Key::f);
    assert_eq!(
        selection
            .selected_item()
            .unwrap()
            .downcast::<gtk4::StringObject>()
            .unwrap()
            .string(),
        "firefox Test"
    );
    let _ = std::fs::remove_file(&capture);
    entry.set_text("g rust gtk -ff");
    entry.grab_focus();
    press_with_modifiers(&win, gdk::Key::Return, gdk::ModifierType::CONTROL_MASK);
    assert_capture(
        &capture,
        "firefox\nhttps://www.google.com/search?q=rust+gtk\n",
    );
    // Unknown bookmarks offer an optional inline add flow, keeping the original request.
    let _ = std::fs::remove_file(&capture);
    entry.set_text(".fir -ff");
    entry.grab_focus();
    press(&win, gdk::Key::Return);
    let editor = titled_window("Add Bookmark");
    let fields = entries(&editor);
    assert_eq!(fields[1].text(), "fir");
    assert!(descendants(editor.upcast_ref())
        .iter()
        .filter_map(|w| w.downcast_ref::<Label>())
        .any(|label| label.text().contains("not available")));
    screenshot(&editor, "missing-bookmark");
    press_with_modifiers(&editor, gdk::Key::Return, gdk::ModifierType::CONTROL_MASK);
    assert!(editor.is_visible());
    assert!(crate::data::shortcuts::LauncherSettings::load(&store)
        .unwrap()
        .bookmarks
        .is_empty());
    fields[0].set_text("Firefox resources");
    fields[2].set_text("invalid-link");
    press_with_modifiers(&editor, gdk::Key::Return, gdk::ModifierType::CONTROL_MASK);
    assert!(editor.is_visible());
    fields[2].set_text("https://example.com/firefox?a=1&b=2");
    press_with_modifiers(&editor, gdk::Key::Return, gdk::ModifierType::CONTROL_MASK);
    assert!(!editor.is_visible());
    assert_eq!(
        crate::data::shortcuts::LauncherSettings::load(&store)
            .unwrap()
            .bookmarks[0]
            .keyword,
        "fir"
    );
    assert!(
        !std::path::Path::new(&capture).exists(),
        "Adding should save without launching"
    );
    assert_eq!(entry.text(), ".fir -ff");
    entry.grab_focus();
    press_with_modifiers(&win, gdk::Key::Return, gdk::ModifierType::CONTROL_MASK);
    assert_capture(&capture, "firefox\nhttps://example.com/firefox?a=1&b=2\n");
    for use_escape in [true, false] {
        entry.set_text(".cancel -ff");
        entry.grab_focus();
        press(&win, gdk::Key::Return);
        let editor = titled_window("Add Bookmark");
        if use_escape {
            press(&editor, gdk::Key::Escape);
        } else {
            button(&editor, "Cancel").emit_clicked();
        }
        assert!(!editor.is_visible());
        assert_eq!(
            crate::data::shortcuts::LauncherSettings::load(&store)
                .unwrap()
                .bookmarks
                .len(),
            1
        );
    }
    entry.set_text(".click -ff");
    entry.grab_focus();
    press(&win, gdk::Key::Return);
    let editor = titled_window("Add Bookmark");
    let fields = entries(&editor);
    fields[0].set_text("Click to add");
    fields[2].set_text("https://example.com/click");
    button(&editor, "Add").emit_clicked();
    assert!(!editor.is_visible());
    assert_eq!(
        crate::data::shortcuts::LauncherSettings::load(&store)
            .unwrap()
            .bookmarks
            .len(),
        2
    );

    win.close();
}
