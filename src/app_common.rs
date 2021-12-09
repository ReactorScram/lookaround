use crate::prelude::*;

#[derive (Debug, thiserror::Error)]
pub enum AppError {
	#[error (transparent)]
	AddrParse (#[from] std::net::AddrParseError),
	#[error (transparent)]
	CliArgs (#[from] CliArgError),
	#[error (transparent)]
	Io (#[from] std::io::Error),
	#[error (transparent)]
	Ip (#[from] crate::ip::IpError),
	#[error (transparent)]
	Join (#[from] tokio::task::JoinError),
	#[error (transparent)]
	MacAddr (#[from] mac_address::MacAddressError),
	#[error (transparent)]
	Message (#[from] crate::message::MessageError),
	#[error (transparent)]
	ParseInt (#[from] std::num::ParseIntError),
	#[error (transparent)]
	Tlv (#[from] crate::tlv::TlvError),
}

#[derive (Debug, thiserror::Error)]
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

pub async fn recv_msg_from (socket: &UdpSocket) -> Result <(Vec <Message>, SocketAddr), AppError> 
{
	let mut buf = vec! [0u8; PACKET_SIZE];
	let (bytes_recved, remote_addr) = socket.recv_from (&mut buf).await?;
	buf.truncate (bytes_recved);
	let msgs = Message::from_slice2 (&buf)?;
	
	Ok ((msgs, remote_addr))
}

#[derive (Clone)]
pub struct Params {
	// Servers bind on this port, clients must send to the port
	pub server_port: u16,
	
	// Clients and servers will all join the same multicast addr
	pub multicast_addr: Ipv4Addr,
}

impl Default for Params {
	fn default () -> Self {
		Self {
			server_port: 9040,
			multicast_addr: Ipv4Addr::new (225, 100, 99, 98),
		}
	}
}
