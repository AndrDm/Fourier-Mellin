#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]

use crate::nivision::*;
use crate::userint::*;
use crate::userint_ex::*;
use std::slice;

use std::{
	ffi::CString,
	fs,
	os::raw::{c_char, c_int, c_void},
	path::{Path, PathBuf},
	ptr::{self, null_mut},
};

use std::f32::consts::PI;

#[allow(dead_code)]
pub struct ComputeResult {
	pub rotation: f32,
	pub scale: f32,
	pub x_shift: f32,
	pub y_shift: f32,
}

pub const SATURATE_U16: PixelValue_union = PixelValue_union { grayscale: 65535.0_f32 };
pub const SATURATE: PixelValue_union = PixelValue_union { grayscale: 65535.0_f32 / 2.0 };
/// Math Helper functions
///
// Bilinear interpolation helper
pub fn bilinear(src: &[f32], rows: usize, cols: usize, x: f32, y: f32) -> f32 {
	let x0 = x.floor() as isize;
	let y0 = y.floor() as isize;
	let x1 = x0 + 1;
	let y1 = y0 + 1;

	// Out of bounds → return 0
	if x0 < 0 || y0 < 0 || x1 >= cols as isize || y1 >= rows as isize {
		return 0.0;
	}

	let dx = x - x0 as f32;
	let dy = y - y0 as f32;

	// Helper closure to index row-major image
	let idx = |xx: isize, yy: isize| -> f32 { src[(yy as usize) * cols + (xx as usize)] };

	let v00 = idx(x0, y0);
	let v01 = idx(x0, y1);
	let v10 = idx(x1, y0);
	let v11 = idx(x1, y1);

	let vx0 = v00 + dx * (v10 - v00);
	let vx1 = v01 + dx * (v11 - v01);

	vx0 + dy * (vx1 - vx0)
}

//------------------------------------------------------------------------------

pub fn build_hipass_kernel(rows: usize, cols: usize) -> Vec<f32> {
	let res_ht = 1.0f32 / (rows as f32 - 1.0);
	let res_wd = 1.0f32 / (cols as f32 - 1.0);

	// eta: length rows
	let eta: Vec<f32> = (0..rows)
		.map(|i| {
			let x = -0.5 + res_ht * (i as f32);
			(PI * x).cos()
		})
		.collect();

	// neta: length cols
	let neta: Vec<f32> = (0..cols)
		.map(|j| {
			let x = -0.5 + res_wd * (j as f32);
			(PI * x).cos()
		})
		.collect();

	// X = outer product eta' * neta
	// H = (1 - X) * (2 - X)
	let mut kernel = vec![0.0f32; rows * cols];

	for r in 0..rows {
		for c in 0..cols {
			let x = eta[r] * neta[c];
			let h = (1.0 - x) * (2.0 - x);
			kernel[r * cols + c] = h;
		}
	}

	kernel
}

pub fn apply_highpass(src: &[f32], rows: usize, cols: usize) -> Vec<f32> {
	let kernel = build_hipass_kernel(rows, cols);

	let mut dst = vec![0.0f32; rows * cols];

	for i in 0..rows * cols {
		dst[i] = src[i] * kernel[i];
	}

	dst
}

//------------------------------------------------------------------------------

/// Apply a 2D Hann window to a (rows × cols) float image.
///
/// `src` and `dst` are row‑major flat buffers of length rows*cols.
/// Safe for `src == dst` (in-place).
pub fn apply_hann_2d(src: &[f32], dst: &mut [f32], rows: usize, cols: usize) {
	if rows < 2 || cols < 2 || src.len() != rows * cols || dst.len() != rows * cols {
		return;
	}

	// Precompute 1D Hann windows for rows and cols
	let mut w_rows = vec![0.0f32; rows];
	let mut w_cols = vec![0.0f32; cols];

	let denom_r = (rows - 1) as f32;
	let denom_c = (cols - 1) as f32;

	for r in 0..rows {
		let x = r as f32 / denom_r;
		w_rows[r] = 0.5 * (1.0 - (2.0 * PI * x).cos());
	}

	for c in 0..cols {
		let x = c as f32 / denom_c;
		w_cols[c] = 0.5 * (1.0 - (2.0 * PI * x).cos());
	}

	// Apply separable 2D Hann = w_rows[i] * w_cols[j]
	for i in 0..rows {
		let wr = w_rows[i];
		let row_off = i * cols;

		for j in 0..cols {
			dst[row_off + j] = src[row_off + j] * wr * w_cols[j];
		}
	}
}

