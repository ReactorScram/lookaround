use thiserror::Error;

type Result <T> = std::result::Result <T, TlvError>;

#[derive (Debug, Error)]
pub enum TlvError {
	#[error ("Buffer too big")]
	BufferTooBig,
	#[error ("Caller-provided buffer too small")]
	CallerBufferTooSmall,
	#[error (transparent)]
	Io (#[from] std::io::Error),
	#[error (transparent)]
	TryFromInt (#[from] std::num::TryFromIntError),
}

struct Writer <W> {
	_x: std::marker::PhantomData <W>,
}

impl <W: std::io::Write> Writer <W> {
	fn length (w: &mut W, x: u32) -> Result <()> {
		w.write_all (&x.to_le_bytes ())?;
		Ok (())
	}
	
	fn lv_bytes (w: &mut W, b: &[u8]) -> Result <()> {
		if b.len () > 2_000_000_000 {
			Err (TlvError::BufferTooBig)?;
		}
		
		let l = u32::try_from (b.len ())?;
		
		Self::length (w, l)?;
		w.write_all (b)?;
		
		Ok (())
	}
}

struct Reader <R> {
	_x: std::marker::PhantomData <R>,
}

impl <R: std::io::Read> Reader <R> {
	fn length (r: &mut R) -> Result <u32> {
		let mut buf = [0; 4];
		r.read_exact (&mut buf)?;
		
		Ok (u32::from_le_bytes (buf))
	}
	
	fn lv_bytes (r: &mut R, buf: &mut [u8]) -> Result <u32> {
		let l = Self::length (r)?;
		if usize::try_from (l)? > buf.len () {
			Err (TlvError::CallerBufferTooSmall)?;
		}
		
		r.read_exact (&mut buf [0..usize::try_from (l)?])?;
		
		Ok (l)
	}
}

#[cfg (test)]
mod test {
	#[test]
	fn test_1 () {
		use std::io::Cursor;
		
		let b = "hi there".as_bytes ();
		
		let mut w = Cursor::new (Vec::default ());
		
		super::Writer::lv_bytes (&mut w, b).unwrap ();
		
		let v = w.into_inner ();
		
		assert_eq! (v, vec! [
			8, 0, 0, 0,
			104, 105, 32,
			116, 104, 101, 114, 101,
		]);
		
		let mut r = Cursor::new (v);
		
		let mut buf = vec! [0; 1024];
		
		let bytes_read = super::Reader::lv_bytes (&mut r, &mut buf).unwrap ();
		
		assert_eq! (usize::try_from (bytes_read).unwrap (), b.len ());
		assert_eq! (b, &buf [0..usize::try_from (bytes_read).unwrap ()]);
	}
}
