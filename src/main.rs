use std::{
	collections::HashMap,
	env,
	net::{
		Ipv4Addr,
		SocketAddr,
		SocketAddrV4,
		UdpSocket,
	},
	time::{Duration, Instant},
};

use mac_address::{
	MacAddress,
	get_mac_address,
};
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
			println!("Our MAC addr = {}", ma);
		}
		Ok(None) => println!("No MAC address found."),
		Err(e) => println!("{:?}", e),
	}
	
	let subcommand: Option <String> = args.next ();
	
	match subcommand.as_ref ().map (|x| &x[..]) {
		None => return Err (CliArgError::MissingSubcommand.into ()),
		Some ("client") => client ()?,
		Some ("server") => server (args)?,
		Some (x) => return Err (CliArgError::UnknownSubcommand (x.to_string ()).into ()),
	}
	
	Ok (())
}

fn client () -> Result <(), AppError> {
	use rand::RngCore;
	
	let mut common_params = CommonParams::default ();
	let socket = UdpSocket::bind ("0.0.0.0:0")?;
	
	socket.join_multicast_v4 (&common_params.multicast_addr, &([0u8, 0, 0, 0].into ()))?;
	socket.set_read_timeout (Some (Duration::from_millis (1_000)))?;
	
	let mut idem_id = [0u8; 8];
	rand::thread_rng ().fill_bytes (&mut idem_id);
	
	let msg = Message::Request {
		idem_id,
		mac: None,
	}.to_vec ()?;
	
	for _ in 0..10 {
		socket.send_to (&msg, (common_params.multicast_addr, common_params.server_port))?;
		std::thread::sleep (Duration::from_millis (100));
	}
	
	let start_time = Instant::now ();
	
	let mut peers = HashMap::with_capacity (10);
	
	while Instant::now () < start_time + Duration::from_secs (2) {
		let (resp, remote_addr) = match recv_msg_from (&socket) {
			Err (_) => continue,
			Ok (x) => x,
		};
		
		let peer_mac_addr = match resp {
			Message::Response (mac) => mac,
			_ => continue,
		};
		
		peers.insert (remote_addr, peer_mac_addr);
	}
	
	let mut peers: Vec <_> = peers.into_iter ().collect ();
	peers.sort ();
	
	println! ("Found {} peers:", peers.len ());
	for (ip, mac) in &peers {
		match mac {
			Some (mac) => println! ("{} = {}", MacAddress::new (*mac), ip.ip ()),
			None => println! ("<Unknown> = {}", ip),
		}
	}
	
	Ok (())
}

fn server <I: Iterator <Item=String>> (args: I) -> Result <(), AppError> 
{
	let mut common_params = CommonParams::default ();
	let mut nickname = String::new ();
	
	let our_mac = get_mac_address ()?.map (|x| x.bytes ());
	if our_mac.is_none () {
		println! ("Warning: Can't find our own MAC address. We won't be able to respond to MAC-specific lookaround requests");
	}
	
	let socket = UdpSocket::bind (SocketAddrV4::new (Ipv4Addr::UNSPECIFIED, common_params.server_port)).unwrap ();
	
	socket.join_multicast_v4 (&common_params.multicast_addr, &([0u8, 0, 0, 0].into ())).unwrap ();
	
	let mut recent_idem_ids = Vec::with_capacity (32);
	
	loop {
		println! ("Waiting for messages...");
		let (req, remote_addr) = recv_msg_from (&socket)?;
		
		let resp = match req {
			Message::Request {
				mac: None,
				idem_id,
			} => {
				if recent_idem_ids.contains (&idem_id) {
					println! ("Ignoring request we already processed");
					None
				}
				else {
					recent_idem_ids.insert (0, idem_id);
					recent_idem_ids.truncate (30);
					Some (Message::Response (our_mac))
				}
			},
			_ => continue,
		};
		
		if let Some (resp) = resp {
			socket.send_to (&resp.to_vec ()?, remote_addr).unwrap ();
		}
	}
}

fn recv_msg_from (socket: &UdpSocket) -> Result <(Message, SocketAddr), AppError> 
{
	let mut buf = vec! [0u8; PACKET_SIZE];
	let (bytes_recved, remote_addr) = socket.recv_from (&mut buf)?;
	buf.truncate (bytes_recved);
	let msg = Message::from_slice (&buf)?;
	
	Ok ((msg, remote_addr))
}
