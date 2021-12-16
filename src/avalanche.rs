type Mac = [u8; 6];

pub fn debug () {
	for input in [
		[0, 0, 0, 0, 0, 0],
		[0, 0, 0, 0, 0, 1],
		[1, 0, 0, 0, 0, 0],
		[1, 0, 0, 0, 0, 1],
	] {
		assert_eq! (unmix (mix (input)), input);
	}
	
	println! ("Passed");
}

// NOT intended for any cryptography or security. This is TRIVIALLY reversible.
// It's just to make it easier for humans to tell apart MACs where only a couple
// numbers differ.

fn mix (i: Mac) -> Mac {
	[
		i [0] ^ i [5],
		i [1] ^ i [4],
		i [2] ^ i [3],
		i [3],
		i [4],
		i [5],
	]
}

fn unmix (i: Mac) -> Mac {
	[
		i [0] ^ i [5],
		i [1] ^ i [4],
		i [2] ^ i [3],
		i [3],
		i [4],
		i [5],
	]
}
