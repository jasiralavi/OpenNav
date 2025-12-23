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
                    
                    // Pin Button
                    if let Some(pin_btn) = label.next_sibling().and_downcast::<gtk4::Button>() {
                         if is_pinned {
                             pin_btn.add_css_class("pinned");
                         } else {
                             pin_btn.remove_css_class("pinned");
                         }
                         pin_btn.set_visible(true);
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
    // Embed CSS at compile time to ensure it is always available
    let css_data = include_str!("../../resources/style.css");
    provider.load_from_data(css_data);
    
    // Resolve resource path
    let mut resource_path = std::path::PathBuf::from("resources");
    if !resource_path.exists() {
        println!("DEBUG: 'resources' in CWD not found. Checking candidates...");
        // Check relative to executable (AppImage/Install)
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                let candidates = [
                    parent.join("resources"),                        // AppImage / Side-by-side
                    parent.join("../../resources"),                  // Cargo target/release/
                    parent.join("../../../resources"),               // Deep nesting?
                    parent.parent().unwrap_or(parent).join("share/opennav/resources"), // Linux Install
                ];
                
                let mut found = false;
                for cand in &candidates {
                    println!("DEBUG: Checking candidate: {:?}", cand);
                    if cand.exists() {
                        println!("DEBUG: Found resources at: {:?}", cand);
                        resource_path = cand.clone();
                        found = true;
                        break;
                    }
                    // Try canonicalizing in case '..' needs resolution
                    if let Ok(canon) = cand.canonicalize() {
                         println!("DEBUG: Checking canonical: {:?}", canon);
                         if canon.exists() {
                             println!("DEBUG: Found resources at: {:?}", canon);
                             resource_path = canon;
                             found = true;
                             break;
                         }
                    }
                }
                
                if !found {
                    println!("DEBUG: Warning: Could not locate resources directory.");
                }
            }
        }
    } else {
        println!("DEBUG: Found 'resources' in CWD.");
    }

    if let Some(display) = gdk::Display::default() {
         gtk4::style_context_add_provider_for_display(
             &display,
             &provider,
             gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
         );
         
         // Add resources to icon theme search path
         let icon_theme = gtk4::IconTheme::for_display(&display);
         if resource_path.exists() {
             if let Some(path_str) = resource_path.to_str() {
                 icon_theme.add_search_path(path_str);
             }
         }
    }
    
    // Set default icon for the process (fallback for some WMs)
    // gtk4::Window::set_default_icon_name("opennav"); // This is a static method in older gtk? No, doesn't exist in gtk4::Window.
    // We rely on window instance icon name.

    // Start background tasks
    crate::data::icons::fetch_missing_icons();

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
        .placeholder_text("Open URL or Search...")
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

    // Sort by usage and pin status
    let store = Store::new().ok();
    
    // Cache engines for icon lookup
    let engines_cache = std::rc::Rc::new(std::cell::RefCell::new(Vec::<crate::data::store::SearchEngine>::new()));
    let default_engine_keyword = std::rc::Rc::new(std::cell::RefCell::new("g".to_string()));
    
    if let Some(ref s) = store {
        if let Ok(list) = s.list_engines() {
            *engines_cache.borrow_mut() = list;
        }
        if let Ok(Some(k)) = s.get_setting("search_engine") {
            // Check if it's a legacy name or keyword
             let keyword = match k.as_str() {
                 "Google" => "g",
                 "DuckDuckGo" => "d",
                 "Bing" => "b",
                 "Brave" => "br",
                 "Ecosia" => "e",
                 k => k,
             };
            *default_engine_keyword.borrow_mut() = keyword.to_string();
        }
    }
    
    // URL Icon Logic
    {
        let engines = engines_cache.clone();
        let def_kw = default_engine_keyword.clone();
        let entry_for_icon = url_entry.clone();
        let res_path = resource_path.clone();
        
        let update_icon = move || {
            let text = entry_for_icon.text();
            let text = text.as_str().trim();
            
            // Helper for Globe Icon
            let set_globe_icon = |entry: &gtk4::Entry| {
                // 1. Try custom globe.png
                let custom_globe = res_path.join("globe.png");
                if custom_globe.exists() {
                     let file = gtk4::gio::File::for_path(custom_globe);
                     if let Ok(texture) = gtk4::gdk::Texture::from_file(&file) {
                         entry.set_icon_from_paintable(gtk4::EntryIconPosition::Primary, Some(&texture));
                         return;
                     }
                }

                // 2. Fallback to Theme
                let display = gtk4::gdk::Display::default().expect("No display");
                let theme = gtk4::IconTheme::for_display(&display);
                
                if theme.has_icon("globe-symbolic") {
                    entry.set_icon_from_icon_name(gtk4::EntryIconPosition::Primary, Some("globe-symbolic"));
                } else if theme.has_icon("applications-internet") {
                    entry.set_icon_from_icon_name(gtk4::EntryIconPosition::Primary, Some("applications-internet"));
                } else {
                    entry.set_icon_from_icon_name(gtk4::EntryIconPosition::Primary, Some("network-server-symbolic"));
                }
            };
            
            // 1. Globe for Empty
            if text.is_empty() {
                set_globe_icon(&entry_for_icon);
                return;
            }
            
            // 2. Identify Type
            let is_url = text.contains("://") || (!text.contains(' ') && text.contains('.'));
            
            if !is_url {
                // It is a Search
                let parts: Vec<&str> = text.splitn(2, ' ').collect();
                
                // Determine effective keyword
                let effective_keyword = if parts.len() > 1 {
                    let potential = parts[0];
                    if engines.borrow().iter().any(|e| e.keyword == potential) {
                         potential.to_string()
                    } else {
                         def_kw.borrow().clone()
                    }
                } else {
                     def_kw.borrow().clone()
                };
                
                // Lookup Engine for this keyword
                let mut icon_name = "system-search-symbolic".to_string();
                
                if let Some(engine) = engines.borrow().iter().find(|e| e.keyword == effective_keyword) {
                    if let Some(path) = &engine.icon_path {
                        if path.contains("/") || path.contains("\\") {
                             // It's a file path (custom icon)
                             if std::path::Path::new(path).exists() {
                                 let file = gtk4::gio::File::for_path(path);
                                 if let Ok(texture) = gtk4::gdk::Texture::from_file(&file) {
                                     entry_for_icon.set_icon_from_paintable(gtk4::EntryIconPosition::Primary, Some(&texture));
                                     return;
                                 }
                             }
                             // If loading failed, fall through to fallback
                        } else {
                            // It's an icon name (theme)
                            icon_name = path.clone();
                        }
                    } else {
                         // Fallback mappings
                         icon_name = match engine.keyword.as_str() {
                             "g" => "google-chrome".to_string(), 
                             "d" => "duckduckgo".to_string(), 
                             "yt" => "youtube".to_string(),
                             "gh" => "github".to_string(),
                             "b" => "bing".to_string(),
                             "br" => "brave".to_string(),
                             "e" => "ecosia".to_string(), 
                             _ => "system-search-symbolic".to_string(),
                         };
                    }
                }
                
                // Verify Icon Existence
                let display = gtk4::gdk::Display::default().expect("No display");
                let theme = gtk4::IconTheme::for_display(&display);
                
                if theme.has_icon(&icon_name) {
                    entry_for_icon.set_icon_from_icon_name(gtk4::EntryIconPosition::Primary, Some(&icon_name));
                } else {
                     // Try alternatives
                     if icon_name == "google-chrome" && theme.has_icon("google") {
                          entry_for_icon.set_icon_from_icon_name(gtk4::EntryIconPosition::Primary, Some("google"));
                     } else if icon_name == "duckduckgo" && theme.has_icon("preferences-web-browser") {
                          entry_for_icon.set_icon_from_icon_name(gtk4::EntryIconPosition::Primary, Some("preferences-web-browser"));
                     } else {
                          // Final Fallback (Search Glass)
                          entry_for_icon.set_icon_from_icon_name(gtk4::EntryIconPosition::Primary, Some("system-search-symbolic"));
                     }
                }

            } else {
                // Is URL -> Globe
                set_globe_icon(&entry_for_icon);
            }
        };
        
        // Initial call
        update_icon();
        
        // Connect signal
        let update_clone = update_icon.clone();
        url_entry.connect_changed(move |_| {
            update_clone();
        });
    }

    // Browser List Logic
    let mut browsers = browser_repository::get_installed_browsers();
    
    if let Some(ref s) = store {
        if let Ok(stats) = s.get_stats() {
             use std::collections::HashMap;
             // id -> (usage, pinned, last_used)
             let stat_map: HashMap<String, (i64, bool, i64)> = stats.into_iter().map(|(id, count, pinned, last)| (id, (count, pinned, last))).collect();
             
             let sort_mode = s.get_setting("sort_order").ok().flatten().unwrap_or("freq".to_string());
             
             // First pass: update is_pinned in struct
             for browser in &mut browsers {
                 if let Some((_, pinned, _)) = stat_map.get(&browser.id) {
                     browser.is_pinned = *pinned;
                 }
             }
             
             browsers.sort_by(|a, b| {
                 // Pin status first (true > false)
                 b.is_pinned.cmp(&a.is_pinned)
                     .then_with(|| {
                         match sort_mode.as_str() {
                             "recent" => {
                                 let last_a = stat_map.get(&a.id).map(|x| x.2).unwrap_or(0);
                                 let last_b = stat_map.get(&b.id).map(|x| x.2).unwrap_or(0);
                                 last_b.cmp(&last_a) // Newest first
                             },
                             "alpha" => {
                                 std::cmp::Ordering::Equal // Defer to name sort at end
                             },
                             _ => { // "freq" or default
                                 let count_a = stat_map.get(&a.id).map(|x| x.0).unwrap_or(0);
                                 let count_b = stat_map.get(&b.id).map(|x| x.0).unwrap_or(0);
                                 count_b.cmp(&count_a) // Highest first
                             }
                         }
                     })
                     .then_with(|| a.name.cmp(&b.name)) // Tie breaker Name ASC
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
    
    // Clones for Setup (Pin Button)
    let browsers_setup = browsers_rc.clone();
    let pinned_map_setup = pinned_map.clone();
    let active_rows_setup = active_rows.clone();
    let search_query_setup = search_query.clone();
    
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
        
        // Pin Button (Button instead of Image)
        let pin_btn = gtk4::Button::builder()
            .icon_name("view-pin-symbolic")
            .css_classes(vec!["pin-btn".to_string()])
            .halign(Align::End)
            .valign(Align::Center)
            .build();
            
        // Handler for Pin Click
        let browsers_pin = browsers_setup.clone();
        let pinned_map_pin = pinned_map_setup.clone();
        let active_rows_pin = active_rows_setup.clone(); 
        let search_query_pin = search_query_setup.clone();
        
        pin_btn.connect_clicked(move |btn| {
             // Avoid row activation by stopping propagation? Button does this naturally.
             // Find browser name
             if let Some(row) = btn.ancestor(gtk4::ListBoxRow::static_type()).or_else(|| btn.parent().and_then(|p| p.parent())) { // Used inside ListView, parent is HBox, then ListItem
                  // Getting usage of ListItem is tricky to resolve data directly.
                  // We can look at the hidden label we added!
                  if let Some(hbox) = btn.parent().and_then(|p| p.downcast::<GtkBox>().ok()) {
                       if let Some(last) = hbox.last_child() {
                           if let Some(lbl) = last.downcast_ref::<Label>() {
                               let name = lbl.text();
                               if !name.is_empty() {
                                   if let Some(browser) = browsers_pin.iter().find(|b| b.name == name.as_str()) {
                                        if let Ok(store) = Store::new() {
                                            if let Ok(new_state) = store.toggle_pin(&browser.id) {
                                                pinned_map_pin.borrow_mut().insert(browser.name.clone(), new_state);
                                                
                                                // Refresh Rows
                                                let query = search_query_pin.borrow();
                                                refresh_rows(&active_rows_pin, &query, &pinned_map_pin.borrow());
                                            }
                                        }
                                   }
                               }
                           }
                       }
                  }
             }
        });
        
        hbox.append(&icon);
        hbox.append(&label);
        hbox.append(&pin_btn);
        
        // Hidden Label for Data Transfer
        let hidden_label = Label::new(None);
        hidden_label.set_visible(false);
        hbox.append(&hidden_label);

        // Click Handling (Row Launch)
        let gesture = gtk4::GestureClick::new();
        let browsers_inner = browsers_for_click.clone();
        let url_inner = url_entry_weak_click.clone();
        let win_inner = window_weak_click.clone();
        
        gesture.connect_released(move |gesture, _, _, _| {
             let modifiers = gesture.current_event().map(|e| e.modifier_state()).unwrap_or_else(gtk4::gdk::ModifierType::empty);
             let keep_open = modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK);
             
             let widget = gesture.widget().expect("Widget attached");
             // If click was on the button, this gesture might catch it if propagation bubbles?
             // Button claims the sequence usually.
             
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
        
        // 0: Icon, 1: Label, 2: Pin Button
        if let Some(child0) = hbox.first_child() {
             // Icon Logic
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
                     
                     if let Some(pin_btn) = child1.next_sibling().and_downcast::<gtk4::Button>() {
                         // Pin Button Logic
                         if is_pinned {
                             pin_btn.add_css_class("pinned");
                         } else {
                             pin_btn.remove_css_class("pinned");
                         }
                         pin_btn.set_visible(true); // Always visible, CSS handles opacity
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
    let resource_path_for_settings = resource_path.clone();
    settings_btn.connect_clicked(move |_| {
        if let Some(parent) = window_weak_for_settings.upgrade() {
            let dialog = gtk4::Window::builder()
                .transient_for(&parent)
                .modal(true)
                .title("Settings")
                .default_width(600)
                .default_height(550)
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
            let logo_path = resource_path_for_settings.join("app-icon.png");
            let logo = if logo_path.exists() {
                gtk4::Image::from_file(&logo_path)
            } else {
                gtk4::Image::from_file("resources/app-icon.png") // Fallback
            };
            logo.set_pixel_size(64);
            vbox.append(&logo);
            
            // About Section
            let about_label = Label::new(Some("OpenNav v1.1.0\n<span size='small'>A fast browser picker and launcher.</span>"));
            about_label.set_use_markup(true);
            about_label.set_justify(gtk4::Justification::Center);
            vbox.append(&about_label);
            
            // Separator
            vbox.append(&gtk4::Separator::new(Orientation::Horizontal));

            // Browser Sort Options
            let sort_box = GtkBox::new(Orientation::Vertical, 10);
            let sort_label = Label::new(Some("<b>Browser List Order</b>"));
            sort_label.set_use_markup(true);
            sort_label.set_halign(Align::Start);
            sort_box.append(&sort_label);
            
            let row_sort = GtkBox::new(Orientation::Horizontal, 10);
            
            // Dropdown
            let sort_items = ["Alphabetical", "Recently Used", "Frequently Used"];
            let model = StringList::new(&sort_items);
            let dropdown = gtk4::DropDown::new(Some(model), None::<&gtk4::Expression>);
            dropdown.set_hexpand(true);
            
            // Determine initial selection
            let current_sort = if let Ok(store) = Store::new() {
                 store.get_setting("sort_order").ok().flatten().unwrap_or("freq".to_string())
            } else {
                 "freq".to_string()
            };
            
            let initial_idx = match current_sort.as_str() {
                "alpha" => 0,
                "recent" => 1,
                _ => 2, // freq
            };
            dropdown.set_selected(initial_idx);
            
            row_sort.append(&dropdown);
            
            // Reset Button
            let reset_btn = gtk4::Button::with_label("Reset");
            reset_btn.add_css_class("suggested-action");
            reset_btn.set_width_request(100);
            // Initial state
            reset_btn.set_sensitive(initial_idx != 0);
            
            // Logic for Dropdown Change
            let reset_btn_clone = reset_btn.clone();
            dropdown.connect_selected_notify(move |d| {
                let idx = d.selected();
                let key = match idx {
                    0 => "alpha",
                    1 => "recent",
                    _ => "freq",
                };
                
                // Save setting
                if let Ok(store) = Store::new() {
                    let _ = store.set_setting("sort_order", key);
                }
                
                // Update Reset Button
                reset_btn_clone.set_sensitive(idx != 0);
            });
            
            // Logic for Reset Click
            let dropdown_clone = dropdown.clone();
            reset_btn.connect_clicked(move |_| {
                let idx = dropdown_clone.selected();
                if let Ok(store) = Store::new() {
                     match idx {
                         1 => { let _ = store.reset_recent_stats(); },
                         2 => { let _ = store.reset_frequent_stats(); },
                         _ => {}
                     }
                }
            });
            
            row_sort.append(&reset_btn);
            sort_box.append(&row_sort);
            
            vbox.append(&sort_box);
            
            // Separator
            vbox.append(&gtk4::Separator::new(Orientation::Horizontal));
       
            // Embed Search Engine Management UI
            let engines_ui = crate::ui::engines_dialog::build_engine_management_ui();
            engines_ui.set_vexpand(true);
            vbox.append(&engines_ui);

            // Separator
            vbox.append(&gtk4::Separator::new(Orientation::Horizontal));
            
            let close_btn = gtk4::Button::with_label("Close");
            close_btn.add_css_class("suggested-action");
            close_btn.set_width_request(100);
            
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
    let list_view_weak = list_view.downgrade();
    
    // Buttons for shortcuts
    let help_btn_weak = help_btn.downgrade();
    let settings_btn_weak = settings_btn.downgrade();

    key_controller.connect_key_pressed(move |_controller, key, _keycode, modifiers| {
        // Handle Esc globally (Highest priority)
        if key == gtk4::gdk::Key::Escape {
            if let Some(window) = window_weak.upgrade() {
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
                return gtk4::glib::Propagation::Stop;
            }
        }

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
            // Esc handled at top


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
                    if let Some(lv) = list_view_weak.upgrade() {
                        lv.grab_focus();
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
            
            // Fix: Grab focus on ListView to prevent it drifting to buttons (e.g. Shortcuts)
            if let Some(lv) = list_view_weak.upgrade() {
                lv.grab_focus();
            }
            
            // Also focus back in Backspace logic?
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
