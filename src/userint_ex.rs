#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
//==============================================================================
//
// Title:		UserInt Extender
// Purpose:		Safe wrappers for CVI User Interface functions
//
// Created on:	25.06.2025 at 08:57:26 by AD.
//
//==============================================================================

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

use crate::userint::*;

#[inline(always)]
pub fn init_cvi_rte(reserved: c_int, argv: *const *const c_char, reserved2: c_int) -> c_int {
	unsafe { InitCVIRTE(reserved, argv, reserved2) }
}

#[inline(always)]
pub fn quit_user_interface() -> c_int {
	unsafe {
		let ret = QuitUserInterface(0);
		println!("QuitUserInterface returned: {}", ret);
		let mut panel_data: i32 = 0;
		let mut control_data: i32 = 0;
		GetUserEvent(0, &mut panel_data as *mut i32, &mut control_data as *mut i32);
		//HidePanel(PANEL as i32);
		RunUserInterface();
		QuitUserInterface(0);
		return ret;
	}
}

#[inline(always)]
pub fn load_panel(reserved: c_int, uir_file: *const c_char, panel: u32) -> c_int {
	unsafe { LoadPanelAnsi(reserved, uir_file, panel as i32) }
}
//pub fn DisplayPanel(panel_handle: c_int) -> c_int;
//pub fn RunUserInterface() -> c_int;
//pub fn DiscardPanel(panel_handle: c_int) -> c_int;

#[inline(always)]
pub fn set_localized_decimal_symbol(enable: bool) {
	unsafe {
		if enable {
			SetSystemAttributeAnsi(ATTR_USE_LOCALIZED_DECIMAL_SYMBOL as i32, 1);
		} else {
			// SetSystemAttributeAnsi(ATTR_USE_LOCALIZED_DECIMAL_SYMBOL as i32, 1);
			// This is the default behavior, so we can skip setting it to 1
			// unless we want to explicitly disable localized decimal symbols.
			SetSystemAttributeAnsi(ATTR_USE_LOCALIZED_DECIMAL_SYMBOL as i32, 0);
		}
	}
}

#[inline(always)]
pub fn display_panel(panel_handle: c_int) -> c_int {
	unsafe { DisplayPanel(panel_handle) }
}

#[inline(always)]
pub fn run_user_interface() -> c_int {
	unsafe { RunUserInterface() }
}

#[inline(always)]
pub fn discard_panel(panel_handle: c_int) -> c_int {
	unsafe { DiscardPanel(panel_handle) }
}

#[inline(always)]
pub fn close_cvi_rte() {
	unsafe { CloseCVIRTE() }
}

#[inline(always)]
pub fn set_ctrl_val_str(panel: c_int, ctrl_id: u32, value: &str) {
	let c_str = std::ffi::CString::new(value).unwrap();
	unsafe { SetCtrlValUtf8(panel, ctrl_id as i32, c_str.as_ptr()) };
}

#[inline(always)]
pub fn set_ctrl_val_i32(panel: c_int, ctrl_id: u32, value: i32) {
	unsafe { SetCtrlValUtf8(panel, ctrl_id as i32, value) };
}

#[inline(always)]
pub fn set_ctrl_val_f64(panel: c_int, ctrl_id: u32, value: f64) {
	unsafe { SetCtrlValUtf8(panel, ctrl_id as i32, value) };
}

unsafe extern "C" {
	pub fn InitCVIRTE(reserved: c_int, argv: *const *const c_char, reserved2: c_int) -> c_int;
	pub fn LoadPanel(reserved: c_int, uir_file: *const c_char, panel: c_int) -> c_int;
	//pub fn DisplayPanel(panel_handle: c_int) -> c_int;
	//pub fn RunUserInterface() -> c_int;
	//pub fn DiscardPanel(panel_handle: c_int) -> c_int;
}

pub const EVENT_FILESDROPPED: i32 = 1005;

unsafe extern "C" {
	pub fn EnableDragAndDrop(panel: c_int) -> c_int;
}

