#[path = "../grid.rs"]
mod grid;
fn main() -> anyhow::Result<()> {
    grid::run()
}