//------------------------------------------------------------------------------

// Log-polar transform

/// Perform a log‑polar transform on a (rows × cols) image.
/// `src` and `dst` are row-major buffers of length rows*cols.
/// The output grid is also (rows × cols).
///
/// NOTE:
/// - Angular direction = horizontal (j = cols)
/// - Radial direction  = vertical   (i = rows)
pub fn log_polar_transform(src: &[f32], dst: &mut [f32], rows: usize, cols: usize) {
	//
	// 1) High‑pass filtering
	//
	let tmp_hp = apply_highpass(src, rows, cols);

	//
	// 2) Apply Hann window (in-place allowed)
	//
	// We must create a second buffer for Hann window output.
	let mut tmp_hann = vec![0.0f32; rows * cols];
	apply_hann_2d(&tmp_hp, &mut tmp_hann, rows, cols);

	// From here on, use `tmp_hann` as the new `src`
	let src = &tmp_hann;

	//
	// ----- Begin original log‑polar code -----
	//

	// Image center
	let cx = (cols as f32 - 1.0) * 0.5;
	let cy = (rows as f32 - 1.0) * 0.5;

	// Max radius = distance to farthest corner
	let max_radius = ((cx * cx) + (cy * cy)).sqrt(); // Euclidean distance (hypot)
	let log_max_r = (max_radius + 1.0).ln();

	for j in 0..cols {
		// theta = angle for this column
		let theta = j as f32 * (2.0 * PI / cols as f32);

		for i in 0..rows {
			// log-radius mapping (vertical direction)
			let rho = (max_radius * (i as f32) / (rows as f32 - 1.0) + 1.0).ln();

			// convert log radius back to normal radius
			let r = max_radius * rho.exp() / log_max_r.exp();

			// Convert to Cartesian coords
			let x = cx + r * theta.cos();
			let y = cy + r * theta.sin();

			// Bilinear sampling
			dst[j * rows + i] = bilinear(src, rows, cols, x, y);
		}
	}
}

/// Returns a scale for a given factor (expected in [-6, 6]) using
/// piecewise-linear interpolation over measured points.
/// Unknown factors are linearly interpolated; out-of-range values are clamped.
pub fn scale_for_factor(factor: f32) -> f32 {
	// Measured (factor, scale) pairs you provided (duplicates removed).
	// NOTE: order matters (must be sorted by factor).
	// WARNING: Hard-coded values for 1024 height only, consider to make it generic
	const POINTS: &[(f32, f32)] = &[
		(-6.0, 1.11),
		(-3.0, 1.06),
		(0.0, 1.00),
		(1.0, 0.99),
		(3.0, 0.91),
		(4.0, 0.92), // note: slight bump here as measured
		(6.0, 0.88),
	];

	// Handle degenerate cases
	if POINTS.is_empty() {
		return 1.0;
	}
	if POINTS.len() == 1 {
		return POINTS[0].1;
	}

	// Clamp to endpoints
	if factor <= POINTS.first().unwrap().0 {
		return POINTS.first().unwrap().1;
	}
	if factor >= POINTS.last().unwrap().0 {
		return POINTS.last().unwrap().1;
	}

	// Find the interval [x0, x1] such that x0 <= factor <= x1
	// Since POINTS is small, a simple linear scan is fine.
	for w in POINTS.windows(2) {
		let (x0, y0) = w[0];
		let (x1, y1) = w[1];
		if factor >= x0 && factor <= x1 {
			if (x1 - x0).abs() < f32::EPSILON {
				// Shouldn't happen with the sorted unique points, but safe-guard anyway
				return y0;
			}
			let t = (factor - x0) / (x1 - x0);
			return y0 + t * (y1 - y0);
		}
	}

	// Fallback (should be unreachable due to clamps)
	POINTS.last().unwrap().1
}

/// Helper: load an image from disk and display it
pub unsafe fn load_and_show_image(
	image_handle: *mut Image,
	folder: &str,
	filename: &str,
	panel_handle: i32,
	ctrl: i32,
	window_index: i32,
) {
	unsafe {
		// Build full path
		let path = PathBuf::from(folder).join(filename).to_string_lossy().to_string();

		// Convert to C string
		let c_path = CString::new(path).expect("CString::new failed");
		let c_path_ptr: *const c_char = c_path.as_ptr();

		// Load image
		let result = imaqReadFile(image_handle, c_path_ptr, ptr::null_mut(), ptr::null_mut());
		if result == 0 {
			eprintln!("imaqReadFile failed with error {}", result);
		}

		// Bind image to image control
		ImageControl_SetAttribute(panel_handle, ctrl, ATTR_IMAGECTRL_IMAGE as i32, image_handle);

		// Display + zoom
		imaqDisplayImage(image_handle, window_index, 0);
		imaqSetWindowZoomToFit(window_index, 1);
	}
}

