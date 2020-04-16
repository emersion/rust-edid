#[macro_use]
extern crate nom;

use nom::{be_u16, le_u8, le_u16, le_u32};

mod cp437;

#[derive(Debug, PartialEq, Copy, Clone)]
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

#[derive(Debug, PartialEq, Copy, Clone)]
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

named!(parse_descriptor_text<&[u8], String>,
	map!(
		map!(take!(13), |b| {
			b.iter()
			.filter(|c| **c != 0x0A)
			.map(|b| cp437::forward(*b))
			.collect::<String>()
		}),
		|s| s.trim().to_string()
	)
);

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct DetailedTiming {
	/// Pixel clock in kHz.
	pub pixel_clock: u32,
	pub horizontal_active_pixels: u16,
	pub horizontal_blanking_pixels: u16,
	pub vertical_active_lines: u16,
	pub vertical_blanking_lines: u16,
	pub horizontal_front_porch: u16,
	pub horizontal_sync_width: u16,
	pub vertical_front_porch: u16,
	pub vertical_sync_width: u16,
	/// Horizontal size in millimeters
	pub horizontal_size: u16,
	/// Vertical size in millimeters
	pub vertical_size: u16,
	/// Border pixels on one side of screen (i.e. total number is twice this)
	pub horizontal_border_pixels: u8,
	/// Border pixels on one side of screen (i.e. total number is twice this)
	pub vertical_border_pixels: u8,
	pub features: u8, /* TODO add enums etc. */
}

named!(parse_detailed_timing<&[u8], DetailedTiming>, do_parse!(
	pixel_clock_10khz: le_u16
	>> horizontal_active_lo: le_u8
	>> horizontal_blanking_lo: le_u8
	>> horizontal_px_hi: le_u8
	>> vertical_active_lo: le_u8
	>> vertical_blanking_lo: le_u8
	>> vertical_px_hi: le_u8
	>> horizontal_front_porch_lo: le_u8
	>> horizontal_sync_width_lo: le_u8
	>> vertical_lo: le_u8
	>> porch_sync_hi: le_u8
	>> horizontal_size_lo: le_u8
	>> vertical_size_lo: le_u8
	>> size_hi: le_u8
	>> horizontal_border: le_u8
	>> vertical_border: le_u8
	>> features: le_u8
	>> (DetailedTiming {
		pixel_clock: pixel_clock_10khz as u32 * 10,
		horizontal_active_pixels: (horizontal_active_lo as u16) |
		                          (((horizontal_px_hi >> 4) as u16) << 8),
		horizontal_blanking_pixels: (horizontal_blanking_lo as u16) |
		                            (((horizontal_px_hi & 0xf) as u16) << 8),
		vertical_active_lines: (vertical_active_lo as u16) |
		                       (((vertical_px_hi >> 4) as u16) << 8),
		vertical_blanking_lines: (vertical_blanking_lo as u16) |
		                         (((vertical_px_hi & 0xf) as u16) << 8),
		horizontal_front_porch: (horizontal_front_porch_lo as u16) |
		                        (((porch_sync_hi >> 6) as u16) << 8),
		horizontal_sync_width: (horizontal_sync_width_lo as u16) |
		                       ((((porch_sync_hi >> 4) & 0x3) as u16) << 8),
		vertical_front_porch: ((vertical_lo >> 4) as u16) |
		                      ((((porch_sync_hi >> 2) & 0x3) as u16) << 8),
		vertical_sync_width: ((vertical_lo & 0xf) as u16) |
		                     (((porch_sync_hi & 0x3) as u16) << 8),
		horizontal_size: (horizontal_size_lo as u16) | (((size_hi >> 4) as u16) << 8),
		vertical_size: (vertical_size_lo as u16) | (((size_hi & 0xf) as u16) << 8),
		horizontal_border_pixels: horizontal_border,
		vertical_border_pixels: vertical_border,
		features: features
	})
));

#[derive(Debug, PartialEq, Clone)]
pub enum Descriptor {
	DetailedTiming(DetailedTiming),
	SerialNumber(String),
	UnspecifiedText(String),
	RangeLimits, // TODO
	ProductName(String),
	WhitePoint, // TODO
	StandardTiming, // TODO
	ColorManagement,
	TimingCodes,
	EstablishedTimings,
	Dummy,
	Unknown([u8; 13]),
}

