mod app;
mod wg_gen;
mod endpoint;

use app::App;

fn main() {
    yew::Renderer::<App>::new().render();
}