pub fn populate_listbox_with_image_files(panel_handle: i32, list_box_ctrl: i32) {
	let images_path = Path::new("img");

	if let Ok(entries) = fs::read_dir(images_path) {
		unsafe {
			ClearListCtrl(panel_handle as i32, list_box_ctrl as i32);
		}
		for (index, entry) in entries.flatten().enumerate() {
			let path = entry.path();
			if let Some(ext) = path.extension() {
				if ext == "png" {
					if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
						let c_string = CString::new(file_name).unwrap();
						unsafe {
							InsertListItemAnsi(
								panel_handle,
								list_box_ctrl as i32,
								index as i32,
								c_string.as_ptr(),
								c_string.as_ptr(),
							);
						}
					}
				}
			}
		}
	} else {
		eprintln!("Could not read Images directory.");
	}
}

pub fn get_first_image_file(panel_handle: i32, list_box_ctrl: i32) -> Option<&'static str> {
	let images_path = Path::new("img");

	if let Ok(entries) = fs::read_dir(images_path) {
		// Find first PNG filename
		for entry in entries.flatten() {
			let path = entry.path();
			if let Some(ext) = path.extension() {
				if ext == "png" {
					if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
						// Optionally populate listbox here too
						unsafe {
							ClearListCtrl(panel_handle as i32, list_box_ctrl as i32);
						}
						let c_string = CString::new(file_name).unwrap();
						unsafe {
							InsertListItemAnsi(
								panel_handle,
								list_box_ctrl as i32,
								0, // index 0 for first
								c_string.as_ptr(),
								c_string.as_ptr(),
							);
						}
						return Some(Box::leak(file_name.to_string().into_boxed_str())); // Leak for 'static
					}
				}
			}
		}
	} else {
		eprintln!("Could not read Images directory.");
	}
	None
}

pub fn build_c_argv() -> Vec<*const c_char> {
	std::env::args().map(|arg| CString::new(arg).unwrap()).map(|cstr| cstr.into_raw() as *const c_char).collect()
}

pub fn init_runtime(c_argv: &[*const c_char]) -> bool {
	init_cvi_rte(0, c_argv.as_ptr(), 0) != 0
}

//------------------------------------------------------------------------------
// Safe wrappers

pub fn imaq_create_image(type_: ImageType) -> *mut Image {
	unsafe { imaqCreateImage(type_, 0) }
}

pub fn imaq_create_image_border(type_: ImageType, border_size: i32) -> *mut Image {
	unsafe { imaqCreateImage(type_, border_size) }
}

/*
pub fn imaq_create_image(type_: ImageType, border_size: i32 = 0) -> *mut Image {
	unsafe { imaqCreateImage(type_, border_size) }
}
*/

pub fn imaq_get_image_size1(
	image: *const Image,
	width: *mut ::std::os::raw::c_int,
	height: *mut ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
	unsafe { imaqGetImageSize(image, width, height) }
}

pub fn imaq_get_image_size2(image: *const Image) -> Result<(i32, i32), i32> {
	let mut width: i32 = 0;
	let mut height: i32 = 0;

	let result = unsafe { imaqGetImageSize(image, &mut width, &mut height) };

	if result == 0 { Ok((width, height)) } else { Err(result) }
}

pub fn imaq_get_image_size(image: *const Image) -> (i32, i32) {
	let mut width: i32 = 0;
	let mut height: i32 = 0;

	let _ = unsafe { imaqGetImageSize(image, &mut width, &mut height) };

	(width, height)
}

pub fn imaq_set_image_size(
	image: *mut Image,
	width: ::std::os::raw::c_int,
	height: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
	unsafe { imaqSetImageSize(image, width, height) }
}

