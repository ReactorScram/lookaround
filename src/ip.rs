use std::{
	net::Ipv4Addr,
	process::Command,
	str::FromStr,
};

use crate::AppError;

pub fn get_ip_addr_output () -> Result <String, AppError> {
	let output = Command::new ("ip")
	.arg ("addr")
	.output ()?;
	let output = output.stdout.as_slice ();
	let output = String::from_utf8 (output.to_vec ())?;
	Ok (output)
}

pub fn parse_ip_addr_output (output: &str) -> Vec <Ipv4Addr> {
	// I wrote this in FP style because I was bored.
	
	output.lines () 
	.map (|l| l.trim_start ())
	.filter_map (|l| l.strip_prefix ("inet "))
	.filter_map (|l| l.find ('/').map (|x| &l [0..x]))
	.filter_map (|l| Ipv4Addr::from_str (l).ok ())
	.collect ()
}

pub fn get_ip_config_output () -> Result <String, AppError> {
	let output = Command::new ("ipconfig")
	.output ()?;
	let output = output.stdout.as_slice ();
	let output = String::from_utf8 (output.to_vec ())?;
	Ok (output)
}

pub fn parse_ip_config_output (output: &str) -> Vec <Ipv4Addr> {
	let mut addrs = vec! [];
	
	for line in output.lines () {
		let line = line.trim_start ();
		
		// Maybe only works on English locales?
		if ! line.starts_with ("IPv4 Address") {
			continue;
		}
		let colon_pos = match line.find (':') {
			None => continue,
			Some (x) => x,
		};
		let line = &line [colon_pos + 2..];
		
		let addr = match Ipv4Addr::from_str (line) {
			Err (_) => continue,
			Ok (x) => x,
		};
		
		addrs.push (addr);
	}
	
	addrs
}
