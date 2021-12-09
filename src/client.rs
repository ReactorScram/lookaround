use crate::prelude::*;

struct ServerResponse {
	mac: Option <[u8; 6]>,
	nickname: Option <String>,
}

pub async fn client <I : Iterator <Item=String>> (mut args: I) -> Result <(), AppError> {
	use rand::RngCore;
	
	let common_params = app_common::Params::default ();
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
	
	let socket = UdpSocket::bind (&format! ("{}:0", bind_addr)).await?;
	
	socket.join_multicast_v4 (common_params.multicast_addr, Ipv4Addr::from_str (&bind_addr)?)?;
	
	let mut idem_id = [0u8; 8];
	rand::thread_rng ().fill_bytes (&mut idem_id);
	
	let msg = Message::Request1 {
		idem_id,
		mac: None,
	}.to_vec ()?;
	
	for _ in 0..10 {
		socket.send_to (&msg, (common_params.multicast_addr, common_params.server_port)).await?;
		sleep (Duration::from_millis (100)).await;
	}
	
	let mut peers = HashMap::with_capacity (10);
	
	timeout (Duration::from_secs (2), listen_for_responses (&socket, &mut peers)).await.ok ();
	
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
