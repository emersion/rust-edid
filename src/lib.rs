#[macro_use]
extern crate nom;

use std::str;
use nom::{be_u16, le_u8, le_u16, le_u32};

#[derive(Debug, PartialEq)]
pub struct Header {
	pub vendor: [char; 3],
	pub product: u16,
	pub serial: u32,
	pub week: u8,
	pub year: u8, // Starting at year 1990
	pub version: u8,
	pub revision: u8,
}

fn parse_vendor(v: u16) -> [char; 3] {
	let mask: u8 = 0x1F; // Each letter is 5 bits
	let i0 = ('A' as u8) - 1; // 0x01 = A
	return [
		(((v >> 10) as u8 & mask) + i0) as char,
		(((v >> 5) as u8 & mask) + i0) as char,
		(((v >> 0) as u8 & mask) + i0) as char,
	]
}

named!(parse_header<&[u8], Header>, do_parse!(
	tag!(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00][..])
	>> vendor: be_u16
	>> product: le_u16
	>> serial: le_u32
	>> week: le_u8
	>> year: le_u8
	>> version: le_u8
	>> revision: le_u8
	>> (Header{vendor: parse_vendor(vendor), product, serial, week, year, version, revision})
));

#[derive(Debug, PartialEq)]
pub struct Display {
	pub video_input: u8,
	pub width: u8, // cm
	pub height: u8, // cm
	pub gamma: u8, // datavalue = (gamma*100)-100 (range 1.00–3.54)
	pub features: u8,
}

named!(parse_display<&[u8], Display>, do_parse!(
	video_input: le_u8
	>> width: le_u8
	>> height: le_u8
	>> gamma: le_u8
	>> features: le_u8
	>> (Display{video_input, width, height, gamma, features})
));

named!(parse_chromaticity<&[u8], ()>, do_parse!(
	take!(10) >> ()
));

named!(parse_established_timing<&[u8], ()>, do_parse!(
	take!(3) >> ()
));

named!(parse_standard_timing<&[u8], ()>, do_parse!(
	take!(16) >> ()
));

// TODO: code page 437 (https://en.wikipedia.org/wiki/Code_page_437)
named!(parse_descriptor_text<&[u8], &str>, map!(map_res!(take!(13), str::from_utf8), |s| s.trim()));

#[derive(Debug, PartialEq)]
pub enum Descriptor {
	DetailedTiming, // TODO
	SerialNumber(String),
	UnspecifiedText(String),
	RangeLimits, // TODO
	Name(String),
	WhitePoint, // TODO
	StandardTiming, // TODO
	Unknown([u8; 13]),
}

named!(parse_descriptor<&[u8], Descriptor>,
	switch!(le_u16,
		0 => do_parse!(
			take!(1)
			>> d: switch!(le_u8,
				0xFF => do_parse!(
					take!(1)
					>> s: parse_descriptor_text
					>> (Descriptor::SerialNumber(s.to_string()))
				) |
				0xFE => do_parse!(
					take!(1)
					>> s: parse_descriptor_text
					>> (Descriptor::UnspecifiedText(s.to_string()))
				) |
				0xFD => do_parse!(
					take!(1)
					>> take!(13)
					>> (Descriptor::RangeLimits)
				) |
				0xFC => do_parse!(
					take!(1)
					>> s: parse_descriptor_text
					>> (Descriptor::Name(s.to_string()))
				) |
				0xFB => do_parse!(
					take!(1)
					>> take!(13)
					>> (Descriptor::WhitePoint)
				) |
				0xFA => do_parse!(
					take!(1)
					>> take!(13)
					>> (Descriptor::StandardTiming)
				) |
				_ => do_parse!(
					take!(1) >>
					data: count_fixed!(u8, le_u8, 13) >>
					(Descriptor::Unknown(data))
				)
			)
			>> (d)
		) |
		_ => do_parse!(take!(16) >> (Descriptor::DetailedTiming))
	)
);

#[derive(Debug, PartialEq)]
pub struct EDID {
	pub header: Header,
	pub display: Display,
	chromaticity: (), // TODO
	established_timing: (), // TODO
	standard_timing: (), // TODO
	pub descriptors: Vec<Descriptor>,
}

named!(parse_edid<&[u8], EDID>, do_parse!(
	header: parse_header
	>> display: parse_display
	>> chromaticity: parse_chromaticity
	>> established_timing: parse_established_timing
	>> standard_timing: parse_standard_timing
	>> descriptors: count!(parse_descriptor, 4)
	>> take!(1) // number of extensions
	>> take!(1) // checksum
	>> (EDID{header, display, chromaticity, established_timing, standard_timing, descriptors})
));

pub fn parse(data: &[u8]) -> nom::IResult<&[u8], EDID> {
	parse_edid(data)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn test(d: &[u8], expected: &EDID) {
		let (remaining, parsed) = parse(d).unwrap();
		assert_eq!(remaining.len(), 0);
		assert_eq!(&parsed, expected);
	}

	#[test]
	fn test_card0_vga_1() {
		let d = include_bytes!("../testdata/card0-VGA-1");

		let expected = EDID{
			header: Header{
				vendor: ['S', 'A', 'M'],
				product: 596,
				serial: 1146106418,
				week: 27,
				year: 17,
				version: 1,
				revision: 3,
			},
			display: Display{
				video_input: 14,
				width: 47,
				height: 30,
				gamma: 120,
				features: 42,
			},
			chromaticity: (),
			established_timing: (),
			standard_timing: (),
			descriptors: vec!(
				Descriptor::DetailedTiming,
				Descriptor::RangeLimits,
				Descriptor::Name("SyncMaster".to_string()),
				Descriptor::SerialNumber("HS3P701105".to_string()),
			),
		};

		test(d, &expected);
	}
}
