use std::{
	io::{
		Cursor,
		Write,
	},
};

use crate::tlv;

use thiserror::Error;

const MAGIC_NUMBER: [u8; 4] = [0x9a, 0x4a, 0x43, 0x81];
pub const PACKET_SIZE: usize = 1024;

type Mac = [u8; 6];

#[derive (Debug, PartialEq)]
pub enum Message {
	// 1
	Request1 {
		idem_id: [u8; 8],
		mac: Option <Mac>
	},
	// 2
	Response1 (Option <Mac>),
	// 3
	Response2 (Response2),
}

#[derive (Debug, PartialEq)]
pub struct Response2 {
	pub idem_id: [u8; 8],
	pub nickname: String,
}

#[derive (Debug, Error)]
pub enum MessageError {
	#[error (transparent)]
	Io (#[from] std::io::Error),
	#[error ("Length prefix too long")]
	LengthPrefixTooLong ((usize, usize)),
	#[error (transparent)]
	Tlv (#[from] tlv::TlvError),
	#[error (transparent)]
	TryFromInt (#[from] std::num::TryFromIntError),
	#[error ("Unknown type")]
	UnknownType,
	#[error (transparent)]
	FromUtf8 (#[from] std::string::FromUtf8Error),
}

#[derive (Default)]
struct DummyWriter {
	position: usize,
}

impl Write for DummyWriter {
	fn flush (&mut self) -> std::io::Result <()> {
		Ok (())
	}
	
	fn write (&mut self, buf: &[u8]) -> std::io::Result <usize> {
		self.position += buf.len ();
		Ok (buf.len ())
	}
}

impl Message {
	pub fn write <T> (&self, w: &mut Cursor <T>) -> Result <(), MessageError> 
	where Cursor <T>: Write
	{
		match self {
			Self::Request1 {
				idem_id,
				mac,
			}=> {
				w.write_all (&[1])?;
				w.write_all (&idem_id[..])?;
				Self::write_mac_opt (w, *mac)?;
			},
			Self::Response1 (mac) => {
				w.write_all (&[2])?;
				Self::write_mac_opt (w, *mac)?;
			},
			Self::Response2 (x) => {
				w.write_all (&[3])?;
				// Measure length with dummy writes
				// This is dumb, I'm just messing around to see if I can do
				// this without allocating.
				let mut dummy_writer = DummyWriter::default ();
				
				Self::write_response_2 (&mut dummy_writer, x)?;
				
				// Write length and real params to real output
				let len = u32::try_from (dummy_writer.position).unwrap ();
				w.write_all (&len.to_le_bytes ())?;
				Self::write_response_2 (w, x)?;
			},
		}
		
		Ok (())
	}
	
	fn write_response_2 <W: Write> (w: &mut W, params: &Response2) 
	-> Result <(), MessageError>
	{
		w.write_all (&params.idem_id)?;
		let nickname = params.nickname.as_bytes ();
		tlv::Writer::<_>::lv_bytes (w, nickname)?;
		Ok (())
	}
	
	fn write_mac_opt <W: Write> (w: &mut W, mac: Option <[u8; 6]>) -> Result <(), std::io::Error>
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
	
	pub fn to_vec (&self) -> Result <Vec <u8>, MessageError> {
		let mut cursor = Cursor::new (Vec::with_capacity (PACKET_SIZE));
		cursor.write_all (&MAGIC_NUMBER)?;
		self.write (&mut cursor)?;
		Ok (cursor.into_inner ())
	}
	
	pub fn many_to_vec (msgs: &[Self]) -> Result <Vec <u8>, MessageError> {
		let mut cursor = Cursor::new (Vec::with_capacity (PACKET_SIZE));
		cursor.write_all (&MAGIC_NUMBER)?;
		for msg in msgs {
			msg.write (&mut cursor)?;
		}
		Ok (cursor.into_inner ())
	}
	
	fn read2 <R: std::io::Read> (r: &mut R) -> Result <Self, MessageError> {
		let t = tlv::Reader::u8 (r)?;
		
		Ok (match t {
			1 => {
				let mut idem_id = [0u8; 8];
				r.read_exact (&mut idem_id)?;
				
				let mac = Self::read_mac_opt (r)?;
				Self::Request1 {
					idem_id,
					mac,
				}
			},
			2 => {
				let mac = Self::read_mac_opt (r)?;
				Self::Response1 (mac)
			},
			3 => {
				tlv::Reader::<_>::length (r)?;
				
				let mut idem_id = [0; 8];
				r.read_exact (&mut idem_id)?;
				
				let nickname = tlv::Reader::<_>::lv_bytes_to_vec (r, 64)?;
				let nickname = String::from_utf8 (nickname)?;
				
				Self::Response2 (Response2 {
					idem_id,
					nickname,
				})
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
	
	pub fn from_slice2 (buf: &[u8]) -> Result <Vec <Self>, MessageError> {
		let mut cursor = Cursor::new (buf);
		tlv::Reader::expect (&mut cursor, &MAGIC_NUMBER)?;
		
		let mut msgs = Vec::with_capacity (2);
		
		while cursor.position () < u64::try_from (buf.len ())? {
			let msg = Self::read2 (&mut cursor)?;
			msgs.push (msg);
		}
		Ok (msgs)
	}
}

#[cfg (test)]
mod test {
	use super::*;
	
	#[test]
	fn test_write_2 () {
		for (input, expected) in [
			(
				vec! [
					Message::Request1 {
						idem_id: [1, 2, 3, 4, 5, 6, 7, 8,],
						mac: None,
					},
				],
				vec! [
					154, 74, 67, 129,
					// Request tag
					1,
					// Idem ID
					1, 2, 3, 4, 5, 6, 7, 8,
					// MAC is None
					0,
				],
			),
			(
				vec! [
					Message::Response1 (Some ([0x11, 0x22, 0x33, 0x44, 0x55, 0x66])),
					Message::Response2 (Response2 {
						idem_id: [1, 2, 3, 4, 5, 6, 7, 8,],
						nickname: ":V".to_string (),
					}),
				],
				vec! [
					// Magic number for LookAround packets
					154, 74, 67, 129,
					// Response1 tag
					2, 
					// MAC is Some
					1,
					// MAC
					17, 34, 51, 68, 85, 102,
					// Response2 tag
					3,
					// Length prefix
					14, 0, 0, 0,
					// Idem ID
					1, 2, 3, 4, 5, 6, 7, 8,
					// Length-prefixed string
					2, 0, 0, 0,
					58, 86,
				],
			),
		] { 
			let actual = Message::many_to_vec (&input).unwrap ();
			assert_eq! (actual, expected, "{:?}", input);
		}
	}
	
	#[test]
	fn test_write_1 () {
		for (input, expected) in [
			(
				Message::Request1 {
					idem_id: [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
					mac: None,
				},
				vec! [
					154, 74, 67, 129,
					// Request tag
					1,
					// Idem ID
					1, 2, 3, 4, 5, 6, 7, 8,
					// MAC is None
					0,
				],
			),
			(
				Message::Response1 (Some ([0x11, 0x22, 0x33, 0x44, 0x55, 0x66])), 
				vec! [
					// Magic number for LookAround packets
					154, 74, 67, 129,
					// Response tag
					2, 
					// MAC is Some
					1,
					// MAC
					17, 34, 51, 68, 85, 102,
				],
			),
			(
				Message::Response1 (None), 
				vec! [
					// Magic number for LookAround packets
					154, 74, 67, 129,
					// Response tag
					2, 
					// MAC is None
					0,
				],
			),
		].into_iter () {
			let actual = input.to_vec ().unwrap ();
			assert_eq! (actual, expected, "{:?}", input);
		}
	}
	
	#[test]
	fn test_read_2 () {
		for input in [
			vec! [
				Message::Request1 {
					idem_id: [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
					mac: None,
				},
			],
			vec! [
				Message::Response1 (Some ([0x11, 0x22, 0x33, 0x44, 0x55, 0x66])),
			],
			vec! [
				Message::Response1 (None),
			],
			vec! [
				Message::Response1 (Some ([0x11, 0x22, 0x33, 0x44, 0x55, 0x66])),
				Message::Response2 (Response2 {
					idem_id: [1, 2, 3, 4, 5, 6, 7, 8,],
					nickname: ":V".to_string (),
				}),
			],
		].into_iter () {
			let encoded = Message::many_to_vec (&input).unwrap ();
			let decoded = Message::from_slice2 (&encoded).unwrap ();
			assert_eq! (input, decoded);
		}
	}
}