pub fn imaq_cast_simple(dest: *mut Image, source: *const Image, type_: ImageType) -> ::std::os::raw::c_int {
	let null = ptr::null_mut::<f32>();
	unsafe { imaqCast(dest, source, type_, null, -1) } //no lookup, no shift
}
/*
pub fn imaq_ippi_win_hamming_32f(dest: *mut Image, source: *const Image) -> ::std::os::raw::c_int {
	unsafe {
		let mut info: ImageInfo_struct = std::mem::zeroed();

		let (width, height) = imaq_get_image_size(source);
		imaq_set_image_size(dest, width, height);

		imaqGetImageInfo(source, &mut info);
		let src_step = info.pixelsPerLine * 4; // f32 -> 4 bytes per pixel
		let p_src = info.imageStart as *const Ipp32f;

		imaqGetImageInfo(dest, &mut info);
		let src_hann_step = info.pixelsPerLine * 4; // f32 -> 4 bytes per pixel
		let p_src_hann = info.imageStart as *mut Ipp32f;

		let roi = IppiSize { width: width, height: height };
		let mut p_size: i32 = 0;
		ippiWinHammingGetBufferSize(IppDataType_ipp32f, roi, &mut p_size);
		let mut p_buffer: Vec<u8> = vec![0u8; p_size as usize];
		println!("pSize is {}", p_size);
		ippiWinHamming_32f_C1R(p_src, src_step, p_src_hann, src_hann_step, roi, p_buffer.as_mut_ptr());
		0
	}
}
*/
pub fn imaq_fft(dest: *mut Image, source: *const Image) -> ::std::os::raw::c_int {
	unsafe { imaqFFT(dest, source) }
}

pub fn imaq_flip_frequencies(dest: *mut Image, source: *const Image) -> ::std::os::raw::c_int {
	unsafe { imaqFlipFrequencies(dest, source) }
}

pub fn imaq_attenuate(dest: *mut Image, source: *const Image, highlow: AttenuateMode) -> ::std::os::raw::c_int {
	unsafe { imaqAttenuate(dest, source, highlow) }
}

pub fn imaq_extract_complex_plane(
	dest: *mut Image,
	source: *const Image,
	plane: ComplexPlane,
) -> ::std::os::raw::c_int {
	unsafe { imaqExtractComplexPlane(dest, source, plane) }
}

pub fn imaq_log_polar_transform(dest: *mut Image, source: *const Image) -> ::std::os::raw::c_int {
	unsafe {
		let (width, height) = imaq_get_image_size(source);
		imaq_set_image_size(dest, width, height);

		let (mut cols, mut rows) = (0, 0);

		let arr = imaqImageToArray(source, imaqMakeRect(0, 0, i32::MAX, i32::MAX), &mut cols, &mut rows);

		if arr.is_null() {
			panic!("imaqImageToArray returned null");
		}

		let pixel_count = (cols * rows) as usize;
		let raw = slice::from_raw_parts(arr as *const f32, pixel_count);

		let n = cols as usize;
		let mut dst = vec![0.0f32; n * n];
		log_polar_transform(raw, &mut dst, rows as usize, cols as usize);

		imaqArrayToImage(dest, dst.as_ptr() as *const c_void, n as i32, n as i32);
		imaqDispose(arr as *mut c_void)
	}
}

pub fn imaq_phase_correlate(source1: *mut Image, source2: *const Image) -> (i32, i32) {
	unsafe {
		//Cross Corr
		let (width, height) = imaq_get_image_size(source1);
		let (mut cols, mut rows) = (width, height);

		let src_fft = imaq_create_image(ImageType_enum_IMAQ_IMAGE_COMPLEX);
		let ref_fft = imaq_create_image(ImageType_enum_IMAQ_IMAGE_COMPLEX);

		imaqFFT(src_fft, source1);
		imaqFFT(ref_fft, source2);

		imaqConjugate(src_fft, src_fft);
		imaqMultiply(src_fft, src_fft, ref_fft);
		let src_inv = imaqCreateImage(ImageType_enum_IMAQ_IMAGE_SGL, 0);
		imaqInverseFFT(src_inv, src_fft);

		let arr = imaqImageToArray(src_inv, imaqMakeRect(0, 0, i32::MAX, i32::MAX), &mut cols, &mut rows);

		if arr.is_null() {
			panic!("imaqImageToArray returned null");
		}

		let pixel_count = (cols * rows) as usize;

		let src = slice::from_raw_parts(arr as *const f32, pixel_count);

		let (mut max_val1, mut max_x1, mut max_y1) = (0f32, 0, 0);

		for (i, &v) in src.iter().enumerate() {
			if v > max_val1 {
				max_val1 = v;
				max_x1 = (i as i32) % cols;
				max_y1 = (i as i32) / cols;
			}
		}

		// Apply wrap-around correction:
		// If the coordinate is larger than half the dimension, subtract full dimension
		if max_x1 > (cols / 2) {
			max_x1 -= cols;
		}

		if max_y1 > (rows / 2) {
			max_y1 -= rows;
		}
		imaqDispose(src_fft as *mut c_void);
		imaqDispose(ref_fft as *mut c_void);
		imaqDispose(src_inv as *mut c_void);
		imaqDispose(arr as *mut c_void);

		println!("Phase correlate max = {} at (x={}, y={})", max_val1, max_x1, max_y1);
		(max_x1, max_y1)
	}
}

