use crate::data::browser_repository;
use crate::data::store::Store;
use crate::ui::chrome_profile_dialog;
use gtk4::gdk;
use gtk4::glib::WeakRef;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, FilterListModel, Label, ListView,
    Orientation, ScrolledWindow, SignalListItemFactory, SingleSelection, StringFilter, StringList,
};

fn update_label_markup(label: &Label, text: &str, query: &str) {
    if query.is_empty() {
        label.set_markup(&gtk4::glib::markup_escape_text(text));
        return;
    }

    let query_lower = query.to_lowercase();
    let text_lower = text.to_lowercase();
    if let Some(idx) = text_lower.find(&query_lower) {
        let end = idx + query_lower.len();
        let before = gtk4::glib::markup_escape_text(&text[..idx]);
        let matched = gtk4::glib::markup_escape_text(&text[idx..end]);
        let after = gtk4::glib::markup_escape_text(&text[end..]);
        label.set_markup(&format!(
            "{}<span foreground='#f0e68c' underline='single'>{}</span>{}",
            before, matched, after
        ));
    } else {
        label.set_markup(&gtk4::glib::markup_escape_text(text));
    }
}

fn refresh_rows(
    rows: &std::rc::Rc<std::cell::RefCell<Vec<WeakRef<GtkBox>>>>,
    query: &str,
    pinned_map: &std::collections::HashMap<String, bool>,
) {
    let mut live = Vec::new();
    for weak in rows.borrow().iter() {
        if let Some(hbox) = weak.upgrade() {
            if let Some(label) = hbox
                .first_child()
                .and_then(|w| w.next_sibling())
                .and_downcast::<Label>()
            {
                let name = label.text().to_string();
                update_label_markup(&label, &name, query);
                if let Some(pin_btn) = label.next_sibling().and_downcast::<gtk4::Button>() {
                    if *pinned_map.get(&name).unwrap_or(&false) {
                        pin_btn.add_css_class("pinned");
                    } else {
                        pin_btn.remove_css_class("pinned");
                    }
                }
            }
            live.push(weak.clone());
        }
    }
    *rows.borrow_mut() = live;
}

