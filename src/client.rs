use crate::prelude::*;

struct ServerResponse {
	mac: Option <[u8; 6]>,
	nickname: Option <String>,
}

struct ClientParams {
	common: app_common::Params,
	bind_addrs: Vec <Ipv4Addr>,
	timeout_ms: u64,
}

pub async fn client <I: Iterator <Item=String>> (args: I) -> Result <(), AppError> {
	match get_mac_address() {
		Ok(Some(ma)) => {
			println!("Our MAC addr = {}", ma);
		}
		Ok(None) => println!("No MAC address found."),
		Err(e) => println!("{:?}", e),
	}
	
	let params = configure_client (args)?;
	let socket = make_socket (&params).await?;
	let msg = Message::new_request1 ().to_vec ()?;
	tokio::spawn (send_requests (Arc::clone (&socket), params.common, msg));
	
	let mut peers = HashMap::with_capacity (10);
	
	timeout (Duration::from_millis (params.timeout_ms), listen_for_responses (&*socket, &mut peers)).await.ok ();
	
	let mut peers: Vec <_> = peers.into_iter ().collect ();
	peers.sort_by_key (|(_, v)| v.mac);
	
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

pub async fn find_nick <I: Iterator <Item=String>> (mut args: I) -> Result <(), AppError> 
{
	let mut nick = None;
	let mut timeout_ms = 500;
	
	while let Some (arg) = args.next () {
		match arg.as_str () {
			"--timeout-ms" => {
				timeout_ms = match args.next () {
					None => return Err (CliArgError::MissingArgumentValue (arg).into ()),
					Some (x) => u64::from_str (&x)?,
				};
			},
			_ => nick = Some (arg),
		}
	}
	
	let needle_nick = nick.ok_or_else (|| CliArgError::MissingRequiredArg ("nickname".to_string ()))?;
	let needle_nick = Some (needle_nick);
	
	let params = ClientParams {
		common: Default::default (),
		bind_addrs: get_ips ()?,
		timeout_ms,
	};
	
	let socket = make_socket (&params).await?;
	let msg = Message::new_request1 ().to_vec ()?;
	tokio::spawn (send_requests (Arc::clone (&socket), params.common, msg));
	
	timeout (Duration::from_millis (params.timeout_ms), async move { loop {
		let (msgs, remote_addr) = match recv_msg_from (&socket).await {
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
		
		if resp.nickname == needle_nick {
			println! ("{}", remote_addr.ip ());
			return;
		}
	}}).await?;
	
	Ok (())
}

fn configure_client <I: Iterator <Item=String>> (mut args: I) 
-> Result <ClientParams, AppError>
{
	let mut bind_addrs = vec! [];
	let mut timeout_ms = 500;
	
	while let Some (arg) = args.next () {
		match arg.as_str () {
			"--bind-addr" => {
				bind_addrs.push (match args.next () {
					None => return Err (CliArgError::MissingArgumentValue (arg).into ()),
					Some (x) => Ipv4Addr::from_str (&x)?,
				});
			},
			"--timeout-ms" => {
				timeout_ms = match args.next () {
					None => return Err (CliArgError::MissingArgumentValue (arg).into ()),
					Some (x) => u64::from_str (&x)?,
				};
			},
			_ => return Err (CliArgError::UnrecognizedArgument (arg).into ()),
		}
	}
	
	if bind_addrs.is_empty () {
		bind_addrs = get_ips ()?;
	}
	
	Ok (ClientParams {
		common: Default::default (),
		bind_addrs,
		timeout_ms,
	})
}

async fn make_socket (params: &ClientParams) -> Result <Arc <UdpSocket>, AppError> {
	let socket = UdpSocket::bind (SocketAddrV4::new (Ipv4Addr::UNSPECIFIED, 0)).await?;
	
	for bind_addr in &params.bind_addrs {
		if let Err (e) = socket.join_multicast_v4 (params.common.multicast_addr, *bind_addr) {
			println! ("Error joining multicast group with iface {}: {:?}", bind_addr, e);
		}
	}
	
	Ok (Arc::new (socket))
}

async fn send_requests (
	socket: Arc <UdpSocket>, 
	params: app_common::Params,
	msg: Vec <u8>,
) 
-> Result <(), AppError> 
{
	for _ in 0..10 {
		socket.send_to (&msg, (params.multicast_addr, params.server_port)).await?;
		sleep (Duration::from_millis (100)).await;
	}
	
	Ok::<_, AppError> (())
}

async fn listen_for_responses (
	socket: &UdpSocket, 
	peers: &mut HashMap <SocketAddr, ServerResponse>
) {
	loop {
		let (msgs, remote_addr) = match recv_msg_from (socket).await {
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
}
