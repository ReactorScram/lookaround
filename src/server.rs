use crate::prelude::*;

#[derive (Clone)]
struct Params {
	common: app_common::Params,
	bind_addrs: Vec <Ipv4Addr>,
	nickname: String,
	our_mac: Option <[u8; 6]>,
}

pub async fn server <I: Iterator <Item=String>> (args: I) -> Result <(), AppError> 
{
	let params = configure (args)?;
	
	// This was too hard to do in a functional style
	let mut tasks = vec! [];
	
	for bind_addr in &params.bind_addrs {
		let socket = match UdpSocket::bind (SocketAddrV4::new (Ipv4Addr::UNSPECIFIED, params.common.server_port)).await {
			Err (e) => {
				println! ("Error binding socket: {:?}", e);
				continue;
			},
			Ok (x) => x,
		};
		
		socket.join_multicast_v4 (params.common.multicast_addr, *bind_addr)?;
		
		dbg! (bind_addr);
		
		tasks.push (tokio::spawn (serve_interface (params.clone (), socket)));
	}
	
	for task in tasks {
		task.await??;
	}
	
	Ok (())
}

fn configure <I: Iterator <Item=String>> (mut args: I) -> Result <Params, AppError>
{
	let common = app_common::Params::default ();
	let mut bind_addrs = vec![];
	let mut nickname = String::new ();
	
	while let Some (arg) = args.next () {
		match arg.as_str () {
			"--bind-addr" => {
				bind_addrs.push (match args.next () {
					None => return Err (CliArgError::MissingArgumentValue (arg).into ()),
					Some (x) => Ipv4Addr::from_str (&x)?,
				});
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
	
	if bind_addrs.is_empty () {
		println! ("No bind addresses found, auto-detecting all local IPs");
		bind_addrs = get_ips ()?;
	}
	
	Ok (Params {
		common,
		bind_addrs,
		nickname,
		our_mac,
	})
}

async fn serve_interface (params: Params, socket: UdpSocket) 
-> Result <(), AppError>
{
	let mut recent_idem_ids = Vec::with_capacity (32);
	
	loop {
		println! ("Listening...");
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
					None
				}
				else {
					recent_idem_ids.insert (0, idem_id);
					recent_idem_ids.truncate (30);
					Some (vec! [
						Message::Response1 (params.our_mac),
						Message::Response2 (message::Response2 {
							idem_id,
							nickname: params.nickname.clone (),
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
