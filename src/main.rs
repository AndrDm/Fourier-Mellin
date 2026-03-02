#![allow(unused_imports)]

mod fourier_mellin;
mod main_utils;
mod nivision;
mod userint;
mod userint_ex;

use crate::{fourier_mellin::*, main_utils::*, nivision::*, userint::*, userint_ex::*};

use std::ffi::{CString, c_char, c_double, c_void};
use std::os::raw::c_int;
use std::path::PathBuf;
use std::ptr;

static mut IMAGE_SRC: *mut Image = ptr::null_mut();
static mut IMAGE_DST: *mut Image = ptr::null_mut();
static mut IMAGE_REF: *mut Image = ptr::null_mut();


fn fourier_mellin(
	src: *const Image,
	reference: *const Image,
	dst: *mut Image,
) -> std::result::Result<ComputeResult, i32> {
	// Validate input image pointers before calling the NI Vision API.
	if src.is_null() || reference.is_null() || dst.is_null() {
		return Err(ERR_NOT_IMAGE);
	}

	// 1a.Cast input images (e.g., U16) to 32-bit float for frequency-domain processing.
	let src_f32 = imaq_create_image(ImageType_enum_IMAQ_IMAGE_SGL);
	let ref_f32 = imaq_create_image(ImageType_enum_IMAQ_IMAGE_SGL);
	imaq_cast_simple(src_f32, src, ImageType_enum_IMAQ_IMAGE_SGL);
	imaq_cast_simple(ref_f32, reference, ImageType_enum_IMAQ_IMAGE_SGL);

	// 1b.Compute the forward FFT of both images.
	let src_fft = imaq_create_image(ImageType_enum_IMAQ_IMAGE_COMPLEX);
	let ref_fft = imaq_create_image(ImageType_enum_IMAQ_IMAGE_COMPLEX);
	imaq_fft(src_fft, src_f32); //FFT
	imaq_fft(ref_fft, ref_f32);

	// 2.Attenuate high frequencies and shift the zero-frequency component to the center.
	imaq_attenuate(src_fft, src_fft, 1);
	imaq_attenuate(ref_fft, ref_fft, 1);
	imaq_flip_frequencies(src_fft, src_fft);
	imaq_flip_frequencies(ref_fft, ref_fft);

	// 3.Extract magnitude spectra from the complex Fourier images.
	let src_magnitude = imaq_create_image(ImageType_enum_IMAQ_IMAGE_SGL);
	let ref_magnitude = imaq_create_image(ImageType_enum_IMAQ_IMAGE_SGL);
	imaq_extract_complex_plane(src_magnitude, src_fft, ComplexPlane_enum_IMAQ_MAGNITUDE);
	imaq_extract_complex_plane(ref_magnitude, ref_fft, ComplexPlane_enum_IMAQ_MAGNITUDE);

	// 4.Transform magnitude spectra into log-polar coordinates (for scale and rotation estimation).
	let src_polar = imaq_create_image(ImageType_enum_IMAQ_IMAGE_SGL);
	let ref_polar = imaq_create_image(ImageType_enum_IMAQ_IMAGE_SGL);
	imaq_log_polar_transform(src_polar, src_magnitude);
	imaq_log_polar_transform(ref_polar, ref_magnitude);

	// 5.Estimate relative scale and rotation using phase correlation in the log-polar domain.
	let (max_x_scale, max_y_angle) = imaq_phase_correlate(src_polar, ref_polar);
	let (_width, height) = imaq_get_image_size(src);
	// Convert vertical/hor peak position to a rotation angle in degrees
	let angle = max_y_angle as f32 / (height as f32 / 360.0); // Fill height is 360 degrees
	let scale = scale_for_factor(max_x_scale as f32);
	println!("transform ({} (scale), {} (angle))", scale, angle);

	// 6.Apply the estimated rotation and scale to the source image.
	let dst_rotated = imaq_create_image(ImageType_enum_IMAQ_IMAGE_SGL);
	imaq_rotate_resample(dst_rotated, src_f32, angle, scale);

	// 7.Estimate the remaining translation using phase correlation in the spatial domain.
	let (max_x1, max_y1) = imaq_phase_correlate(dst_rotated, ref_f32);

	// 8.Apply the estimated translation and saturate out-of-bounds pixels.
	let shifted: *mut Image_struct = imaq_create_image(ImageType_enum_IMAQ_IMAGE_SGL);
	imaq_shift(shifted, dst_rotated, max_x1, max_y1, SATURATE);

	// 9.Copy the registered image into the output image.
	imaq_duplicate(dst, shifted);

	// Dispose of all temporary images to free NI Vision resources.
	imaq_dispose_image(src_f32);
	imaq_dispose_image(ref_f32);
	imaq_dispose_image(src_fft);
	imaq_dispose_image(ref_fft);
	imaq_dispose_image(src_magnitude);
	imaq_dispose_image(ref_magnitude);
	imaq_dispose_image(src_polar);
	imaq_dispose_image(ref_polar);
	imaq_dispose_image(dst_rotated);
	imaq_dispose_image(shifted);

	Ok(ComputeResult { rotation: angle as f32, scale: scale as f32, x_shift: max_x1 as f32, y_shift: max_y1 as f32 })
}

