use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Entry, Image, Label, ListBox, ListBoxRow, Orientation, Window, ScrolledWindow};
use crate::data::store::{Store, SearchEngine};
use crate::data::icons;
// use std::rc::Rc; // Unused

pub fn build_engine_management_ui() -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 10);
    // container.set_margin_all(0); // Embedded, let parent handle outer margins

    // Header / Toolbar
    let toolbar = GtkBox::new(Orientation::Horizontal, 10);
    let label = Label::new(Some("<b>Search Engines</b>"));
    label.set_use_markup(true);
    label.set_hexpand(true);
    label.set_halign(Align::Start);
    toolbar.append(&label);
    
    // Add Button (Header style)
    let add_btn = Button::with_label("Add"); // Minimal text, or icon "list-add-symbolic"
    add_btn.add_css_class("suggested-action");
    add_btn.set_width_request(100);
    toolbar.append(&add_btn);
    
    container.append(&toolbar);
    
    // List
    let scrolled = ScrolledWindow::new();
    scrolled.set_vexpand(true);
    scrolled.set_min_content_height(300); // Ensure visible height in embedded mode
    scrolled.add_css_class("frame");
    
    let list_box = ListBox::new();
    list_box.set_selection_mode(gtk4::SelectionMode::None);
    list_box.add_css_class("content"); // Clean look
    scrolled.set_child(Some(&list_box));
    container.append(&scrolled);
    
    // Logic to populate list
    let populate_list = {
        let list_box = list_box.clone();
        move || {
            // Clear
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }
            if let Ok(store) = Store::new() {
                if let Ok(engines) = store.list_engines() {
                    for engine in engines {
                        add_row(&list_box, engine);
                    }
                }
            }
        }
    };
    
    populate_list();
    
    // Add Handler
    let list_box_clone = list_box.clone();
    // We need parent for dialog? We can get root from widget
    
    add_btn.connect_clicked(move |btn| {
        // Find Toplevel for dialog parent
        let root = btn.root().and_then(|r| r.downcast::<Window>().ok());
        if let Some(parent) = root {
            show_add_edit_dialog(&parent, list_box_clone.clone(), None);
        }
    });

    container
}

