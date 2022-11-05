mod publish;
mod render;

use anyhow::Result;
use argh::FromArgs;
use command_run::Command;

/// nbishop-net renderer and publisher.
#[derive(FromArgs)]
struct Opt {
    /// open in a web browser.
    #[argh(switch)]
    open: bool,

    /// publish after building.
    #[argh(switch)]
    publish: bool,
}

fn main() -> Result<()> {
    let opt: Opt = argh::from_env();

    render::render()?;

    if opt.open {
        Command::with_args("xdg-open", ["output/index.html"]).run()?;
    }

    if opt.publish {
        publish::publish()?;
    }

    Ok(())
}
