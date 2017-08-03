#[macro_use]
extern crate nom;

use nom::{le_u8, le_u16, le_u32};

#[derive(Debug, PartialEq)]
struct Header {
	vendor: u16,
	product: u16,
	serial: u32,
	version: u8,
	revision: u8,
}

named!(header<&[u8], Header>, do_parse!(
	tag!(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00][..])
	>> vendor: le_u16
	>> product: le_u16
	>> serial: le_u32
	>> le_u8 // week
	>> le_u8 // year
	>> version: le_u8
	>> revision: le_u8
	>> (Header{vendor, product, serial, version, revision})
));

#[derive(Debug, PartialEq)]
struct Display {
	video_input: u8,
	width: u8, // cm
	height: u8, // cm
	gamma: u8, // datavalue = (gamma*100)-100 (range 1.00–3.54)
	features: u8,
}

named!(display<&[u8], Display>, do_parse!(
	video_input: le_u8
	>> width: le_u8
	>> height: le_u8
	>> gamma: le_u8
	>> features: le_u8
	>> (Display{video_input, width, height, gamma, features})
));

named!(edid<&[u8], (Header, Display)>, tuple!(header, display));

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_host() {
		let d = include_bytes!("/sys/devices/pci0000:00/0000:00:02.0/drm/card0/card0-VGA-1/edid");
		let r = edid(d).unwrap();
		println!("{:?}", r);
	}
}
