use gtk4::prelude::*;
use gtk4::{Application, gio};

pub struct App {
    pub app: Application,
}

impl App {
    pub fn new() -> Self {
        let app = Application::builder()
            .application_id("com.opennav.app")
            .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
            .build();
            
        // Default activation (no args)
        app.connect_activate(move |app| {
            crate::ui::window::build_ui(app, None);
        });
        
        // Command line activation (with args)
        // This handles both primary arg parsing and secondary instance activation
        app.connect_command_line(move |app, cmd| {
            let args = cmd.arguments();
            let url = if args.len() > 1 {
                Some(args[1].to_string_lossy().to_string())
            } else {
                None
            };
            
            crate::ui::window::build_ui(app, url.as_deref());
            0
        });
        
        App { app }
    }
    
    pub fn run(&self) {
        self.app.run();
    }
}
