mod driver;

use actix::prelude::*;
use anyhow::Result;

#[actix_rt::main]
async fn main() -> Result<()> {
    let mut driver = driver::Driver::init()?;
    driver.main_loop()
}