fn add_row(list_box: &ListBox, engine: SearchEngine) {
    let row = ListBoxRow::new();
    let hbox = GtkBox::new(Orientation::Horizontal, 12);
    hbox.set_margin_top(12);
    hbox.set_margin_bottom(12);
    hbox.set_margin_start(12);
    hbox.set_margin_end(12);
    
    // Icon
    let icon_path = engine.icon_path.clone().unwrap_or_default();
    let icon = if !icon_path.is_empty() && std::path::Path::new(&icon_path).exists() {
         Image::from_file(&icon_path)
    } else {
         Image::from_icon_name("system-search-symbolic")
    };
    icon.set_pixel_size(32);
    hbox.append(&icon);
    
    // Info
    let vbox_info = GtkBox::new(Orientation::Vertical, 2);
    vbox_info.set_hexpand(true);
    let name_label = Label::builder().label(&engine.name).halign(Align::Start).build();
    name_label.add_css_class("heading");
    
    let kw_label = Label::builder()
         .label(&format!("<tt>{}</tt>  <span color='gray'>{}</span>", engine.keyword, engine.url))
         .halign(Align::Start)
         .use_markup(true)
         .ellipsize(gtk4::pango::EllipsizeMode::End)
         .build();
    kw_label.add_css_class("caption");
    
    vbox_info.append(&name_label);
    vbox_info.append(&kw_label);
    hbox.append(&vbox_info);
    
    // Set Default Logic
    let is_default_check = {
        let keyword = engine.keyword.clone();
        if let Ok(store) = Store::new() {
             let current = store.get_setting("search_engine").ok().flatten().unwrap_or("g".to_string());
             current == keyword
        } else {
             false
        }
    };
    
    if is_default_check {
        let def_lbl = Label::new(Some("<i>(Default)</i>"));
        def_lbl.set_use_markup(true);
        def_lbl.add_css_class("dim-label");
        hbox.append(&def_lbl);
    } else {
        // Tick button to make default
        let make_def_btn = Button::from_icon_name("emblem-ok-symbolic"); // Tick
        make_def_btn.set_tooltip_text(Some("Set as Default"));
        make_def_btn.add_css_class("flat");
        let keyword = engine.keyword.clone();
        
        let lb_weak = list_box.downgrade();
        
        make_def_btn.connect_clicked(move |btn| {
             if let Ok(store) = Store::new() {
                 let _ = store.set_setting("search_engine", &keyword);
                 // We need to refresh the list to update UI state for ALL rows (remove default from old, add to new)
                 // Or we find sibling rows. 
                 // Simpler: Just trigger full reload of the listbox content.
                 // How? We don't have reference to populate_list closure easily.
                 // Hack: Trigger a "refresh" by emitting a signal? 
                 // Or just modify THIS row visual state? 
                 // But we need to remove "(Default)" from the previous default row.
                 
                 // Best approach for embedded UI:
                 // Find usage of listbox and clear/repopulate.
                 // But we are inside the closure. 
                 
                 // For now, let's try to notify user or reload?
                 // Actually, since we are inside `window.rs` logic now (conceptually), maybe we can just traverse.
                 // Let's iterate all rows and update them? Too complex.
                 
                 // Let's just assume the user sees the button clicked.
                 // Wait, "Show (default) against...".
                 // Use `btn.set_visible(false)` and `hbox.insert_child_after(label)`.
                 // But the *other* row (previous default) remains marked as default visually? That's incorrect.
                 
                 // Solution: We need a full refresh.
                 // Given structure, rebuilding the list is best.
                 // Let's recurse? `build_engine_management_ui` has `populate_list`.
                 // Pass a weak ref to `populate_list`? No.
                 
                 // Alternative: Let the parent (Settings Window) handle refresh?
                 // No, too coupled.
                 
                 // Okay, quick fix: iterate and toggle classes?
                 // Let's implement full refresh by clearing listbox and re-querying inside the click handler?
                 if let Some(lb) = lb_weak.upgrade() {
                     // Clear
                     while let Some(child) = lb.first_child() {
                         lb.remove(&child);
                     }
                     // Repopulate
                     if let Ok(engines) = store.list_engines() {
                         for engine in engines {
                             add_row(&lb, engine);
                         }
                     }
                 }
             }
        });
        hbox.append(&make_def_btn);
    }
    
    // Edit Button
    let edit_btn = Button::from_icon_name("document-edit-symbolic");
    edit_btn.set_tooltip_text(Some("Edit Engine"));
    edit_btn.add_css_class("flat");
    
    let engine_clone = engine.clone();
    let lb_weak_edit = list_box.downgrade();
    
    edit_btn.connect_clicked(move |btn| {
         // Find root
         let root = btn.root().and_then(|r| r.downcast::<Window>().ok());
         if let Some(parent) = root {
              if let Some(lb) = lb_weak_edit.upgrade() {
                  show_add_edit_dialog(&parent, lb, Some(engine_clone.clone())); // Pass clone for editing
              }
         }
    });
    hbox.append(&edit_btn);

    // Delete Button
    let del_btn = Button::from_icon_name("user-trash-symbolic");
    del_btn.add_css_class("destructive-action");
    del_btn.set_tooltip_text(Some("Delete Engine"));
    
    let keyword_del = engine.keyword.clone();
    let lb_weak_del = list_box.downgrade(); // duplicate weak ref
    
    del_btn.connect_clicked(move |btn| {
          if let Ok(store) = Store::new() {
              if store.delete_engine(&keyword_del).is_ok() {
                  if let Some(row_widget) = btn.ancestor(ListBoxRow::static_type()) {
                      if let Some(lb) = lb_weak_del.upgrade() {
                          lb.remove(&row_widget);
                      }
                  }
              }
          }
    });
    
    hbox.append(&del_btn);
    row.set_child(Some(&hbox));
    list_box.append(&row);
}

