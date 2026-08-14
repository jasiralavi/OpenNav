use std::env;

mod app;
mod data;
mod ui;

fn main() {
    let app = app::App::new();
    app.run();
}
