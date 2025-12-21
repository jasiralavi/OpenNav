use gtk4::prelude::*;
use gtk4::{Application, gio};

pub struct App {
    pub app: Application,
}

impl App {
    pub fn new(url: Option<String>) -> Self {
        let app = Application::builder()
            .application_id("com.opennav.app")
            .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
            .build();
            
        let url_clone = url.clone();
        app.connect_activate(move |app| {
            crate::ui::window::build_ui(app, url_clone.as_deref());
        });
        
        // When HANDLES_COMMAND_LINE is set, we must handle the command-line signal
        // or the app won't activate properly with args.
        app.connect_command_line(|app, _cmd| {
            app.activate();
            0
        });
        
        App { app }
    }
    
    pub fn run(&self) {
        self.app.run();
    }
}