pub fn enable_drag_and_drop(panel: i32) -> i32 {
	unsafe {
		//https://www.ni.com/docs/en-US/bundle/labwindows-cvi/page/toolslib/functionreference/cvienabledraganddrop.htm
		EnableDragAndDrop(panel);
	}
	0
}

unsafe extern "C" {
	pub fn GetDragAndDropDataAnsi(
		fileNames: *mut *mut *mut i8, // char *** in C
		mousePoint: *mut Point,
	) -> i32;
}

// Helper functions to get control values
pub fn get_numeric_value(panel: i32, control: u32) -> f64 {
	let mut value: f64 = 0.0;
	unsafe {
		GetCtrlValAnsi(panel, control as i32, &mut value as *mut f64 as *mut c_void);
	}
	value
}

pub fn get_string_value(panel: i32, control: u32) -> String {
	let mut buffer = vec![0u8; 256];
	unsafe {
		GetCtrlValUtf8(panel, control as i32, buffer.as_mut_ptr() as *mut c_void);
	}
	let cstr = CString::new(&buffer[..buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len())]).unwrap_or_default();
	cstr.to_string_lossy().into_owned()
}

pub fn get_numeric_value_i32(panel: i32, control: u32) -> i32 {
	let mut value: i32 = -1;
	unsafe {
		GetCtrlValAnsi(panel, control as i32, &mut value as *mut i32 as *mut c_void);
	}
	value
}

pub fn get_bool_value_i32(panel: i32, control: i32) -> bool {
	let mut value: bool = false;
	unsafe {
		GetCtrlValAnsi(panel, control as i32, &mut value as *mut bool as *mut c_void);
	}
	value
}

pub fn create_meta_font(
	name: *const ::std::os::raw::c_char,
	font_name: *const ::std::os::raw::c_char,
	size: i32,
	bold: i32,
	italic: i32,
	underline: i32,
	strikeout: i32,
) {
	unsafe {
		CreateMetaFontUtf8(
			name, font_name, size, bold, italic, underline, strikeout, // strikeout
		);
	}
}

pub fn set_ctrl_attribute_color(panel: i32, control: u32, attribute: u32, color: u32) {
	unsafe {
		SetCtrlAttributeAnsi(
			panel as i32,
			control as i32,
			attribute as i32,
			color as i32, //background_color as i32,
		);
	}
}

pub fn canvas_draw_text(
	pnl_handle: i32,
	pnl_canvas: u32,
	text: *const ::std::os::raw::c_char,
	metafont: *const ::std::os::raw::c_char,
	bounds: Rect,
	alignment: u32,
) {
	unsafe {
		CanvasDrawTextUtf8(pnl_handle, pnl_canvas as i32, text, metafont, bounds, alignment as i32);
	}
}

pub fn save_canvas_bitmap(pnl_handle: i32, pnl_canvas: u32, filename: &str) -> Result<(), String> {
	let mut bitmap: i32 = 0;
	let result = unsafe { GetCtrlBitmap(pnl_handle, pnl_canvas as i32, 0, &mut bitmap) };
	if result != 0 {
		return Err(format!("Failed to get bitmap handle: {}", result));
	}
	let c_filename = CString::new(filename).map_err(|e| e.to_string())?;
	let result = unsafe { SaveBitmapToPNGFileAnsi(bitmap, c_filename.as_ptr()) };
	if result != 0 {
		unsafe {
			DiscardBitmap(bitmap);
		}
		return Err(format!("Failed to save bitmap: {}", result));
	}
	let result = unsafe { DiscardBitmap(bitmap) };
	if result != 0 {
		return Err(format!("Failed to discard bitmap: {}", result));
	}
	Ok(())
}

#[inline]
pub fn set_ctrl_attribute_ptr<T>(panel: i32, control: i32, attribute: i32, value: &T) {
	unsafe {
		SetCtrlAttributeUtf8(panel, control, attribute, value as *const T as *const c_void);
	}
}

#[inline]
pub fn set_pattern<T>(panel: i32, control: u32, attribute: u32, value: &T) {
	unsafe {
		SetCtrlAttributeAnsi(panel as i32, control as i32, attribute as i32, value as *const T as *const c_void);
	}
}

