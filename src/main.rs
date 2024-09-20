#![deny(clippy::all, clippy::pedantic)]
#![allow(non_snake_case, clippy::module_name_repetitions)]

use leptos::{component, view, IntoView};
use leptos_router::{Route, Router, Routes, A};
use tracing::Level;
use wasm_tracing::WASMLayerConfigBuilder;

use crate::home::Home;
use crate::log::Log;
use crate::measures::Measures;

mod binary_search;
mod components;
mod datetime;
mod home;
mod log;
mod measures;
mod request;
mod types;
mod use_canvas;

fn main() {
    console_error_panic_hook::set_once();
    wasm_tracing::set_as_global_default_with_config(
        WASMLayerConfigBuilder::new()
            .set_max_level(Level::INFO)
            .build(),
    );
    leptos::mount_to_body(|| view! { <App /> });
}

#[component]
fn App() -> impl IntoView {
    view! {
        <main class="container-fluid">
            <Router>
                <nav>
                    <ul>
                        <li>
                            <strong>
                                <A href="/">"Sirius"</A>
                            </strong>
                        </li>
                    </ul>
                </nav>
                <div class="h-full w-full overflow-auto">
                    <Routes>
                        <Route path="/" view=Home />
                        <Route path="/measures/:id" view=Measures />
                        <Route path="/log/:id" view=Log />
                        <Route path="/*any" view=|| view! { <h1>"Not Found"</h1> } />
                    </Routes>
                </div>
            </Router>
        </main>
    }
}
