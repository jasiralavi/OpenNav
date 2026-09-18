use crate::data::{
    shortcuts::{Bookmark, LauncherSettings},
    store::Store,
};
use gtk4::{
    prelude::*, Align, Box as GtkBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation,
    Window,
};

pub fn build_bookmark_settings() -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 10);
    let toolbar = GtkBox::new(Orientation::Horizontal, 10);
    let title = Label::new(Some("Bookmarks"));
    title.add_css_class("heading");
    title.set_halign(Align::Start);
    title.set_hexpand(true);
    toolbar.append(&title);
    let add = Button::with_label("Add");
    add.add_css_class("suggested-action");
    add.set_tooltip_text(Some("Add Bookmark"));
    add.set_width_request(100);
    toolbar.append(&add);
    container.append(&toolbar);
    let hint = Label::new(Some("Type .keyword to open; append -fx to use Firefox."));
    hint.set_halign(Align::Start);
    hint.set_wrap(true);
    hint.add_css_class("dim-label");
    container.append(&hint);
    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    list.add_css_class("frame");
    container.append(&list);
    let status = Label::new(None);
    status.set_wrap(true);
    container.append(&status);
    populate(&list, &status);
    add.connect_clicked(move |button| {
        if let Some(parent) = button.root().and_downcast::<Window>() {
            show_editor(&parent, &list, &status, None);
        }
    });
    container
}

fn populate(list: &ListBox, status: &Label) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let result = Store::new()
        .map_err(anyhow::Error::from)
        .and_then(|s| LauncherSettings::load(&s));
    let config = match result {
        Ok(c) => c,
        Err(e) => {
            status.set_text(&e.to_string());
            return;
        }
    };
    if config.bookmarks.is_empty() {
        list.append(&Label::new(Some("No bookmarks yet.")));
    }
    for bookmark in config.bookmarks {
        let row = ListBoxRow::new();
        let content = GtkBox::new(Orientation::Horizontal, 12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        let info = GtkBox::new(Orientation::Vertical, 2);
        info.set_hexpand(true);
        let name = Label::new(Some(&bookmark.name));
        name.set_halign(Align::Start);
        name.add_css_class("heading");
        info.append(&name);
        // Plain text: URLs and names may contain ampersands or markup characters.
        let detail = Label::new(Some(&format!(".{}  {}", bookmark.keyword, bookmark.url)));
        detail.set_halign(Align::Start);
        detail.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        detail.set_max_width_chars(45);
        detail.add_css_class("caption");
        info.append(&detail);
        content.append(&info);
        let edit = Button::from_icon_name("document-edit-symbolic");
        edit.set_tooltip_text(Some("Edit Bookmark"));
        edit.add_css_class("flat");
        let target = bookmark.clone();
        let weak_list = list.downgrade();
        let status_edit = status.clone();
        edit.connect_clicked(move |button| {
            if let (Some(parent), Some(list)) =
                (button.root().and_downcast::<Window>(), weak_list.upgrade())
            {
                show_editor(&parent, &list, &status_edit, Some(target.clone()));
            }
        });
        content.append(&edit);
        let delete = Button::from_icon_name("user-trash-symbolic");
        delete.set_tooltip_text(Some("Delete Bookmark"));
        delete.add_css_class("destructive-action");
        let weak_list = list.downgrade();
        let status_delete = status.clone();
        delete.connect_clicked(move |_| {
            let result = (|| -> anyhow::Result<()> {
                let store = Store::new()?;
                let mut config = LauncherSettings::load(&store)?;
                config.bookmarks.retain(|b| b.keyword != bookmark.keyword);
                config.save(&store)?;
                Ok(())
            })();
            match result {
                Ok(()) => {
                    status_delete.set_text("");
                    if let Some(list) = weak_list.upgrade() {
                        populate(&list, &status_delete);
                    }
                }
                Err(e) => status_delete.set_text(&e.to_string()),
            }
        });
        content.append(&delete);
        row.set_child(Some(&content));
        list.append(&row);
    }
}

fn show_editor(parent: &Window, list: &ListBox, status: &Label, target: Option<Bookmark>) {
    let dialog = Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(if target.is_some() {
            "Edit Bookmark"
        } else {
            "Add Bookmark"
        })
        .default_width(440)
        .build();
    let content = GtkBox::new(Orientation::Vertical, 10);
    content.set_margin_top(20);
    content.set_margin_bottom(20);
    content.set_margin_start(20);
    content.set_margin_end(20);
    let name = Entry::builder().placeholder_text("Dashboard").build();
    let keyword = Entry::builder().placeholder_text("dp").build();
    let url = Entry::builder()
        .placeholder_text("https://example.com/dashboard")
        .build();
    for (title, entry) in [
        ("Name", &name),
        ("Keyword (without the dot)", &keyword),
        ("URL", &url),
    ] {
        content.append(&Label::new(Some(title)));
        content.append(entry);
    }
    if let Some(ref bookmark) = target {
        name.set_text(&bookmark.name);
        keyword.set_text(&bookmark.keyword);
        url.set_text(&bookmark.url);
    }
    let error = Label::new(None);
    error.set_wrap(true);
    content.append(&error);
    let save = Button::with_label("Save Bookmark");
    save.add_css_class("suggested-action");
    content.append(&save);
    let weak_dialog = dialog.downgrade();
    let weak_list = list.downgrade();
    let status = status.clone();
    save.connect_clicked(move |_| {
        let result = (|| -> anyhow::Result<()> {
            let store = Store::new()?;
            let mut config = LauncherSettings::load(&store)?;
            let bookmark = Bookmark {
                name: name.text().into(),
                keyword: keyword.text().into(),
                url: url.text().into(),
            };
            if let Some(ref target) = target {
                let index = config
                    .bookmarks
                    .iter()
                    .position(|b| b.keyword == target.keyword)
                    .ok_or_else(|| {
                        anyhow::anyhow!("This bookmark was removed. Close and reopen the editor.")
                    })?;
                config.bookmarks[index] = bookmark;
            } else {
                config.bookmarks.push(bookmark);
            }
            config.save(&store)?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                status.set_text("");
                if let Some(list) = weak_list.upgrade() {
                    populate(&list, &status);
                }
                if let Some(dialog) = weak_dialog.upgrade() {
                    dialog.close();
                }
            }
            Err(e) => error.set_text(&e.to_string()),
        }
    });
    let weak_dialog = dialog.downgrade();
    let keys = gtk4::EventControllerKey::new();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Escape {
            if let Some(dialog) = weak_dialog.upgrade() {
                dialog.close();
            }
            return gtk4::glib::Propagation::Stop;
        }
        gtk4::glib::Propagation::Proceed
    });
    dialog.add_controller(keys);
    dialog.set_child(Some(&content));
    dialog.present();
}
