use crate::content;
use crate::error::Result;

pub fn run() -> Result<()> {
    content::load()?;
    println!("content check passed");
    Ok(())
}
