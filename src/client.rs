use crate::prelude::*;

struct ServerResponse {
	mac: Option <[u8; 6]>,
	nickname: Option <String>,
}

struct ConfigFile {
	nicknames: HashMap <String, String>,
}

struct ClientParams {
	common: app_common::Params,
	bind_addrs: Vec <Ipv4Addr>,
	nicknames: HashMap <String, String>,
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
	let socket = make_socket (&params.common, params.bind_addrs).await?;
	let msg = Message::new_request1 ().to_vec ()?;
	tokio::spawn (send_requests (Arc::clone (&socket), params.common, msg));
	
	let mut peers = HashMap::with_capacity (10);
	
	timeout (Duration::from_millis (params.timeout_ms), listen_for_responses (&*socket, params.nicknames, &mut peers)).await.ok ();
	
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
	let ConfigFile {
		nicknames,
	} = load_config_file ();
	
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
	
	let common_params = Default::default ();
	
	let socket = make_socket (&common_params, get_ips ()?).await?;
	let msg = Message::new_request1 ().to_vec ()?;
	tokio::spawn (send_requests (Arc::clone (&socket), common_params, msg));
	
	timeout (Duration::from_millis (timeout_ms), async move { loop {
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
		
		resp.nickname = get_peer_nickname (&nicknames, resp.mac, resp.nickname);
		
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
	
	let ConfigFile {
		nicknames,
	} = load_config_file ();
	
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
		nicknames,
		timeout_ms,
	})
}

fn load_config_file () -> ConfigFile {
	let mut nicknames: HashMap <String, String> = Default::default ();
	
	if let Some (proj_dirs) = find_project_dirs () {
		let mut ini = Ini::new_cs ();
		let path = proj_dirs.config_dir ().join ("client.ini");
		if ini.load (&path).is_ok () {
			let map_ref = ini.get_map_ref ();
			if let Some (x) = map_ref.get ("nicknames") {
				for (k, v) in x {
					if let Some (v) = v {
						let k = k.replace ('-', ":");
						nicknames.insert (k, v.to_string ());
					}
				}
			}
		}
	}
	
	ConfigFile {
		nicknames,
	}
}

async fn make_socket (
	common_params: &app_common::Params,
	bind_addrs: Vec <Ipv4Addr>,
) -> Result <Arc <UdpSocket>, AppError> {
	let socket = UdpSocket::bind (SocketAddrV4::new (Ipv4Addr::UNSPECIFIED, 0)).await?;
	
	for bind_addr in &bind_addrs {
		if let Err (e) = socket.join_multicast_v4 (common_params.multicast_addr, *bind_addr) {
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
	nicknames: HashMap <String, String>,
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
		
		resp.nickname = get_peer_nickname (&nicknames, resp.mac, resp.nickname);
		
		peers.insert (remote_addr, resp);
	}
}

fn get_peer_nickname (
	nicknames: &HashMap <String, String>,
	mac: Option <[u8; 6]>,
	peer_nickname: Option <String>
) -> Option <String>
{
	match peer_nickname.as_ref ().map (String::as_str) {
		None => (),
		Some ("") => (),
		_ => return peer_nickname,
	}
	
	if let Some (mac) = &mac {
		return nicknames.get (&format! ("{}", MacAddress::new (*mac))).cloned ()
	}
	
	None
}

#[cfg (test)]
mod test {
	use super::*;
	
	#[test]
	fn test_nicknames () {
		let mut nicks = HashMap::new ();
		
		for (k, v) in [
			("01:01:01:01:01:01", "phoenix")
		] {
			nicks.insert (k.to_string (), v.to_string ());
		}
		
		for (num, (mac, peer_nickname), expected) in [
			// Somehow the server returns no MAC nor nick. In this case we are helpless
			( 1, (None, None), None),
			// If the server tells us its MAC, we can look up our nickname for it
			( 2, (Some ([1, 1, 1, 1, 1, 1]), None), Some ("phoenix")),
			// Unless it's not in our nick list.
			( 3, (Some ([1, 1, 1, 1, 1, 2]), None), None),
			// If the server tells us its nickname, that always takes priority
			( 4, (None, Some ("snowflake")), Some ("snowflake")),
			( 5, (Some ([1, 1, 1, 1, 1, 1]), Some ("snowflake")), Some ("snowflake")),
			( 6, (Some ([1, 1, 1, 1, 1, 2]), Some ("snowflake")), Some ("snowflake")),
			// But blank nicknames are treated like None
			( 7, (None, Some ("")), None),
			( 8, (Some ([1, 1, 1, 1, 1, 1]), Some ("")), Some ("phoenix")),
			( 9, (Some ([1, 1, 1, 1, 1, 2]), Some ("")), None),
		] {
			let actual = get_peer_nickname (&nicks, mac, peer_nickname.map (str::to_string));
			assert_eq! (actual.as_ref ().map (String::as_str), expected, "{}", num);
		}
	}
}