pub fn build_ui(app: &Application, url_to_open: Option<&str>) {
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(include_str!("../../resources/style.css"));
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    crate::data::icons::fetch_missing_icons();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("OpenNav")
        .default_width(500)
        .default_height(500)
        .modal(true)
        .decorated(false)
        .build();
    window.set_icon_name(Some("opennav"));

    let vbox = GtkBox::new(Orientation::Vertical, 0);

    let url_entry = gtk4::Entry::builder()
        .placeholder_text("Open URL or Search...")
        .margin_top(15)
        .margin_bottom(15)
        .margin_start(15)
        .margin_end(15)
        .build();
    if let Some(url) = url_to_open {
        url_entry.set_text(url);
        url_entry.set_position(-1);
    }
    vbox.append(&url_entry);

    let store = Store::new().ok();
    let mut browsers = browser_repository::get_installed_browsers();
    if let Some(ref s) = store {
        if let Ok(stats) = s.get_stats() {
            use std::collections::HashMap;
            let stat_map: HashMap<String, (i64, bool, i64)> = stats
                .into_iter()
                .map(|(id, count, pinned, last)| (id, (count, pinned, last)))
                .collect();
            let sort_mode = s
                .get_setting("sort_order")
                .ok()
                .flatten()
                .unwrap_or_else(|| "freq".to_string());

            for browser in &mut browsers {
                if let Some((_, pinned, _)) = stat_map.get(&browser.id) {
                    browser.is_pinned = *pinned;
                }
            }

            browsers.sort_by(|a, b| {
                b.is_pinned
                    .cmp(&a.is_pinned)
                    .then_with(|| match sort_mode.as_str() {
                        "recent" => stat_map
                            .get(&b.id)
                            .map(|x| x.2)
                            .unwrap_or(0)
                            .cmp(&stat_map.get(&a.id).map(|x| x.2).unwrap_or(0)),
                        "alpha" => std::cmp::Ordering::Equal,
                        _ => stat_map
                            .get(&b.id)
                            .map(|x| x.0)
                            .unwrap_or(0)
                            .cmp(&stat_map.get(&a.id).map(|x| x.0).unwrap_or(0)),
                    })
                    .then_with(|| a.name.cmp(&b.name))
            });
        }
    }

    let browsers = std::rc::Rc::new(browsers);
    let chrome_browser = browsers
        .iter()
        .find(|b| browser_repository::is_google_chrome(b))
        .cloned();

    let pinned_map = std::rc::Rc::new(std::cell::RefCell::new(
        browsers
            .iter()
            .map(|b| (b.name.clone(), b.is_pinned))
            .collect::<std::collections::HashMap<_, _>>(),
    ));
    let icon_map = std::rc::Rc::new(
        browsers
            .iter()
            .map(|b| (b.name.clone(), b.icon.clone()))
            .collect::<std::collections::HashMap<_, _>>(),
    );
    let search_query = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    let active_rows = std::rc::Rc::new(std::cell::RefCell::new(Vec::<WeakRef<GtkBox>>::new()));

    let names: Vec<&str> = browsers.iter().map(|b| b.name.as_str()).collect();
    let string_list = StringList::new(&names);
    let filter = StringFilter::builder()
        .match_mode(gtk4::StringFilterMatchMode::Substring)
        .ignore_case(true)
        .build();
    filter.set_expression(Some(gtk4::PropertyExpression::new(
        gtk4::StringObject::static_type(),
        None::<&gtk4::Expression>,
        "string",
    )));
    let filter_model = FilterListModel::builder()
        .model(&string_list)
        .filter(&filter)
        .incremental(true)
        .build();
    let selection = SingleSelection::new(Some(filter_model));
    selection.set_autoselect(true);

    let factory = SignalListItemFactory::new();
    {
        let browsers = browsers.clone();
        let pinned_map = pinned_map.clone();
        let active_rows = active_rows.clone();
        let search_query = search_query.clone();
        factory.connect_setup(move |_, item| {
            let item = item.downcast_ref::<gtk4::ListItem>().unwrap();
            let row = GtkBox::new(Orientation::Horizontal, 12);
            row.set_css_classes(&["browser-row"]);

            let icon = gtk4::Image::builder().pixel_size(32).build();
            let label = Label::new(None);
            label.set_halign(Align::Start);
            label.set_hexpand(true);
            label.set_use_markup(true);
            let pin_btn = gtk4::Button::builder()
                .icon_name("view-pin-symbolic")
                .css_classes(vec!["pin-btn".to_string()])
                .has_frame(false)
                .build();
            let hidden = Label::new(None);
            hidden.set_visible(false);

            {
                let browsers = browsers.clone();
                let pinned_map = pinned_map.clone();
                let active_rows = active_rows.clone();
                let search_query = search_query.clone();
                pin_btn.connect_clicked(move |btn| {
                    let Some(row) = btn.parent().and_downcast::<GtkBox>() else { return; };
                    let Some(hidden) = row.last_child().and_downcast::<Label>() else { return; };
                    let name = hidden.text();
                    let Some(browser) = browsers.iter().find(|b| b.name == name) else { return; };
                    if let Ok(store) = Store::new() {
                        if let Ok(new_state) = store.toggle_pin(&browser.id) {
                            pinned_map.borrow_mut().insert(browser.name.clone(), new_state);
                            refresh_rows(&active_rows, &search_query.borrow(), &pinned_map.borrow());
                        }
                    }
                });
            }

            row.append(&icon);
            row.append(&label);
            row.append(&pin_btn);
            row.append(&hidden);
            item.set_child(Some(&row));
        });
    }

    {
        let icon_map = icon_map.clone();
        let pinned_map = pinned_map.clone();
        let search_query = search_query.clone();
        let active_rows = active_rows.clone();
        factory.connect_bind(move |_, item| {
            let item = item.downcast_ref::<gtk4::ListItem>().unwrap();
            let object = item.item().and_downcast::<gtk4::StringObject>().unwrap();
            let name = object.string();
            let row = item.child().and_downcast::<GtkBox>().unwrap();
            active_rows.borrow_mut().push(row.downgrade());

            if let Some(hidden) = row.last_child().and_downcast::<Label>() {
                hidden.set_text(name.as_str());
            }
            if let Some(icon) = row.first_child().and_downcast::<gtk4::Image>() {
                let icon_name = icon_map
                    .get(name.as_str())
                    .cloned()
                    .unwrap_or_else(|| "web-browser".to_string());
                icon.set_icon_name(Some(&icon_name));
            }
            if let Some(label) = row
                .first_child()
                .and_then(|w| w.next_sibling())
                .and_downcast::<Label>()
            {
                label.set_text(name.as_str());
                update_label_markup(&label, name.as_str(), &search_query.borrow());
                if let Some(pin_btn) = label.next_sibling().and_downcast::<gtk4::Button>() {
                    if *pinned_map.borrow().get(name.as_str()).unwrap_or(&false) {
                        pin_btn.add_css_class("pinned");
                    } else {
                        pin_btn.remove_css_class("pinned");
                    }
                }
            }
        });
    }

    let list_view = ListView::new(Some(selection.clone()), Some(factory));
    list_view.set_single_click_activate(true);
    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .min_content_height(300)
        .vexpand(true)
        .child(&list_view)
        .build();
    vbox.append(&scrolled);

    let status_box = GtkBox::new(Orientation::Horizontal, 10);
    status_box.set_halign(Align::Center);
    status_box.set_margin_bottom(10);
    status_box.set_opacity(0.7);

    let help_btn = gtk4::Button::builder()
        .icon_name("help-about-symbolic")
        .has_frame(false)
        .tooltip_text("Shortcuts")
        .build();
    let settings_btn = gtk4::Button::builder()
        .icon_name("emblem-system-symbolic")
        .has_frame(false)
        .tooltip_text("Settings")
        .build();
    status_box.append(&help_btn);
    status_box.append(&settings_btn);
    vbox.append(&status_box);
    window.set_child(Some(&vbox));

    let launch_browser: std::rc::Rc<dyn Fn(String, bool)> = {
        let browsers = browsers.clone();
        let window_weak = window.downgrade();
        let url_entry_weak = url_entry.downgrade();
        std::rc::Rc::new(move |name, keep_open| {
            let Some(browser) = browsers.iter().find(|b| b.name == name) else { return; };
            let target_url = url_entry_weak
                .upgrade()
                .map(|e| e.text().to_string())
                .unwrap_or_default();

            if let Ok(store) = Store::new() {
                let _ = store.increment_usage(&browser.id);
            }

            if browser_repository::is_google_chrome(browser) {
                if let Some(parent) = window_weak.upgrade() {
                    chrome_profile_dialog::show_profile_picker(
                        &parent,
                        browser.id.clone(),
                        target_url,
                        "",
                    );
                }
                return;
            }

            let _ = browser_repository::launch_browser(&browser.id, &target_url);
            if let Some(window) = window_weak.upgrade() {
                if keep_open {
                    window.present();
                } else {
                    window.close();
                }
            }
        })
    };

    {
        let selection = selection.clone();
        let launch_browser = launch_browser.clone();
        list_view.connect_activate(move |_, _| {
            if let Some(item) = selection.selected_item() {
                if let Ok(object) = item.downcast::<gtk4::StringObject>() {
                    launch_browser(object.string().to_string(), false);
                }
            }
        });
    }

    {
        let window_weak = window.downgrade();
        help_btn.connect_clicked(move |_| {
            let Some(parent) = window_weak.upgrade() else { return; };
            let dialog = gtk4::Window::builder()
                .transient_for(&parent)
                .modal(true)
                .title("Shortcuts")
                .default_width(360)
                .default_height(340)
                .build();
            let box_ = GtkBox::new(Orientation::Vertical, 10);
            box_.set_margin_top(20);
            box_.set_margin_bottom(20);
            box_.set_margin_start(20);
            box_.set_margin_end(20);
            for (key, desc) in [
                ("Type", "Search browsers"),
                ("cp", "Search Chrome profiles"),
                ("Enter", "Open selected browser/profile"),
                ("Ctrl + Enter", "Open and keep OpenNav open"),
                ("Ctrl + L", "Focus URL bar"),
                ("Ctrl + P", "Toggle browser pin"),
                ("Ctrl + S", "Settings"),
                ("Esc", "Clear search / close"),
            ] {
                let row = GtkBox::new(Orientation::Horizontal, 20);
                let k = Label::new(None);
                k.set_markup(&format!("<b>{}</b>", key));
                k.set_width_chars(14);
                k.set_halign(Align::Start);
                let d = Label::new(Some(desc));
                d.set_halign(Align::Start);
                row.append(&k);
                row.append(&d);
                box_.append(&row);
            }
            dialog.set_child(Some(&box_));
            dialog.present();
        });
    }

    {
        let window_weak = window.downgrade();
        settings_btn.connect_clicked(move |_| {
            let Some(parent) = window_weak.upgrade() else { return; };
            let dialog = gtk4::Window::builder()
                .transient_for(&parent)
                .modal(true)
                .title("Settings")
                .default_width(600)
                .default_height(520)
                .build();
            let box_ = GtkBox::new(Orientation::Vertical, 16);
            box_.set_margin_top(20);
            box_.set_margin_bottom(20);
            box_.set_margin_start(20);
            box_.set_margin_end(20);

            let title = Label::new(Some("<b>Browser List Order</b>"));
            title.set_use_markup(true);
            title.set_halign(Align::Start);
            box_.append(&title);

            let sort_items = ["Alphabetical", "Recently Used", "Frequently Used"];
            let model = StringList::new(&sort_items);
            let dropdown = gtk4::DropDown::new(Some(model), None::<&gtk4::Expression>);
            let current = Store::new()
                .ok()
                .and_then(|s| s.get_setting("sort_order").ok().flatten())
                .unwrap_or_else(|| "freq".to_string());
            dropdown.set_selected(match current.as_str() {
                "alpha" => 0,
                "recent" => 1,
                _ => 2,
            });
            dropdown.connect_selected_notify(|d| {
                let value = match d.selected() {
                    0 => "alpha",
                    1 => "recent",
                    _ => "freq",
                };
                if let Ok(store) = Store::new() {
                    let _ = store.set_setting("sort_order", value);
                }
            });
            box_.append(&dropdown);
            box_.append(&gtk4::Separator::new(Orientation::Horizontal));

            let engines = crate::ui::engines_dialog::build_engine_management_ui();
            engines.set_vexpand(true);
            box_.append(&engines);
            dialog.set_child(Some(&box_));
            dialog.present();
        });
    }

    let key_controller = gtk4::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    {
        let selection = selection.clone();
        let window_weak = window.downgrade();
        let url_entry_weak = url_entry.downgrade();
        let list_view_weak = list_view.downgrade();
        let filter = filter.clone();
        let search_query = search_query.clone();
        let active_rows = active_rows.clone();
        let pinned_map = pinned_map.clone();
        let browsers = browsers.clone();
        let chrome_browser = chrome_browser.clone();
        let launch_browser = launch_browser.clone();
        let help_btn_weak = help_btn.downgrade();
        let settings_btn_weak = settings_btn.downgrade();

        key_controller.connect_key_pressed(move |_, key, _, modifiers| {
            let Some(window) = window_weak.upgrade() else {
                return gtk4::glib::Propagation::Proceed;
            };

            if let Some(focus) = gtk4::prelude::GtkWindowExt::focus(&window) {
                if let Some(entry) = url_entry_weak.upgrade() {
                    let widget = entry.upcast_ref::<gtk4::Widget>();
                    if &focus == widget || focus.is_ancestor(widget) {
                        if key == gtk4::gdk::Key::Down {
                            if let Some(list) = list_view_weak.upgrade() {
                                list.grab_focus();
                                return gtk4::glib::Propagation::Stop;
                            }
                        }
                        return gtk4::glib::Propagation::Proceed;
                    }
                }
            }

            if key == gtk4::gdk::Key::Escape {
                if !search_query.borrow().is_empty() {
                    search_query.borrow_mut().clear();
                    filter.set_search(None::<&str>);
                    refresh_rows(&active_rows, "", &pinned_map.borrow());
                } else {
                    window.close();
                }
                return gtk4::glib::Propagation::Stop;
            }

            if modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                if key == gtk4::gdk::Key::l {
                    if let Some(entry) = url_entry_weak.upgrade() {
                        entry.grab_focus();
                        entry.select_region(0, -1);
                    }
                    return gtk4::glib::Propagation::Stop;
                }
                if key == gtk4::gdk::Key::s {
                    if let Some(btn) = settings_btn_weak.upgrade() {
                        btn.emit_clicked();
                    }
                    return gtk4::glib::Propagation::Stop;
                }
                if key == gtk4::gdk::Key::question || key == gtk4::gdk::Key::slash {
                    if let Some(btn) = help_btn_weak.upgrade() {
                        btn.emit_clicked();
                    }
                    return gtk4::glib::Propagation::Stop;
                }
                if key == gtk4::gdk::Key::p {
                    if let Some(item) = selection.selected_item() {
                        if let Ok(object) = item.downcast::<gtk4::StringObject>() {
                            if let Some(browser) = browsers.iter().find(|b| b.name == object.string()) {
                                if let Ok(store) = Store::new() {
                                    if let Ok(state) = store.toggle_pin(&browser.id) {
                                        pinned_map.borrow_mut().insert(browser.name.clone(), state);
                                        refresh_rows(
                                            &active_rows,
                                            &search_query.borrow(),
                                            &pinned_map.borrow(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    return gtk4::glib::Propagation::Stop;
                }
            }

            if key == gtk4::gdk::Key::Return || key == gtk4::gdk::Key::KP_Enter {
                if let Some(item) = selection.selected_item() {
                    if let Ok(object) = item.downcast::<gtk4::StringObject>() {
                        launch_browser(
                            object.string().to_string(),
                            modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK),
                        );
                    }
                }
                return gtk4::glib::Propagation::Stop;
            }

            if key == gtk4::gdk::Key::BackSpace {
                if !search_query.borrow().is_empty() {
                    search_query.borrow_mut().pop();
                    let q = search_query.borrow().clone();
                    filter.set_search(if q.is_empty() { None } else { Some(q.as_str()) });
                    refresh_rows(&active_rows, &q, &pinned_map.borrow());
                }
                return gtk4::glib::Propagation::Stop;
            }

            if let Some(ch) = key.to_unicode() {
                if ch.is_control() {
                    return gtk4::glib::Propagation::Proceed;
                }

                search_query.borrow_mut().push(ch);
                let q = search_query.borrow().clone();

                if q.eq_ignore_ascii_case("cp") {
                    search_query.borrow_mut().clear();
                    filter.set_search(None::<&str>);
                    refresh_rows(&active_rows, "", &pinned_map.borrow());
                    if let Some(chrome) = chrome_browser.as_ref() {
                        let target_url = url_entry_weak
                            .upgrade()
                            .map(|e| e.text().to_string())
                            .unwrap_or_default();
                        chrome_profile_dialog::show_profile_picker(
                            &window,
                            chrome.id.clone(),
                            target_url,
                            "",
                        );
                    }
                    return gtk4::glib::Propagation::Stop;
                }

                filter.set_search(Some(q.as_str()));
                refresh_rows(&active_rows, &q, &pinned_map.borrow());
                if let Some(list) = list_view_weak.upgrade() {
                    list.grab_focus();
                }
                return gtk4::glib::Propagation::Stop;
            }

            gtk4::glib::Propagation::Proceed
        });
    }
    window.add_controller(key_controller);

    window.present();
    list_view.grab_focus();
}
