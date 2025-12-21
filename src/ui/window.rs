use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, ListView, SignalListItemFactory, SingleSelection, StringList, Label, Box as GtkBox, Orientation, Align, ScrolledWindow, FilterListModel, StringFilter};
use crate::data::browser_repository;
use gtk4::gdk;

use gtk4::glib::WeakRef; 
use crate::data::store::Store; 

// Helper to update label markup
// Helper to update label markup
fn update_label_markup(label: &Label, text: &str, query: &str, _is_pinned: bool) {
    // Prefix removed, handled by icon now
    if query.is_empty() {
        label.set_markup(&gtk4::glib::markup_escape_text(text));
    } else {
        let query_lower = query.to_lowercase();
        let text_lower = text.to_lowercase();
        
        if let Some(idx) = text_lower.find(&query_lower) {
            let start = idx;
            let end = start + query_lower.len();
            
            let before = gtk4::glib::markup_escape_text(&text[0..start]);
            let matched = gtk4::glib::markup_escape_text(&text[start..end]);
            let after = gtk4::glib::markup_escape_text(&text[end..]);
            
            let markup = format!("{}<span foreground='#f0e68c' underline='single'>{}</span>{}", before, matched, after);
            label.set_markup(&markup);
        } else {
             label.set_markup(&gtk4::glib::markup_escape_text(text));
        }
    }
}

fn refresh_rows(
    rows: &std::rc::Rc<std::cell::RefCell<Vec<WeakRef<GtkBox>>>>, 
    query: &str,
    pinned_map: &std::collections::HashMap<String, bool>
) {
    let mut live = Vec::new();
    let borrowed = rows.borrow();
    for weak in borrowed.iter() {
        if let Some(hbox) = weak.upgrade() {
            // Structure: Icon (0), Label (1), Pin (2)
            if let Some(child1) = hbox.first_child().and_then(|w| w.next_sibling()) {
                if let Some(label) = child1.downcast_ref::<Label>() {
                    let current_text = label.text();
                    // No prefix removal needed anymore
                    let is_pinned = *pinned_map.get(current_text.as_str()).unwrap_or(&false);
                    update_label_markup(label, current_text.as_str(), query, false); // false = no prefix
                    
                    // Pin Icon
                    if let Some(child2) = label.next_sibling() {
                         child2.set_visible(is_pinned);
                    }
                }
            }
            live.push(weak.clone());
        }
    }
    drop(borrowed);
    *rows.borrow_mut() = live;
}