fn main() {
	println!("Welcome to Fourier-Mellin World!");

	let c_argv = build_c_argv();
	if !init_runtime(&c_argv) {
		eprintln!("Failed to initialize CVIRTE.");
		return;
	}

	unsafe {
		// Load panel
		let uir_file = CString::new("bin/fourier_mellin.uir").unwrap();
		let panel_handle = LoadPanel(0, uir_file.as_ptr(), PANEL);
		if panel_handle < 0 {
			eprintln!("Failed to load panel.");
			return;
		}

		IMAGE_SRC = imaqCreateImage(ImageType_enum_IMAQ_IMAGE_U16, 0);
		IMAGE_DST = imaqCreateImage(ImageType_enum_IMAQ_IMAGE_U16, 0);
		IMAGE_REF = imaqCreateImage(ImageType_enum_IMAQ_IMAGE_U16, 0);

		ImageControl_ConvertFromDecoration(panel_handle, PANEL_IMAGECTRL_REF, std::ptr::null_mut()); //important
		ImageControl_ConvertFromDecoration(panel_handle, PANEL_IMAGECTRL_ROT, std::ptr::null_mut()); //important
		ImageControl_ConvertFromDecoration(panel_handle, PANEL_IMAGECTRL_REG, std::ptr::null_mut());

		imaqSetWindowZoomToFit(0, 1);
		imaqSetWindowZoomToFit(1, 1);
		imaqSetWindowZoomToFit(2, 1);

		if let Some(first_png) = get_first_image_file(panel_handle, PANEL_LISTBOX) {
			load_and_show_image(IMAGE_REF, "img", first_png, panel_handle, PANEL_IMAGECTRL_REF, 0);
			load_and_show_image(IMAGE_SRC, "img", first_png, panel_handle, PANEL_IMAGECTRL_ROT, 1);
		}
		populate_listbox_with_image_files(panel_handle, PANEL_LISTBOX);

		// Display panel and run UI
		if DisplayPanel(panel_handle) < 0 {
			eprintln!("Failed to display panel.");
			DiscardPanel(panel_handle);
			return;
		}

		if RunUserInterface() < 0 {
			eprintln!("Failed to run user interface.");
			DiscardPanel(panel_handle);
			return;
		}

		DiscardPanel(panel_handle); // Clean up
	} // unsafe
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn listbox_cb(
	panel: c_int,
	_control: c_int,
	event: c_int,
	_callback_data: *mut c_void,
	_event_data1: c_int,
	_event_data2: c_int,
) -> c_int {
	unsafe {
		if event == EVENT_VAL_CHANGED as i32 {
			let selected_file = get_string_value(panel, PANEL_LISTBOX as u32);
			if !selected_file.is_empty() {
				let path_buf = PathBuf::from("img").join(selected_file);
				let path = path_buf.to_string_lossy().to_string();

				// --- FIX: convert Rust string-> C string ---
				let c_path = CString::new(path).expect("CString::new failed");

				// Pointer passed to C function
				let c_path_ptr: *const c_char = c_path.as_ptr();

				// Call NI Vision function
				let result = imaqReadFile(IMAGE_SRC, c_path_ptr, ptr::null_mut(), ptr::null_mut());

				ImageControl_SetAttribute(PANEL, PANEL_IMAGECTRL_ROT as i32, ATTR_IMAGECTRL_IMAGE as i32, IMAGE_SRC);
				imaqDisplayImage(IMAGE_SRC, 1, 0);
				imaqSetWindowZoomToFit(1, 1);

				let res = fourier_mellin(IMAGE_SRC, IMAGE_REF, IMAGE_DST).unwrap();
				let _ = display_image_fit(IMAGE_DST, PANEL, PANEL_IMAGECTRL_REG, 2);

				SetCtrlValUtf8(PANEL, PANEL_ROTATION, res.rotation as c_double);
				SetCtrlValUtf8(PANEL, PANEL_SCALE, res.scale as c_double);
				SetCtrlValUtf8(PANEL, PANEL_X_OFFSET, res.x_shift as c_int);
				SetCtrlValUtf8(PANEL, PANEL_Y_OFFSET, res.y_shift as c_int);

				if result != 0 {
					// handle NI Vision error if needed
				}
			}
		}

		0
	}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn quit_cb(
	_panel: c_int,
	_control: c_int,
	event: c_int,
	_callback_data: *mut c_void,
	_event_data1: c_int,
	_event_data2: c_int,
) -> c_int {
	unsafe {
		if event == EVENT_COMMIT {
			QuitUserInterface(0);
		}
	}
	0
}
