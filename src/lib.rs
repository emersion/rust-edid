#[macro_use]
extern crate nom;

use std::str;
use nom::{le_u8, le_u16, le_u32};

#[derive(Debug)]
struct Header {
	vendor: u16,
	product: u16,
	serial: u32,
	week: u8,
	year: u8,
	version: u8,
	revision: u8,
}

named!(header<&[u8], Header>, do_parse!(
	tag!(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00][..])
	>> vendor: le_u16
	>> product: le_u16
	>> serial: le_u32
	>> week: le_u8
	>> year: le_u8
	>> version: le_u8
	>> revision: le_u8
	>> (Header{vendor, product, serial, week, year, version, revision})
));

#[derive(Debug)]
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

named!(chromaticity<&[u8], ()>, do_parse!(
	take!(10) >> ()
));

named!(established_timing<&[u8], ()>, do_parse!(
	take!(3) >> ()
));

named!(standard_timing<&[u8], ()>, do_parse!(
	take!(16) >> ()
));

// TODO: code page 437 (https://en.wikipedia.org/wiki/Code_page_437)
named!(descriptor_text<&[u8], &str>, map!(map_res!(take!(13), str::from_utf8), |s| s.trim()));

#[derive(Debug)]
enum Descriptor {
	DetailedTiming, // TODO
	SerialNumber(String),
	UnspecifiedText, // TODO
	RangeLimits, // TODO
	Name, // TODO
	WhitePoint, // TODO
	StandardTiming, // TODO
	Unknown,
}

named!(descriptor<&[u8], Descriptor>,
	switch!(le_u16,
		0 => do_parse!(
			take!(1)
			>> d: switch!(le_u8,
				0xFF => do_parse!(take!(1) >> s: descriptor_text >> (Descriptor::SerialNumber(s.to_string()))) |
				0xFE => do_parse!(take!(1) >> take!(13) >> (Descriptor::UnspecifiedText)) |
				0xFD => do_parse!(take!(1) >> take!(13) >> (Descriptor::RangeLimits)) |
				0xFC => do_parse!(take!(1) >> take!(13) >> (Descriptor::Name)) |
				0xFB => do_parse!(take!(1) >> take!(13) >> (Descriptor::WhitePoint)) |
				0xFA => do_parse!(take!(1) >> take!(13) >> (Descriptor::StandardTiming)) |
				_ => do_parse!(take!(1) >> take!(13) >> (Descriptor::Unknown))
			)
			>> (d)
		) |
		_ => do_parse!(take!(16) >> (Descriptor::DetailedTiming))
	)
);

#[derive(Debug)]
struct EDID {
	header: Header,
	display: Display,
	descriptors: Vec<Descriptor>,
}

named!(edid<&[u8], EDID>, do_parse!(
	header: header
	>> display: display
	>> chromaticity
	>> established_timing
	>> standard_timing
	>> descriptors: count!(descriptor, 4)
	>> take!(1) // number of extensions
	>> take!(1) // checksum
	>> (EDID{header, display, descriptors})
));

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_host() {
		//let d = include_bytes!("/sys/devices/pci0000:00/0000:00:02.0/drm/card0/card0-LVDS-1/edid");
		let d = include_bytes!("/sys/devices/pci0000:00/0000:00:02.0/drm/card0/card0-VGA-1/edid");
		println!("{} {:?}", d.len(), &d[..]);
		let (remaining, result) = edid(d).unwrap();
		println!("{} {:?}", remaining.len(), remaining);
		println!("{} {:?}", d.len() - remaining.len(), result);
	}
}