pub fn build_ui(app: &Application, url_to_open: Option<&str>) {
    // Load CSS
    let provider = gtk4::CssProvider::new();
    provider.load_from_path("resources/style.css");
    
    if let Some(display) = gdk::Display::default() {
         gtk4::style_context_add_provider_for_display(
             &display,
             &provider,
             gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
         );
         
         // Add resources to icon theme search path
         let icon_theme = gtk4::IconTheme::for_display(&display);
         icon_theme.add_search_path("resources");
    }
    
    // Set default icon for the process (fallback for some WMs)
    // gtk4::Window::set_default_icon_name("opennav"); // This is a static method in older gtk? No, doesn't exist in gtk4::Window.
    // We rely on window instance icon name.

    let window = ApplicationWindow::builder()
        .application(app)
        .title("OpenNav")
        .default_width(500)
        .default_height(500)
        .modal(true)
        .modal(true)
        .decorated(false)
        .build();
        
    // Attempt to set window icon
    window.set_icon_name(Some("opennav"));
    
    // Shared state for search query

    // Shared state for search query
    // Shared state for search query and active labels
    // Shared state
    // Shared state
    let search_query = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    let active_rows = std::rc::Rc::new(std::cell::RefCell::new(Vec::<WeakRef<GtkBox>>::new()));
    let pinned_map = std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashMap::<String, bool>::new()));
    let icon_map = std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashMap::<String, String>::new()));

    // Main layout container (Vertical Box)
    let vbox = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .build();

    // URL Entry Area (was Search Bar)
    let url_entry = gtk4::Entry::builder()
        .placeholder_text("URL to open...")
        .margin_top(15)
        .margin_bottom(15)
        .margin_start(15)
        .margin_end(15)
        .build();
    
    if let Some(u) = url_to_open {
        url_entry.set_text(u);
        url_entry.set_position(-1);
    }
    
    vbox.append(&url_entry);

    // Browser List Logic
    let mut browsers = browser_repository::get_installed_browsers();
    
    // Sort by usage and pin status
    let store = Store::new().ok();
    if let Some(ref s) = store {
        if let Ok(stats) = s.get_stats() {
             use std::collections::HashMap;
             let usage_map: HashMap<String, (i64, bool)> = stats.into_iter().map(|(id, count, pinned, _)| (id, (count, pinned))).collect();
             
             // First pass: update is_pinned in struct
             for browser in &mut browsers {
                 if let Some((_, pinned)) = usage_map.get(&browser.id) {
                     browser.is_pinned = *pinned;
                 }
             }
             
             browsers.sort_by(|a, b| {
                 // Pin status first (true > false)
                 b.is_pinned.cmp(&a.is_pinned)
                     .then_with(|| {
                         // Then usage
                         let count_a = usage_map.get(&a.id).map(|x| x.0).unwrap_or(0);
                         let count_b = usage_map.get(&b.id).map(|x| x.0).unwrap_or(0);
                         count_b.cmp(&count_a)
                     })
                     .then_with(|| a.name.cmp(&b.name))
             });
        }
    }
    
    // Populate maps
    {
        let mut p_map = pinned_map.borrow_mut();
        let mut i_map = icon_map.borrow_mut();
        for b in &browsers {
            p_map.insert(b.name.clone(), b.is_pinned);
            i_map.insert(b.name.clone(), b.icon.clone());
        }
    }

    let browsers_rc = std::rc::Rc::new(browsers);
    let string_list = StringList::new(&browsers_rc.iter().map(|b| b.name.as_str()).collect::<Vec<&str>>());
    
    // Search Filter
    let filter = StringFilter::builder()
        .match_mode(gtk4::StringFilterMatchMode::Substring)
        .ignore_case(true)
        .build();
    
    // Bind search entry to filter (Implicitly via variable now)
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

    let selection_model = SingleSelection::new(Some(filter_model));
    selection_model.set_autoselect(true); 

    let factory = SignalListItemFactory::new();

    let search_query_for_bind = search_query.clone();
    let active_rows_for_bind = active_rows.clone();
    let pinned_map_for_bind = pinned_map.clone();
    let icon_map_for_bind = icon_map.clone();

    // Context for GestureClick
    let browsers_for_click = browsers_rc.clone();
    let url_entry_weak_click = url_entry.downgrade();
    let window_weak_click = window.downgrade();
    
    // factory.connect_setup
    factory.connect_setup(move |_, list_item| {
        let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
        let hbox = GtkBox::new(Orientation::Horizontal, 12);
        hbox.set_css_classes(&["browser-row"]);
        
        // Browser Icon
        let icon = gtk4::Image::builder()
            .pixel_size(32)
            .build();
            
        let label = Label::new(None);
        label.set_halign(Align::Start);
        label.set_hexpand(true);
        label.set_use_markup(true);
        
        // Pin Icon
        let pin_icon = gtk4::Image::builder()
            .icon_name("view-pin-symbolic") // Should adapt to theme (white in dark)
            .pixel_size(16)
            .visible(false)
            .build();
        
        hbox.append(&icon);
        hbox.append(&label);
        hbox.append(&pin_icon);
        
        // Hidden Label for Data Transfer
        let hidden_label = Label::new(None);
        hidden_label.set_visible(false);
        hbox.append(&hidden_label);

        // Click Handling
        let gesture = gtk4::GestureClick::new();
        let browsers_inner = browsers_for_click.clone();
        let url_inner = url_entry_weak_click.clone();
        let win_inner = window_weak_click.clone();
        
        gesture.connect_released(move |gesture, _, _, _| {
             let modifiers = gesture.current_event().map(|e| e.modifier_state()).unwrap_or_else(gtk4::gdk::ModifierType::empty);
             let keep_open = modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK);
             
             let widget = gesture.widget().expect("Widget attached");
             if let Some(hbox) = widget.downcast_ref::<GtkBox>() {
                 if let Some(last_child) = hbox.last_child() {
                     if let Some(lbl) = last_child.downcast_ref::<Label>() {
                         let name = lbl.text();
                         if !name.is_empty() {
                             if let Some(browser) = browsers_inner.iter().find(|b| b.name == name.as_str()) {
                                 let target_url = if let Some(entry) = url_inner.upgrade() {
                                     entry.text().to_string()
                                 } else {
                                     String::new()
                                 };

                                 // Increment usage
                                 if let Ok(store) = crate::data::store::Store::new() {
                                     let _ = store.increment_usage(&browser.id);
                                 }
                                 // Launch
                                 let _ = browser_repository::launch_browser(&browser.id, &target_url);

                                 if let Some(win) = win_inner.upgrade() {
                                     if !keep_open {
                                         win.close();
                                     } else {
                                         // Re-present to ensure focus stays if needed
                                         win.present();
                                     }
                                 }
                             }
                         }
                     }
                 }
             }
        });
        hbox.add_controller(gesture);
        
        list_item.set_child(Some(&hbox));
    });

    factory.connect_bind(move |_, list_item| {
        let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
        let string_object = list_item
            .item()
            .and_downcast::<gtk4::StringObject>()
            .unwrap();
            
        let hbox = list_item
            .child()
            .and_downcast::<GtkBox>()
            .unwrap();
            
        // Track the row
        active_rows_for_bind.borrow_mut().push(hbox.downgrade());
        
        let name = string_object.string();
        let name_str = name.as_str();

        // Update Hidden Label for Gesture
        if let Some(last_child) = hbox.last_child() {
            if let Some(lbl) = last_child.downcast_ref::<Label>() {
                lbl.set_text(name_str);
            }
        }
        
        // Update Label
        let query = search_query_for_bind.borrow();
        let p_map = pinned_map_for_bind.borrow();
        let i_map = icon_map_for_bind.borrow();
        
        let is_pinned = *p_map.get(name_str).unwrap_or(&false);
        let icon_name = i_map.get(name_str).cloned().unwrap_or_else(|| "web-browser".to_string());
        
        // 0: Icon, 1: Label, 2: Pin
        if let Some(child0) = hbox.first_child() {
             if let Some(icon) = child0.downcast_ref::<gtk4::Image>() {
                 let icon_path_str = if icon_name.starts_with("file://") {
                     &icon_name[7..]
                 } else {
                     &icon_name
                 };
                 
                 let path = std::path::Path::new(icon_path_str);
                 if path.is_absolute() && path.exists() {
                     // Try loading as Texture for better format support (e.g. WebP)
                     let file = gtk4::gio::File::for_path(path);
                     match gtk4::gdk::Texture::from_file(&file) {
                         Ok(texture) => {
                             icon.set_paintable(Some(&texture));
                         }
                         Err(_e) => {
                             icon.set_from_file(Some(path));
                         }
                     }
                 } else {
                     icon.set_icon_name(Some(&icon_name));
                 }
             }
             
             if let Some(child1) = child0.next_sibling() {
                 if let Some(label) = child1.downcast_ref::<Label>() {
                     update_label_markup(&label, name_str, &query, is_pinned); 
                     
                     if let Some(child2) = child1.next_sibling() {
                         child2.set_visible(is_pinned);
                     }
                 }
             }
        }
    });

    let list_view = ListView::new(Some(selection_model.clone()), Some(factory));
    list_view.set_single_click_activate(true);
    
    // Scrolled Window for the list
    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .min_content_height(300)
        .vexpand(true)
        .child(&list_view)
        .build();

    vbox.append(&scrolled_window);


    
    // Status Bar
    let status_box = GtkBox::new(Orientation::Horizontal, 10);
    status_box.set_margin_bottom(10);
    status_box.set_halign(Align::Center);
    status_box.set_opacity(0.7);
    
    // Help Button
    let help_btn = gtk4::Button::builder()
        .icon_name("help-about-symbolic")
        .has_frame(false)
        .tooltip_text("Shortcuts")
        .build();
    
    let window_weak_for_help = window.downgrade();
    help_btn.connect_clicked(move |_| {
        if let Some(parent) = window_weak_for_help.upgrade() {
             let dialog = gtk4::Window::builder()
                .transient_for(&parent)
                .modal(true)
                .title("Shortcuts")
                .default_width(300)
                .default_height(350)
                .build();
    
    // Add Esc handler for dialog
    let d_weak_help = dialog.downgrade();
    let key_controller_help = gtk4::EventControllerKey::new();
    key_controller_help.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Escape {
             if let Some(d) = d_weak_help.upgrade() { d.close(); }
             return gtk4::glib::Propagation::Stop;
        }
        gtk4::glib::Propagation::Proceed
    });
    dialog.add_controller(key_controller_help);
            
    let vbox = GtkBox::new(Orientation::Vertical, 10);
            vbox.set_margin_top(20);
            vbox.set_margin_bottom(20);
            vbox.set_margin_start(20);
            vbox.set_margin_end(20);
            
            let shortcuts = [
                ("Type", "Search Browsers"),
                ("Ctrl + L", "Focus URL Bar"),
                ("Up/Down Arrows", "Navigation"),
                ("Enter / Click", "Launch Selected"),
                ("Ctrl + Enter", "Launch & Keep Open"),
                ("Ctrl + Click", "Launch & Keep Open"),
                ("Ctrl + P", "Toggle Pin"),
                ("Ctrl + S", "Settings"),
                ("Ctrl + ?", "Shortcuts (Help)"),
                ("Esc", "Close / Clear Search"),
            ];
            
            let grid = gtk4::Grid::builder()
                .column_spacing(20)
                .row_spacing(10)
                .margin_start(10)
                .margin_end(10)
                .build();

            for (i, (key, desc)) in shortcuts.iter().enumerate() {
                let key_label = Label::new(None);
                key_label.set_markup(&format!("<b>{}</b>", key));
                key_label.set_halign(Align::Start);
                
                let desc_label = Label::new(Some(desc));
                desc_label.set_halign(Align::Start);
                
                grid.attach(&key_label, 0, i as i32, 1, 1);
                grid.attach(&desc_label, 1, i as i32, 1, 1);
            }
            vbox.append(&grid);
            
            // Close btn
            let close_btn = gtk4::Button::with_label("Close");
            let d_weak = dialog.downgrade();
            close_btn.connect_clicked(move |_| {
                if let Some(d) = d_weak.upgrade() { d.close(); }
            });
            vbox.append(&close_btn);
            
            dialog.set_child(Some(&vbox));
            dialog.present();
        }
    });
    
    status_box.append(&help_btn);
    
    // Settings Button
    let settings_btn = gtk4::Button::builder()
        .icon_name("emblem-system-symbolic")
        .has_frame(false)
        .tooltip_text("Settings")
        .build();
    
    let window_weak_for_settings = window.downgrade();
    settings_btn.connect_clicked(move |_| {
        if let Some(parent) = window_weak_for_settings.upgrade() {
            let dialog = gtk4::Window::builder()
                .transient_for(&parent)
                .modal(true)
                .title("Settings")
                .default_width(320)
                .default_height(240)
                .build();

            // Add Esc handler for dialog
            let d_weak_settings = dialog.downgrade();
            let key_controller_settings = gtk4::EventControllerKey::new();
            key_controller_settings.connect_key_pressed(move |_, key, _, _| {
                if key == gtk4::gdk::Key::Escape {
                     if let Some(d) = d_weak_settings.upgrade() { d.close(); }
                     return gtk4::glib::Propagation::Stop;
                }
                gtk4::glib::Propagation::Proceed
            });
            dialog.add_controller(key_controller_settings);
                
            let vbox = GtkBox::new(Orientation::Vertical, 20);
            vbox.set_margin_top(20);
            vbox.set_margin_bottom(20);
            vbox.set_margin_start(20);
            vbox.set_margin_end(20);
            
            // App Logo
            let logo = gtk4::Image::from_file("resources/app-icon.png");
            logo.set_pixel_size(64);
            vbox.append(&logo);
            
            // About Section
            let about_label = Label::new(Some("OpenNav v0.1.0\n<span size='small'>A fast browser picker and launcher.</span>"));
            about_label.set_use_markup(true);
            about_label.set_justify(gtk4::Justification::Center);
            vbox.append(&about_label);
            
            // Actions
            let clear_stats_btn = gtk4::Button::with_label("Reset Usage Stats");
            let parent_weak = parent.downgrade();
            let dialog_weak = dialog.downgrade();
            
            clear_stats_btn.connect_clicked(move |_| {
                 if let Ok(store) = Store::new() {
                     if let Ok(_) = store.clear_stats() {
                         if let Some(_p) = parent_weak.upgrade() {
                             // Use a proper dialog or just print/label?
                             // Let's change label text temporarily or close
                         }
                         println!("Stats cleared.");
                     }
                 }
                 if let Some(d) = dialog_weak.upgrade() {
                     d.close();
                 }
            });
            vbox.append(&clear_stats_btn);
            
            let close_btn = gtk4::Button::with_label("Close");
            let dialog_weak_2 = dialog.downgrade();
            close_btn.connect_clicked(move |_| {
                if let Some(d) = dialog_weak_2.upgrade() {
                    d.close();
                }
            });
            vbox.append(&close_btn);
            
            dialog.set_child(Some(&vbox));
            dialog.present();
        }
    });
    
    status_box.append(&settings_btn);
    // Adjust logic to keep help centered? 
    // We expanded help_label, so it takes space. Settings btn is at end.
    // To keep help centered, we might need a dummy spacer at start or use CenterBox.
    // For now, let's just append settings at the end, help might shift left slightly.
    
    vbox.append(&status_box);
    


    window.set_child(Some(&vbox));

    // List View Key Controller (Left/Up Arrow to URL Entry)
    let url_entry_weak = url_entry.downgrade();
    let selection_model_weak_for_list = selection_model.downgrade();
    let list_key_controller = gtk4::EventControllerKey::new();
    list_key_controller.connect_key_pressed(move |_, key, _, modifiers| {

        if key == gtk4::gdk::Key::Up {
             if let Some(sel) = selection_model_weak_for_list.upgrade() {
                 if sel.selected() == 0 {
                     if let Some(entry) = url_entry_weak.upgrade() {
                        entry.grab_focus();
                        return gtk4::glib::Propagation::Stop;
                    }
                 }
            }
        }
        gtk4::glib::Propagation::Proceed
    });
    list_view.add_controller(list_key_controller);

    // Global Key Controller for Shortcuts (Enter, Esc, Typing)
    let key_controller = gtk4::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    
    let browsers_for_key = browsers_rc.clone();
    let selection_model_weak = selection_model.downgrade();
    let window_weak = window.downgrade();
    let url_entry_weak_2 = url_entry.downgrade();
    let search_query_clone = search_query.clone();
    let active_rows_clone = active_rows.clone();
    let pinned_map_clone = pinned_map.clone();
    let filter_weak = filter.downgrade();
    
    // Buttons for shortcuts
    let help_btn_weak = help_btn.downgrade();
    let settings_btn_weak = settings_btn.downgrade();

    key_controller.connect_key_pressed(move |_controller, key, _keycode, modifiers| {
        // Check focus to avoid eating URL entry inputs
        // "has_focus()" on Entry might return false if internal Text widget has focus
        // We must check if the focused widget is the entry or a child of it.
        if let Some(window) = window_weak.upgrade() {
            if let Some(focus_widget) = gtk4::prelude::GtkWindowExt::focus(&window) {
                if let Some(entry) = url_entry_weak_2.upgrade() {
                    let entry_widget = entry.upcast_ref::<gtk4::Widget>();
                    if &focus_widget == entry_widget || focus_widget.is_ancestor(entry_widget) {
                         // Focus is in URL Entry (or its internal text widget).
                        if key == gtk4::gdk::Key::Down {
                             return gtk4::glib::Propagation::Proceed; 
                        }
                        
                        if key == gtk4::gdk::Key::Return || key == gtk4::gdk::Key::KP_Enter {
                            // Fallthrough to launch logic
                        } else {
                             return gtk4::glib::Propagation::Proceed;
                        }
                    }
                }
                
                // If focus is on a Button, let it handle Enter/Space
                if focus_widget.downcast_ref::<gtk4::Button>().is_some() {
                     if key == gtk4::gdk::Key::Return || key == gtk4::gdk::Key::KP_Enter || key == gtk4::gdk::Key::space {
                         return gtk4::glib::Propagation::Proceed;
                     }
                }
            }
        }
        
        // Handle Shortcuts
        if let Some(window) = window_weak.upgrade() {
            if key == gtk4::gdk::Key::Escape {
                let should_stop = {
                    let mut query = search_query_clone.borrow_mut();
                    if !query.is_empty() {
                        query.clear();
                        true
                    } else {
                        false
                    }
                };

                if should_stop {
                    if let Some(f) = filter_weak.upgrade() {
                        f.set_search(None::<&str>);
                        // Refresh labels to clear markup
                        refresh_rows(&active_rows_clone, "", &pinned_map_clone.borrow());
                    }
                    return gtk4::glib::Propagation::Stop;
                }
                
                window.close();
                window.close();
                return gtk4::glib::Propagation::Stop;
            }

            // Ctrl + L (Focus URL Bar)
            if key == gtk4::gdk::Key::l && modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                if let Some(entry) = url_entry_weak_2.upgrade() {
                    entry.grab_focus();
                    entry.select_region(0, -1);
                }
                return gtk4::glib::Propagation::Stop;
            }
            
            // Ctrl+P for Pinning
            if key == gtk4::gdk::Key::p && modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                 if let Some(selection_model) = selection_model_weak.upgrade() {
                    if let Some(item) = selection_model.selected_item() {
                        let string_object = item.downcast::<gtk4::StringObject>().unwrap();
                        let name = string_object.string();
                        
                        // Find browser ID
                        if let Some(browser) = browsers_for_key.iter().find(|b| b.name == name) {
                             if let Ok(store) = Store::new() {
                                 if let Ok(new_state) = store.toggle_pin(&browser.id) {
                                     // Update map
                                     pinned_map_clone.borrow_mut().insert(browser.name.clone(), new_state);
                                     
                                     // Refresh labels immediately to show/hide pin
                                     refresh_rows(&active_rows_clone, &search_query_clone.borrow(), &pinned_map_clone.borrow());
                                 }
                             }
                        }
                    }
                 }
                 return gtk4::glib::Propagation::Stop;
            }
            
            // Ctrl + S (Settings)
            if key == gtk4::gdk::Key::s && modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                if let Some(btn) = settings_btn_weak.upgrade() {
                    btn.emit_clicked();
                }
                return gtk4::glib::Propagation::Stop;
            }

            // Ctrl + ? (Help)
            // Note: '?' is usually Shift+/, so modifiers might contain Shift. 
            // We check for Key::question OR (Key::slash + Ctrl).
            // Actually, simplified check:
            if (key == gtk4::gdk::Key::question || key == gtk4::gdk::Key::slash) 
                && modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                if let Some(btn) = help_btn_weak.upgrade() {
                    btn.emit_clicked();
                }
                return gtk4::glib::Propagation::Stop;
            }
            

            if key == gtk4::gdk::Key::BackSpace {
                let new_state = {
                    let mut query = search_query_clone.borrow_mut();
                    if !query.is_empty() {
                        query.pop();
                        Some(query.clone())
                    } else {
                        None
                    }
                };
                
                if let Some(q) = new_state {
                    if let Some(f) = filter_weak.upgrade() {
                        let s = if q.is_empty() { None } else { Some(q.as_str()) };
                        f.set_search(s);
                        refresh_rows(&active_rows_clone, &q, &pinned_map_clone.borrow());
                    }
                }
                // Always consume Backspace to prevent default behavior (which might be closing window or navigating back)
                return gtk4::glib::Propagation::Stop;
            }
            
            if key == gtk4::gdk::Key::Return || key == gtk4::gdk::Key::KP_Enter {
                // Launch logic
                if let Some(selection_model) = selection_model_weak.upgrade() {
                    if let Some(item) = selection_model.selected_item() {
                        let string_object = item.downcast::<gtk4::StringObject>().unwrap();
                        let name = string_object.string();
                        
                        if let Some(browser) = browsers_for_key.iter().find(|b| b.name == name) {
                             let target_url = if let Some(entry) = url_entry_weak_2.upgrade() {
                                 entry.text().to_string()
                             } else {
                                 String::new()
                             };

                             if let Ok(store) = crate::data::store::Store::new() {
                                 let _ = store.increment_usage(&browser.id);
                             }

                             let _ = browser_repository::launch_browser(&browser.id, &target_url);

                             if !modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                                 window.close();
                             } else {
                                 let window_weak_for_timeout = window.downgrade();
                                 gtk4::glib::timeout_add_local(std::time::Duration::from_millis(300), move || {
                                     if let Some(win) = window_weak_for_timeout.upgrade() {
                                         win.present();
                                     }
                                     gtk4::glib::ControlFlow::Break
                                 });
                             }
                        }
                    }
                }
                return gtk4::glib::Propagation::Stop;
            }
        }
        
        // Handle Typing for Filter
        if let Some(ch) = key.to_unicode() {
            if ch.is_control() {
                 return gtk4::glib::Propagation::Proceed;
            }
            
            let query_str = {
                let mut query = search_query_clone.borrow_mut();
                query.push(ch);
                query.clone()
            };
            
            if let Some(f) = filter_weak.upgrade() {
                 f.set_search(Some(query_str.as_str()));
                 refresh_rows(&active_rows_clone, &query_str, &pinned_map_clone.borrow());
            }
            return gtk4::glib::Propagation::Stop;
        }

        gtk4::glib::Propagation::Proceed
    });
    window.add_controller(key_controller);

    // URL Entry Key Controller (Down Arrow to List)
    let list_view_weak = list_view.downgrade();
    let entry_controller = gtk4::EventControllerKey::new();
    entry_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Down {
            if let Some(list_view) = list_view_weak.upgrade() {
                list_view.grab_focus();
                return gtk4::glib::Propagation::Stop;
            }
        }
        gtk4::glib::Propagation::Proceed
    });
    url_entry.add_controller(entry_controller);

    window.present();
    
    // Focus list by default so typing searches
    list_view.grab_focus();
}
