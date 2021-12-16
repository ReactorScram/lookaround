use prelude::*;

pub mod app_common;
mod avalanche;
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
	
	let subcommand: Option <String> = args.next ();
	
	match subcommand.as_ref ().map (|x| &x[..]) {
		None => return Err (CliArgError::MissingSubcommand.into ()),
		Some ("--version") => println! ("lookaround v{}", LOOKAROUND_VERSION),
		Some ("client") => client::client (args).await?,
		Some ("config") => config (),
		Some ("debug-avalanche") => avalanche::debug (),
		Some ("find-nick") => client::find_nick (args).await?,
		Some ("my-ips") => my_ips ()?,
		Some ("server") => server::server (args).await?,
		Some (x) => return Err (CliArgError::UnknownSubcommand (x.to_string ()).into ()),
	}
	
	Ok (())
}

fn config () {
	if let Some (proj_dirs) = ProjectDirs::from ("", "ReactorScram", "LookAround") {
		println! ("Using config dir {:?}", proj_dirs.config_local_dir ());
	}
	else {
		println! ("Can't detect config dir.");
	}
}

fn my_ips () -> Result <(), AppError> {
	for addr in ip::get_ips ()?
	{
		println! ("{:?}", addr);
	}
	
	Ok (())
}
