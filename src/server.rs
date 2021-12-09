use crate::prelude::*;

pub async fn server <I: Iterator <Item=String>> (mut args: I) -> Result <(), AppError> 
{
	let common_params = app_common::Params::default ();
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
	
	let socket = UdpSocket::bind (SocketAddrV4::new (Ipv4Addr::from_str (&bind_addr)?, common_params.server_port)).await?;
	
	socket.join_multicast_v4 (common_params.multicast_addr, [0u8, 0, 0, 0].into ())?;
	
	let mut recent_idem_ids = Vec::with_capacity (32);
	
	loop {
		println! ("Waiting for messages...");
		let (req_msgs, remote_addr) = match recv_msg_from (&socket).await {
			Ok (x) => x,
			Err (e) => {
				println! ("Error while receiving message: {:?}", e);
				continue;
			},
		};
		
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
			socket.send_to (&Message::many_to_vec (&resp)?, remote_addr).await?;
		}
	}
}
