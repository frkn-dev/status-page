mod app;
mod i18n;
mod network;
mod services;

use app::App;
use leptos::*;

fn main() {
    mount_to_body(|| view! { <App/> });
}
