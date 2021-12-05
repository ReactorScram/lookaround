use std::{
	env,
	net::{
		Ipv4Addr,
		SocketAddr,
		SocketAddrV4,
		UdpSocket,
	},
};

use mac_address::get_mac_address;
use thiserror::Error;

mod message;
mod tlv;

use message::{
	PACKET_SIZE,
	Message,
};

#[derive (Debug, Error)]
enum AppError {
	#[error (transparent)]
	CliArgs (#[from] CliArgError),
	#[error (transparent)]
	Io (#[from] std::io::Error),
	#[error (transparent)]
	MacAddr (#[from] mac_address::MacAddressError),
	#[error (transparent)]
	Message (#[from] message::MessageError),
	#[error (transparent)]
	Tlv (#[from] tlv::TlvError),
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
	
	let msg = Message::Request (None).to_vec ()?;
	
	socket.send_to (&msg, (params.multicast_addr, params.server_port)).unwrap ();
	
	let (resp, remote_addr) = recv_msg_from (&socket)?;
	
	dbg! (remote_addr);
	
	Ok (())
}

fn server () -> Result <(), AppError> {
	let our_mac = get_mac_address ()?.map (|x| x.bytes ());
	if our_mac.is_none () {
		println! ("Warning: Can't find our own MAC address. We won't be able to respond to MAC-specific lookaround requests");
	}
	
	let params = CommonParams::default ();
	let socket = UdpSocket::bind (SocketAddrV4::new (Ipv4Addr::UNSPECIFIED, params.server_port)).unwrap ();
	
	socket.join_multicast_v4 (&params.multicast_addr, &([0u8, 0, 0, 0].into ())).unwrap ();
	
	let (req, remote_addr) = recv_msg_from (&socket)?;
	dbg! (remote_addr);
	
	let resp = match req {
		Message::Request (None) => {
			Some (Message::Response (our_mac))
		},
		_ => None,
	};
	
	if let Some (resp) = resp {
		socket.send_to (&resp.to_vec ()?, remote_addr).unwrap ();
	}
	
	Ok (())
}

fn recv_msg_from (socket: &UdpSocket) -> Result <(Message, SocketAddr), AppError> 
{
	let mut buf = vec! [0u8; PACKET_SIZE];
	let (bytes_recved, remote_addr) = socket.recv_from (&mut buf)?;
	buf.truncate (bytes_recved);
	let msg = Message::from_slice (&buf)?;
	
	Ok ((msg, remote_addr))
}