named!(parse_descriptor<&[u8], Descriptor>,
	switch!(peek!(le_u16),
		0 => do_parse!(
			take!(3)
			>> d: switch!(le_u8,
				0xFF => do_parse!(
					take!(1)
					>> s: parse_descriptor_text
					>> (Descriptor::SerialNumber(s))
				) |
				0xFE => do_parse!(
					take!(1)
					>> s: parse_descriptor_text
					>> (Descriptor::UnspecifiedText(s))
				) |
				0xFD => do_parse!(
					take!(1)
					>> take!(13)
					>> (Descriptor::RangeLimits)
				) |
				0xFC => do_parse!(
					take!(1)
					>> s: parse_descriptor_text
					>> (Descriptor::ProductName(s))
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
				0xF9 => do_parse!(
					take!(1)
					>> take!(13)
					>> (Descriptor::ColorManagement)
				) |
				0xF8 => do_parse!(
					take!(1)
					>> take!(13)
					>> (Descriptor::TimingCodes)
				) |
				0xF7 => do_parse!(
					take!(1)
					>> take!(13)
					>> (Descriptor::EstablishedTimings)
				) |
				0x10 => do_parse!(
					take!(1)
					>> take!(13)
					>> (Descriptor::Dummy)
				) |
				_ => do_parse!(
					take!(1)
					>> data: count_fixed!(u8, le_u8, 13)
					>> (Descriptor::Unknown(data))
				)
			)
			>> (d)
		) |
		_ => do_parse!(
			d: parse_detailed_timing
			>> (Descriptor::DetailedTiming(d))
		)
	)
);

#[derive(Debug, PartialEq, Clone)]
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
		match parse(d) {
			nom::IResult::Done(remaining, parsed) => {
				assert_eq!(remaining.len(), 0);
				assert_eq!(&parsed, expected);
			},
			nom::IResult::Error(err) => {
				panic!(format!("{}", err));
			},
			nom::IResult::Incomplete(_) => {
				panic!("Incomplete");
			},
		}
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
				Descriptor::DetailedTiming(DetailedTiming {
					pixel_clock: 146250,
					horizontal_active_pixels: 1680,
					horizontal_blanking_pixels: 560,
					vertical_active_lines: 1050,
					vertical_blanking_lines: 39,
					horizontal_front_porch: 104,
					horizontal_sync_width: 176,
					vertical_front_porch: 3,
					vertical_sync_width: 6,
					horizontal_size: 474,
					vertical_size: 296,
					horizontal_border_pixels: 0,
					vertical_border_pixels: 0,
					features: 28
				}),
				Descriptor::RangeLimits,
				Descriptor::ProductName("SyncMaster".to_string()),
				Descriptor::SerialNumber("HS3P701105".to_string()),
			),
		};

		test(d, &expected);
	}

	#[test]
	fn test_card0_edp_1() {
		let d = include_bytes!("../testdata/card0-eDP-1");

		let expected = EDID{
			header: Header{
				vendor: ['S', 'H', 'P'],
				product: 5193,
				serial: 0,
				week: 32,
				year: 25,
				version: 1,
				revision: 4,
			},
			display: Display{
				video_input: 165,
				width: 29,
				height: 17,
				gamma: 120,
				features: 14,
			},
			chromaticity: (),
			established_timing: (),
			standard_timing: (),
			descriptors: vec!(
				Descriptor::DetailedTiming(DetailedTiming {
					pixel_clock: 138500,
					horizontal_active_pixels: 1920,
					horizontal_blanking_pixels: 160,
					vertical_active_lines: 1080,
					vertical_blanking_lines: 31,
					horizontal_front_porch: 48,
					horizontal_sync_width: 32,
					vertical_front_porch: 3,
					vertical_sync_width: 5,
					horizontal_size: 294,
					vertical_size: 165,
					horizontal_border_pixels: 0,
					vertical_border_pixels: 0,
					features: 24,
				}),
				Descriptor::Dummy,
				Descriptor::UnspecifiedText("DJCP6ÇLQ133M1".to_string()),
				Descriptor::Unknown([2, 65, 3, 40, 0, 18, 0, 0, 11, 1, 10, 32, 32]),
			),
		};

		test(d, &expected);
	}
}
