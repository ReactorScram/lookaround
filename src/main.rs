use std::{
	env,
	net::{
		Ipv4Addr,
		SocketAddrV4,
		UdpSocket,
	},
};

use mac_address::get_mac_address;
use thiserror::Error;

#[derive (Debug, Error)]
enum AppError {
	#[error (transparent)]
	CliArgs (#[from] CliArgError),
}

#[derive (Debug, Error)]
enum CliArgError {
	#[error ("First argument should be a subcommand")]
	MissingSubcommand,
	#[error ("Unknown subcommand `{0}`")]
	UnknownSubcommand (String),
}

struct CommonParams {
	// Servers bind on this port, clients must send to the port
	server_port: u16,
	
	// Clients and servers will all join the same multicast addr
	multicast_addr: Ipv4Addr,
}

impl Default for CommonParams {
	fn default () -> Self {
		Self {
			server_port: 9040,
			multicast_addr: Ipv4Addr::new (225, 100, 99, 98),
		}
	}
}

fn main () -> Result <(), AppError> {
	let mut args = env::args ();
	
	let _exe_name = args.next ();
	
	match get_mac_address() {
		Ok(Some(ma)) => {
			println!("MAC addr = {}", ma);
			println!("bytes = {:?}", ma.bytes());
		}
		Ok(None) => println!("No MAC address found."),
		Err(e) => println!("{:?}", e),
	}
	
	match args.next ().as_ref ().map (|s| &s[..]) {
		None => return Err (CliArgError::MissingSubcommand.into ()),
		Some ("client") => client ()?,
		Some ("server") => server ()?,
		Some (x) => return Err (CliArgError::UnknownSubcommand (x.to_string ()).into ()),
	}
	
	Ok (())
}

fn client () -> Result <(), AppError> {
	let params = CommonParams::default ();
	let socket = UdpSocket::bind ("0.0.0.0:0").unwrap ();
	
	socket.join_multicast_v4 (&params.multicast_addr, &([0u8, 0, 0, 0].into ())).unwrap ();
	
	socket.send_to ("hi there".as_bytes (), (params.multicast_addr, params.server_port)).unwrap ();
	
	let mut buf = vec! [0u8; 4096];
	let (bytes_recved, remote_addr) = socket.recv_from (&mut buf).unwrap ();
	buf.truncate (bytes_recved);
	let _buf = buf;
	dbg! (remote_addr);
	
	Ok (())
}

fn server () -> Result <(), AppError> {
	let params = CommonParams::default ();
	let socket = UdpSocket::bind (SocketAddrV4::new (Ipv4Addr::UNSPECIFIED, params.server_port)).unwrap ();
	
	socket.join_multicast_v4 (&params.multicast_addr, &([0u8, 0, 0, 0].into ())).unwrap ();
	
	let mut buf = vec! [0u8; 4096];
	let (bytes_recved, remote_addr) = socket.recv_from (&mut buf).unwrap ();
	buf.truncate (bytes_recved);
	let _buf = buf;
	dbg! (remote_addr);
	
	socket.send_to ("hi there".as_bytes (), remote_addr).unwrap ();
	
	Ok (())
}
