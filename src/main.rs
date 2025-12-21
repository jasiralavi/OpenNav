use std::env;

mod app;
mod data;
mod ui;

fn main() {
    let args: Vec<String> = env::args().collect();
    let url = args.get(1).cloned();
    
    let app = app::App::new(url);
    app.run();
}
