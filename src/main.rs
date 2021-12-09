use prelude::*;

pub mod app_common;
mod client;
mod ip;
pub mod message;
mod prelude;
mod server;
pub mod tlv;

fn main () -> Result <(), AppError> {
	let rt = tokio::runtime::Builder::new_current_thread ()
	.enable_io ()
	.enable_time ()
	.build ()?;
	
	rt.block_on (async_main ())?;
	
	Ok (())
}

async fn async_main () -> Result <(), AppError> {
	let mut args = env::args ();
	
	let _exe_name = args.next ();
	
	match get_mac_address() {
		Ok(Some(ma)) => {
			println!("Our MAC addr = {}", ma);
		}
		Ok(None) => println!("No MAC address found."),
		Err(e) => println!("{:?}", e),
	}
	
	let subcommand: Option <String> = args.next ();
	
	match subcommand.as_ref ().map (|x| &x[..]) {
		None => return Err (CliArgError::MissingSubcommand.into ()),
		Some ("client") => client::client (args).await?,
		Some ("my-ips") => my_ips ()?,
		Some ("server") => server::server (args).await?,
		Some (x) => return Err (CliArgError::UnknownSubcommand (x.to_string ()).into ()),
	}
	
	Ok (())
}

fn my_ips () -> Result <(), AppError> {
	for addr in ip::get_ips ()?
	{
		println! ("{:?}", addr);
	}
	
	Ok (())
}
