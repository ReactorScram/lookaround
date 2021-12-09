use std::{
	collections::HashMap,
	env,
	net::{
		Ipv4Addr,
		SocketAddr,
		SocketAddrV4,
		UdpSocket,
	},
	str::FromStr,
	time::{Duration, Instant},
};

use mac_address::{
	MacAddress,
	get_mac_address,
};
use thiserror::Error;

mod ip;
mod message;
mod tlv;

use message::{
	PACKET_SIZE,
	Message,
};

#[derive (Debug, Error)]
pub enum AppError {
	#[error (transparent)]
	AddrParse (#[from] std::net::AddrParseError),
	#[error (transparent)]
	CliArgs (#[from] CliArgError),
	#[error (transparent)]
	FromUtf8 (#[from] std::string::FromUtf8Error),
	#[error (transparent)]
	Io (#[from] std::io::Error),
	#[error (transparent)]
	MacAddr (#[from] mac_address::MacAddressError),
	#[error (transparent)]
	Message (#[from] message::MessageError),
	#[error (transparent)]
	Tlv (#[from] tlv::TlvError),
	#[error (transparent)]
	Utf8 (#[from] std::str::Utf8Error),
}

#[derive (Debug, Error)]
pub enum CliArgError {
	#[error ("Missing value for argument `{0}`")]
	MissingArgumentValue (String),
	#[error ("First argument should be a subcommand")]
	MissingSubcommand,
	#[error ("Unknown subcommand `{0}`")]
	UnknownSubcommand (String),
	#[error ("Unrecognized argument `{0}`")]
	UnrecognizedArgument (String),
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
	let rt = tokio::runtime::Builder::new_current_thread ().build ()?;
	
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
		Some ("client") => client (args)?,
		Some ("my-ips") => my_ips ()?,
		Some ("server") => server (args)?,
		Some (x) => return Err (CliArgError::UnknownSubcommand (x.to_string ()).into ()),
	}
	
	Ok (())
}

#[cfg(target_os = "linux")]
fn my_ips () -> Result <(), AppError> {
	let output = ip::linux::get_ip_addr_output ()?;
	
	for addr in ip::linux::parse_ip_addr_output (&output)
	.iter ()
	.filter (|a| ! a.is_loopback ())
	{
		println! ("{:?}", addr);
	}
	
	Ok (())
}

#[cfg(target_os = "macos")]
fn my_ips () -> Result <(), AppError> {
	println! ("my-ips subcommand not implemented for macos");
	Ok (())
}

#[cfg(target_os = "windows")]
fn my_ips () -> Result <(), AppError> {
	let output = ip::windows::get_ip_config_output ()?;
	
	for addr in ip::windows::parse_ip_config_output (&output) {
		println! ("{:?}", addr);
	}
	
	Ok (())
}

struct ServerResponse {
	mac: Option <[u8; 6]>,
	nickname: Option <String>,
}

fn client <I : Iterator <Item=String>> (mut args: I) -> Result <(), AppError> {
	use rand::RngCore;
	
	let mut common_params = CommonParams::default ();
	let mut bind_addr = "0.0.0.0".to_string ();
	
	while let Some (arg) = args.next () {
		match arg.as_str () {
			"--bind-addr" => {
				bind_addr = match args.next () {
					None => return Err (CliArgError::MissingArgumentValue (arg).into ()),
					Some (x) => x
				};
			},
			_ => return Err (CliArgError::UnrecognizedArgument (arg).into ()),
		}
	}
	
	let socket = UdpSocket::bind (&format! ("{}:0", bind_addr))?;
	
	socket.join_multicast_v4 (&common_params.multicast_addr, &Ipv4Addr::from_str (&bind_addr)?)?;
	socket.set_read_timeout (Some (Duration::from_millis (1_000)))?;
	
	let mut idem_id = [0u8; 8];
	rand::thread_rng ().fill_bytes (&mut idem_id);
	
	let msg = Message::Request1 {
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
		let (msgs, remote_addr) = match recv_msg_from (&socket) {
			Err (_) => continue,
			Ok (x) => x,
		};
		
		let mut resp = ServerResponse {
			mac: None,
			nickname: None,
		};
		
		for msg in msgs.into_iter () {
			match msg {
				Message::Response1 (x) => resp.mac = x,
				Message::Response2 (x) => resp.nickname = Some (x.nickname),
				_ => (),
			}
		}
		
		peers.insert (remote_addr, resp);
	}
	
	let mut peers: Vec <_> = peers.into_iter ().collect ();
	peers.sort_by_key (|(k, v)| v.mac);
	
	println! ("Found {} peers:", peers.len ());
	for (ip, resp) in peers.into_iter () {
		let mac = match resp.mac {
			None => {
				println! ("<Unknown> = {}", ip);
				continue;
			},
			Some (x) => x,
		};
		
		let nickname = match resp.nickname {
			None => {
				println! ("{} = {}", MacAddress::new (mac), ip.ip ());
				continue;
			},
			Some (x) => x,
		};
		
		println! ("{} = {} `{}`", MacAddress::new (mac), ip.ip (), nickname);
	}
	
	Ok (())
}

fn server <I: Iterator <Item=String>> (mut args: I) -> Result <(), AppError> 
{
	let mut common_params = CommonParams::default ();
	let mut bind_addr = "0.0.0.0".to_string ();
	let mut nickname = String::new ();
	
	while let Some (arg) = args.next () {
		match arg.as_str () {
			"--bind-addr" => {
				bind_addr = match args.next () {
					None => return Err (CliArgError::MissingArgumentValue (arg).into ()),
					Some (x) => x
				};
			},
			"--nickname" => {
				nickname = match args.next () {
					None => return Err (CliArgError::MissingArgumentValue (arg).into ()),
					Some (x) => x
				};
			},
			_ => return Err (CliArgError::UnrecognizedArgument (arg).into ()),
		}
	}
	
	let our_mac = get_mac_address ()?.map (|x| x.bytes ());
	if our_mac.is_none () {
		println! ("Warning: Can't find our own MAC address. We won't be able to respond to MAC-specific lookaround requests");
	}
	
	let socket = UdpSocket::bind (SocketAddrV4::new (Ipv4Addr::from_str (&bind_addr)?, common_params.server_port)).unwrap ();
	
	socket.join_multicast_v4 (&common_params.multicast_addr, &([0u8, 0, 0, 0].into ())).unwrap ();
	
	let mut recent_idem_ids = Vec::with_capacity (32);
	
	loop {
		println! ("Waiting for messages...");
		let (req_msgs, remote_addr) = recv_msg_from (&socket)?;
		
		let req = match req_msgs.into_iter ().next () {
			Some (x) => x,
			_ => {
				println! ("Don't know how to handle this message, ignoring");
				continue;
			},
		};
		
		let resp = match req {
			Message::Request1 {
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
					Some (vec! [
						Message::Response1 (our_mac),
						Message::Response2 (message::Response2 {
							idem_id,
							nickname: nickname.clone (),
						}),
					])
				}
			},
			_ => continue,
		};
		
		if let Some (resp) = resp {
			socket.send_to (&Message::many_to_vec (&resp)?, remote_addr).unwrap ();
		}
	}
}

fn recv_msg_from (socket: &UdpSocket) -> Result <(Vec <Message>, SocketAddr), AppError> 
{
	let mut buf = vec! [0u8; PACKET_SIZE];
	let (bytes_recved, remote_addr) = socket.recv_from (&mut buf)?;
	buf.truncate (bytes_recved);
	let msgs = Message::from_slice2 (&buf)?;
	
	Ok ((msgs, remote_addr))
}
