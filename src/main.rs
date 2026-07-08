use std::{env, error::Error};

use npa_web::routes::{app, bind_addr_from_args};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let bind_addr = bind_addr_from_args(env::args())
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;
    let listener = TcpListener::bind(bind_addr).await?;

    eprintln!("npa-web listening on http://{bind_addr}");
    axum::serve(listener, app()?).await?;

    Ok(())
}
