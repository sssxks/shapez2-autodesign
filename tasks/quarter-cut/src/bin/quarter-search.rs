#[path = "../search/mod.rs"]
mod search;
fn main() -> anyhow::Result<()> {
    search::run()
}