fn show_add_edit_dialog(parent: &Window, list_box: ListBox, edit_target: Option<SearchEngine>) {
    let is_edit = edit_target.is_some();
    let title = if is_edit { "Edit Search Engine" } else { "Add Search Engine" };
    
    let dialog = Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(title)
        .default_width(400)
        .default_height(350)
        .build();

    // Add Esc handler for dialog
    let d_weak = dialog.downgrade();
    let key_controller = gtk4::EventControllerKey::new();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Escape {
             if let Some(d) = d_weak.upgrade() { d.close(); }
             return gtk4::glib::Propagation::Stop;
        }
        gtk4::glib::Propagation::Proceed
    });
    dialog.add_controller(key_controller);
        
    let vbox = GtkBox::new(Orientation::Vertical, 15);
    vbox.set_margin_top(20);
    vbox.set_margin_bottom(20);
    vbox.set_margin_start(20);
    vbox.set_margin_end(20);
    dialog.set_child(Some(&vbox));
    
    // Fields
    let name_entry = Entry::builder().placeholder_text("Name (e.g. GitHub)").build();
    let kw_entry = Entry::builder().placeholder_text("Keyword (e.g. gh)").build();
    let url_entry = Entry::builder().placeholder_text("URL (e.g. https://github.com?q={})").build();
    
    // Pre-fill if editing
    if let Some(ref e) = edit_target {
        name_entry.set_text(&e.name);
        kw_entry.set_text(&e.keyword);
        url_entry.set_text(&e.url);
        // Disable keyword editing effectively? Or allow it? 
        // If we allow, we need to handle PK change. update_engine handles it if logic supports it.
        // For simplicity, let's allow it.
    }
    
    vbox.append(&Label::new(Some("Name")));
    vbox.append(&name_entry);
    
    vbox.append(&Label::new(Some("Keyword (Alias)")));
    vbox.append(&kw_entry);
    
    vbox.append(&Label::new(Some("Search URL (use {} for query)")));
    vbox.append(&url_entry);
    
    // Save
    let btn_label = if is_edit { "Update Engine" } else { "Save Engine" };
    let save_btn = Button::with_label(btn_label);
    save_btn.add_css_class("suggested-action");
    
    let dialog_weak = dialog.downgrade();
    let original_keyword = edit_target.map(|e| e.keyword);
    
    save_btn.connect_clicked(move |_| {
        let name = name_entry.text().to_string();
        let keyword = kw_entry.text().to_string();
        let url = url_entry.text().to_string();
        
        if name.is_empty() || keyword.is_empty() || url.is_empty() {
            return; // TODO: Show error
        }
        
        // Try fetch icon (only if new or changed? for now always fetch if logical)
        // If editing, maybe we preserve existing icon if URL didn't change?
        // Let's just fetch, it caches anyway in `icons.rs`.
        let icon_path = icons::fetch_favicon(&url).ok();
        
        let engine = SearchEngine {
            name,
            keyword: keyword.clone(),
            url,
            icon_path
        };
        
        if let Ok(store) = Store::new() {
            let res = if let Some(ref orig_kw) = original_keyword {
                 store.update_engine(orig_kw, &engine)
            } else {
                 store.add_engine(&engine)
            };
            
            if res.is_ok() {
                // Refresh List by clearing and re-adding?
                // Actually, `add_row` appends. We need to replace or refresh fully.
                // Since this dialog doesn't know about loop, we should ideally trigger full refresh on listbox again.
                // Re-using the logic from Set Default:
                while let Some(child) = list_box.first_child() {
                     list_box.remove(&child);
                }
                if let Ok(engines) = store.list_engines() {
                     for engine in engines {
                         add_row(&list_box, engine);
                     }
                }
                
                if let Some(d) = dialog_weak.upgrade() {
                    d.close();
                }
            }
        }
    });
    
    vbox.append(&save_btn);
    dialog.present();
}