#[inline]
pub fn set_pen_pattern<T>(panel: i32, control: u32, value: &T) {
	unsafe {
		SetCtrlAttributeAnsi(panel as i32, control as i32, ATTR_PEN_PATTERN as i32, value as *const T as *const c_void);
	}
} //https://www.ni.com/docs/de-DE/bundle/labwindows-cvi/page/cvi/uiref/cprogramming_with_canvas_controls.htm

#[inline]
pub fn set_attribute_u32(panel: i32, control: u32, attribute: u32, value: u32) {
	unsafe {
		SetCtrlAttributeUtf8(panel, control as i32, attribute as i32, value as i32);
	}
}

#[inline]
pub fn set_pen_color(panel: i32, control: u32, value: u32) {
	unsafe {
		SetCtrlAttributeUtf8(panel, control as i32, ATTR_PEN_COLOR as i32, value as i32);
	}
}

#[inline]
pub fn set_pen_fill_color(panel: i32, control: u32, value: u32) {
	unsafe {
		SetCtrlAttributeUtf8(panel, control as i32, ATTR_PEN_FILL_COLOR as i32, value as i32);
	}
}

#[inline]
pub fn set_pen_width(panel: i32, control: u32, value: u32) {
	unsafe {
		SetCtrlAttributeUtf8(panel, control as i32, ATTR_PEN_WIDTH as i32, value as i32);
	}
}

#[inline]
pub fn set_pen_mode(panel: i32, control: u32, value: u32) {
	unsafe {
		SetCtrlAttributeUtf8(panel, control as i32, ATTR_PEN_MODE as i32, value as i32);
	}
} //restore default https://www.ni.com/docs/de-DE/bundle/labwindows-cvi/page/cvi/uiref/cviattrpenmode.htm

#[inline]
pub fn canvas_draw_poly(panel: i32, control: u32, num_points: usize, points: *const Point, wrap: i32, draw_mode: u32) {
	unsafe {
		CanvasDrawPoly(panel, control as i32, num_points, points, wrap, draw_mode as i32);
	}
}

#[inline]
pub fn canvas_draw_line(panel: i32, control: u32, p1: Point, p2: Point) {
	unsafe {
		CanvasDrawLine(panel, control as i32, p1, p2);
	}
}

#[inline]
pub fn canvas_draw_rect(panel: i32, control: u32, rect: Rect, draw_mode: u32) {
	unsafe {
		CanvasDrawRect(panel, control as i32, rect, draw_mode as i32);
	}
}

#[inline]
pub fn canvas_clear(panel: i32, control: u32, rect: Rect) {
	unsafe {
		CanvasClear(panel, control as i32, rect);
	}
}

#[inline]
pub fn make_point_f64(x: f64, y: f64) -> Point {
	unsafe { MakePoint(x as i32, y as i32) }
}

#[inline]
pub fn make_point_i32(x: i32, y: i32) -> Point {
	unsafe { MakePoint(x, y) }
}

#[inline]
pub fn get_text_display_size(text: *const i8, font: *const i8, text_height: &mut i32, text_length: &mut i32) {
	unsafe {
		GetTextDisplaySizeUtf8(text, font, text_height, text_length);
	}
}

pub fn set_menu_bar_attribute(menu_bar: u32, menu_item: u32, attribute: u32, value: u32) {
	unsafe {
		SetMenuBarAttributeAnsi(menu_bar as i32, menu_item as i32, attribute as i32, value as i32);
	}
}

//==============================================================================
// From toolbox.h
unsafe extern "C" {
	fn ShowHtmlHelpUtf8(szFile: *const std::os::raw::c_char, uCommand: u32, dwData: *mut std::ffi::c_void);
}

pub fn show_html_help(file: &str, command: u32, data: *mut c_void) {
	let c_file = CString::new(file).expect("CString::new failed");
	unsafe {
		ShowHtmlHelpUtf8(c_file.as_ptr(), command, data);
	}
}
