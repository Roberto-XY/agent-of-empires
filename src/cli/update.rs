//! `agent-of-empires update` command implementation

use anyhow::Result;
use clap::Args;
use std::io::{self, Write};

use crate::update;

#[derive(Args)]
pub struct UpdateArgs {
    /// Only check for updates, don't install
    #[arg(long)]
    check: bool,

    /// Force check (bypass cache)
    #[arg(long)]
    force: bool,
}

pub async fn run(args: UpdateArgs) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");

    println!("Agent of Empires v{}", version);
    println!("Checking for updates...");

    let info = update::check_for_update(version, args.force).await?;

    if !info.available {
        println!("✓ You're running the latest version!");
        return Ok(());
    }

    println!();
    println!(
        "⬆ Update available: v{} → v{}",
        info.current_version, info.latest_version
    );
    println!("  Release: {}", info.release_url);

    if args.check {
        println!();
        println!("Run 'agent-of-empires update' to install.");
        return Ok(());
    }

    print!("\nInstall update? [Y/n] ");
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;
    let response = response.trim().to_lowercase();

    if !response.is_empty() && response != "y" && response != "yes" {
        println!("Update cancelled.");
        return Ok(());
    }

    println!();
    update::perform_update(&info.download_url).await?;

    println!();
    println!("✓ Updated to v{}", info.latest_version);
    println!("  Restart agent-of-empires to use the new version.");

    Ok(())
}
