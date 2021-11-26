use std::{
	env,
	net::{
		Ipv4Addr,
		UdpSocket,
	},
	str::FromStr,
};

use anyhow::{
	self,
	Result,
	bail,
};

fn main () -> Result <()> {
	let mut args = env::args ();
	
	let _exe_name = args.next ();
	
	match args.next ().as_ref ().map (|s| &s[..]) {
		None => bail! ("First argument must be a subcommand"),
		Some ("client") => client ()?,
		Some ("server") => server ()?,
		Some (x) => bail! ("Unknown subcommand {}", x),
	}
	
	Ok (())
}

fn client () -> Result <()> {
	let socket = UdpSocket::bind ("0.0.0.0:9041").unwrap ();
	
	socket.join_multicast_v4 (&(Ipv4Addr::from_str ("225.100.99.98").unwrap ()), &([0u8, 0, 0, 0].into ())).unwrap ();
	
	socket.send_to ("hi there".as_bytes (), ("225.100.99.98", 9040)).unwrap ();
	
	Ok (())
}

fn server () -> Result <()> {
	let socket = UdpSocket::bind ("0.0.0.0:9040").unwrap ();
	
	socket.join_multicast_v4 (&(Ipv4Addr::from_str ("225.100.99.98").unwrap ()), &([0u8, 0, 0, 0].into ())).unwrap ();
	
	let mut buf = vec! [0u8; 4096];
	let (bytes_recved, remote_addr) = socket.recv_from (&mut buf).unwrap ();
	
	dbg! (bytes_recved, remote_addr);
	
	Ok (())
}
