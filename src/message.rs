use std::{
	io::Cursor,
};

use crate::tlv;

use thiserror::Error;

const MAGIC_NUMBER: [u8; 4] = [0x9a, 0x4a, 0x43, 0x81];
pub const PACKET_SIZE: usize = 1024;

#[derive (Debug)]
pub enum Message {
	Request (Option <[u8; 6]>),
	Response (Option <[u8; 6]>),
}

#[derive (Debug, Error)]
pub enum MessageError {
	#[error (transparent)]
	Io (#[from] std::io::Error),
	#[error (transparent)]
	Tlv (#[from] tlv::TlvError),
	#[error ("Unknown type")]
	UnknownType,
}

impl Message {
	pub fn write <W: std::io::Write> (&self, w: &mut W) -> Result <(), std::io::Error> 
	{
		w.write_all (&MAGIC_NUMBER)?;
		
		match self {
			Self::Request (mac) => {
				w.write_all (&[1])?;
				Self::write_mac_opt (w, *mac)?;
			},
			Self::Response (mac) => {
				w.write_all (&[2])?;
				Self::write_mac_opt (w, *mac)?;
			},
		}
		
		Ok (())
	}
	
	fn write_mac_opt <W: std::io::Write> (w: &mut W, mac: Option <[u8; 6]>) -> Result <(), std::io::Error>
	{
		match mac {
			Some (mac) => {
				w.write_all (&[1])?;
				w.write_all (&mac[..])?;
			},
			None => w.write_all (&[0])?,
		}
		Ok (())
	}
	
	pub fn to_vec (&self) -> Result <Vec <u8>, tlv::TlvError> {
		let mut cursor = Cursor::new (Vec::with_capacity (PACKET_SIZE));
		self.write (&mut cursor)?;
		Ok (cursor.into_inner ())
	}
	
	pub fn read <R: std::io::Read> (r: &mut R) -> Result <Self, MessageError> {
		tlv::Reader::expect (r, &MAGIC_NUMBER)?;
		let t = tlv::Reader::u8 (r)?;
		
		Ok (match t {
			1 => {
				let mac = Self::read_mac_opt (r)?;
				Self::Request (mac)
			},
			2 => {
				let mac = Self::read_mac_opt (r)?;
				Self::Response (mac)
			},
			_ => return Err (MessageError::UnknownType),
		})
	}
	
	fn read_mac_opt <R: std::io::Read> (r: &mut R) 
	-> Result <Option <[u8; 6]>, std::io::Error> 
	{
		Ok (if tlv::Reader::u8 (r)? == 1 {
			let mut mac = [0u8; 6];
			r.read_exact (&mut mac)?;
			Some (mac)
		}
		else {
			None
		})
	}
	
	pub fn from_slice (buf: &[u8]) -> Result <Self, MessageError> {
		let mut cursor = Cursor::new (buf);
		Self::read (&mut cursor)
	}
}

#[cfg (test)]
mod test {
	#[test]
	fn test_1 () {
		
	}
}
