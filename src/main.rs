mod render;

use anyhow::Result;
use argh::FromArgs;

/// nbishop-net renderer and publisher.
#[derive(FromArgs)]
struct Opt {
    /// publish after building.
    #[argh(switch)]
    publish: bool,
}

fn main() -> Result<()> {
    let opt: Opt = argh::from_env();

    render::render()?;

    Ok(())
}
