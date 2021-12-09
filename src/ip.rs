use std::{
	net::Ipv4Addr,
	process::Command,
	str::FromStr,
};

#[derive (Debug, thiserror::Error)]
pub enum IpError {
	#[error (transparent)]
	Io (#[from] std::io::Error),
	#[error (transparent)]
	FromUtf8 (#[from] std::string::FromUtf8Error),
	#[error ("Self-IP detection is not implemented on Mac OS")]
	NotImplementedOnMac,
}

#[cfg(target_os = "linux")]
pub fn get_ips () -> Result <Vec <Ipv4Addr>, IpError> {
	let output = linux::get_ip_addr_output ()?;
	
	Ok (linux::parse_ip_addr_output (&output))
}

#[cfg(target_os = "macos")]
pub fn get_ips () -> Result <Vec <Ipv4Addr>, IpError> {
	Err (IpError::NotImplementedOnMac)
}

#[cfg(target_os = "windows")]
pub fn get_ips () -> Result <Vec <Ipv4Addr>, IpError> {
	let output = windows::get_ip_config_output ()?;
	
	Ok (windows::parse_ip_config_output (&output))
}

#[cfg(target_os = "linux")]
pub mod linux {
	use super::*;
	
	pub fn get_ip_addr_output () -> Result <String, IpError> {
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
}

#[cfg(target_os = "windows")]
pub mod windows {
	use super::*;
	
	pub fn get_ip_config_output () -> Result <String, IpError> {
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

	#[cfg (test)]
	mod test {
		use super::*;
		
		#[test]
		fn test () {
			for (input, expected) in [
				(
					r"
	IPv4 Address .   .  .. . . . : 192.168.1.1
	",
					vec! [
						Ipv4Addr::new (192, 168, 1, 1),
					]
				),
			] {
				let actual = parse_ip_config_output (input);
				assert_eq! (actual, expected);
			}
		}
	}
}
