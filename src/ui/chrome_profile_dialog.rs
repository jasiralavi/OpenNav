use crate::data::{browser_repository, chrome_profiles};
use gtk4::prelude::*;
use gtk4::{Align, ApplicationWindow, Box as GtkBox, Label, ListView, Orientation, ScrolledWindow, SignalListItemFactory, SingleSelection, StringFilter, StringList};

pub fn show_profile_picker(
    parent: &ApplicationWindow,
    browser_id: String,
    url: String,
    initial_query: &str,
) {
    let profiles = std::rc::Rc::new(chrome_profiles::get_chrome_profiles());

    if profiles.is_empty() {
        let dialog = gtk4::MessageDialog::builder()
            .transient_for(parent)
            .modal(true)
            .text("No Chrome profiles found")
            .secondary_text("Open Google Chrome once and make sure its profiles exist in ~/.config/google-chrome.")
            .build();
        dialog.add_button("Close", gtk4::ResponseType::Close);
        dialog.connect_response(|d, _| d.close());
        dialog.present();
        return;
    }

    let dialog = gtk4::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("Chrome Profile")
        .default_width(500)
        .default_height(430)
        .decorated(false)
        .build();

    let vbox = GtkBox::new(Orientation::Vertical, 0);

    let search_entry = gtk4::Entry::builder()
        .placeholder_text("Search Chrome profiles…")
        .margin_top(15)
        .margin_bottom(15)
        .margin_start(15)
        .margin_end(15)
        .build();
    search_entry.set_text(initial_query);
    search_entry.set_position(-1);
    vbox.append(&search_entry);

    let names: Vec<&str> = profiles.iter().map(|p| p.name.as_str()).collect();
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
    if !initial_query.trim().is_empty() {
        filter.set_search(Some(initial_query.trim()));
    }

    let filter_model = gtk4::FilterListModel::builder()
        .model(&string_list)
        .filter(&filter)
        .incremental(true)
        .build();

    let selection = SingleSelection::new(Some(filter_model));
    selection.set_autoselect(true);

    let factory = SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let item = item.downcast_ref::<gtk4::ListItem>().unwrap();
        let row = GtkBox::new(Orientation::Horizontal, 12);
        row.set_css_classes(&["browser-row"]);

        let icon = gtk4::Image::builder()
            .icon_name("google-chrome")
            .pixel_size(32)
            .build();
        let label = Label::new(None);
        label.set_halign(Align::Start);
        label.set_hexpand(true);

        row.append(&icon);
        row.append(&label);
        item.set_child(Some(&row));
    });

    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk4::ListItem>().unwrap();
        let string_object = item.item().and_downcast::<gtk4::StringObject>().unwrap();
        let row = item.child().and_downcast::<GtkBox>().unwrap();
        if let Some(label) = row.last_child().and_downcast::<Label>() {
            label.set_text(string_object.string().as_str());
        }
    });

    let list_view = ListView::new(Some(selection.clone()), Some(factory));
    list_view.set_single_click_activate(true);

    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .min_content_height(300)
        .vexpand(true)
        .child(&list_view)
        .build();
    vbox.append(&scrolled);

    let hint = Label::new(Some("Enter to open  •  Esc to return"));
    hint.set_margin_bottom(12);
    hint.set_opacity(0.65);
    vbox.append(&hint);

    dialog.set_child(Some(&vbox));

    {
        let filter = filter.clone();
        search_entry.connect_changed(move |entry| {
            let text = entry.text();
            let q = text.trim();
            filter.set_search(if q.is_empty() { None } else { Some(q) });
        });
    }

    let launch_selected: std::rc::Rc<dyn Fn(bool)> = {
        let profiles = profiles.clone();
        let selection = selection.clone();
        let dialog_weak = dialog.downgrade();
        let parent_weak = parent.downgrade();
        let browser_id = browser_id.clone();
        let url = url.clone();

        std::rc::Rc::new(move |keep_open| {
            let Some(item) = selection.selected_item() else { return; };
            let Ok(string_object) = item.downcast::<gtk4::StringObject>() else { return; };
            let name = string_object.string();
            let Some(profile) = profiles.iter().find(|p| p.name == name.as_str()) else { return; };

            let _ = browser_repository::launch_chrome_profile(&browser_id, &profile.directory, &url);

            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.close();
            }
            if let Some(parent) = parent_weak.upgrade() {
                if keep_open {
                    parent.present();
                } else {
                    parent.close();
                }
            }
        })
    };

    {
        let launch_selected = launch_selected.clone();
        list_view.connect_activate(move |_, _| launch_selected(false));
    }

    {
        let launch_selected = launch_selected.clone();
        search_entry.connect_activate(move |_| launch_selected(false));
    }

    let key_controller = gtk4::EventControllerKey::new();
    {
        let dialog_weak = dialog.downgrade();
        let list_view_weak = list_view.downgrade();
        let search_entry_weak = search_entry.downgrade();
        let launch_selected = launch_selected.clone();
        key_controller.connect_key_pressed(move |_, key, _, modifiers| {
            if key == gtk4::gdk::Key::Escape {
                if let Some(dialog) = dialog_weak.upgrade() {
                    dialog.close();
                }
                return gtk4::glib::Propagation::Stop;
            }

            if key == gtk4::gdk::Key::Down {
                if let Some(list) = list_view_weak.upgrade() {
                    list.grab_focus();
                }
                return gtk4::glib::Propagation::Stop;
            }

            if key == gtk4::gdk::Key::Return || key == gtk4::gdk::Key::KP_Enter {
                launch_selected(modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK));
                return gtk4::glib::Propagation::Stop;
            }

            if let Some(entry) = search_entry_weak.upgrade() {
                if !entry.has_focus() {
                    if let Some(ch) = key.to_unicode() {
                        if !ch.is_control() {
                            entry.grab_focus();
                            let mut text = entry.text().to_string();
                            text.push(ch);
                            entry.set_text(&text);
                            entry.set_position(-1);
                            return gtk4::glib::Propagation::Stop;
                        }
                    }
                }
            }

            gtk4::glib::Propagation::Proceed
        });
    }
    dialog.add_controller(key_controller);

    dialog.present();
    search_entry.grab_focus();
}