pub fn imaq_dispose_image(object: *mut Image) -> ::std::os::raw::c_int {
	unsafe { imaqDispose(object as *mut c_void) }
}

pub fn imaq_rotate(
	dest: *mut Image,
	source: *const Image,
	angle: f32,
	fill: PixelValue,
	method: InterpolationMethod,
) -> ::std::os::raw::c_int {
	unsafe { imaqRotate(dest, source, angle, fill, method) }
}

pub fn imaq_resample(
	dest: *mut Image,
	source: *const Image,
	newWidth: ::std::os::raw::c_int,
	newHeight: ::std::os::raw::c_int,
	method: InterpolationMethod,
	rect: Rect_struct, //rect: nivision::Rect,
) -> ::std::os::raw::c_int {
	unsafe { imaqResample(dest, source, newWidth, newHeight, method, rect) }
}

pub fn imaq_shift(
	dest: *mut Image,
	source: *const Image,
	shift_x: ::std::os::raw::c_int,
	shift_y: ::std::os::raw::c_int,
	fill: PixelValue,
) -> ::std::os::raw::c_int {
	unsafe { imaqShift(dest, source, shift_x, shift_y, fill) }
}

pub fn imaq_display_image(
	image: *const Image,
	windowNumber: ::std::os::raw::c_int,
	resize: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
	unsafe { imaqDisplayImage(image, windowNumber, resize) }
}

pub fn imaq_image_to_image(
	largeImage: *mut Image,
	smallImage: *const Image,
	dest: *mut Image,
	offset: *const Rect_struct,
	mask: *const Image,
	keepOverlays: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
	unsafe { imaqImageToImage(largeImage, smallImage, dest, offset, mask, keepOverlays) }
}

pub fn imaq_fill_image(image: *mut Image, value: PixelValue, mask: *const Image) -> ::std::os::raw::c_int {
	unsafe { imaqFillImage(image, value, mask) }
}

pub fn display_image_fit(image: *const Image, panel: i32, control: i32, window_id: i32) -> Result<(), i32> {
	let fit: i32 = 1;
	unsafe {
		ImageControl_SetAttribute(panel, control, ATTR_IMAGECTRL_IMAGE, image);
		imaq_display_image(image, window_id, 0);
		imaqSetWindowZoomToFit(window_id, fit);
	}
	Ok(())
}

pub fn imaq_duplicate(dest: *mut Image, source: *const Image) -> ::std::os::raw::c_int {
	unsafe { imaqDuplicate(dest, source) }
}

pub fn imaq_rotate_resample(dest: *mut Image, source: *const Image, angle: f32, scale: f32) -> ::std::os::raw::c_int {
	let (width, height) = imaq_get_image_size(source);

	let fill = PixelValue_union {
        	grayscale: 16371.0_f32, // grayscale = saturation (sorry for hard-coded)
    	};

	imaq_rotate(dest, source, -angle, fill, InterpolationMethod_enum_IMAQ_BILINEAR);

	let out_w: i32 = ((width as f32) * scale).round() as i32;
	let out_h: i32 = ((height as f32) * scale).round() as i32;
	fn full_image_rect(width: i32, height: i32) -> Rect_struct {
		Rect_struct { top: 0, left: 0, height, width }
	}
	let rec = full_image_rect(width, height);
	imaq_resample(dest, dest, out_w, out_h, InterpolationMethod_enum_IMAQ_BILINEAR, rec);

	let to_be_shifted = imaq_create_image(ImageType_enum_IMAQ_IMAGE_SGL);
	imaq_set_image_size(to_be_shifted, width, height);
	let fill = PixelValue_union {
        	grayscale: 16371.0_f32, // grayscale = saturation (sorry for hard-coded)
    	};
	imaq_fill_image(to_be_shifted, fill, null_mut());

	imaq_image_to_image(to_be_shifted, dest, to_be_shifted, &rec, null_mut(), 0);
	imaq_duplicate(dest, to_be_shifted);
	imaq_dispose_image(to_be_shifted);
	0 // we are optimists
}
