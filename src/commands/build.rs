use crate::content;
use crate::error::Result;
use crate::site::render;

pub fn run(site_url: &str) -> Result<()> {
    let content = content::load()?;
    render::build(&content, site_url)?;
    println!("built dist/");
    Ok(())
}
