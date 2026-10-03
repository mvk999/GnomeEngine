//! Small display-system-specific setup for the renderer surface.
//!
//! GNOME Shell 49+ classifies the renderer window on Wayland. The GNOME 46
//! X11 path instead relies on the public EWMH window properties below. They
//! must be installed after GDK creates the native surface and before GTK maps
//! it with `present()`.

use gtk::prelude::*;
use std::ffi::{c_char, c_int, c_uchar, c_ulong, c_void};

type XDisplay = c_void;
type XWindow = c_ulong;
type XAtom = c_ulong;
const FALSE: c_int = 0;
const PROP_MODE_REPLACE: c_int = 0;
const XA_ATOM: XAtom = 4;
const XA_CARDINAL: XAtom = 6;
const INPUT_HINT: c_ulong = 1;

#[repr(C)]
struct XWmHints {
    flags: c_ulong,
    input: c_int,
    initial_state: c_int,
    icon_pixmap: c_ulong,
    icon_window: c_ulong,
    icon_x: c_int,
    icon_y: c_int,
    icon_mask: c_ulong,
    window_group: c_ulong,
}

#[link(name = "X11")]
unsafe extern "C" {
    fn XInternAtom(display: *mut XDisplay, name: *const c_char, only_if_exists: c_int) -> XAtom;
    fn XChangeProperty(
        display: *mut XDisplay,
        window: XWindow,
        property: XAtom,
        type_: XAtom,
        format: c_int,
        mode: c_int,
        data: *const c_uchar,
        element_count: c_int,
    ) -> c_int;
    fn XFlush(display: *mut XDisplay) -> c_int;
    fn XGetWMHints(display: *mut XDisplay, window: XWindow) -> *mut XWmHints;
    fn XSetWMHints(display: *mut XDisplay, window: XWindow, hints: *mut XWmHints) -> c_int;
    fn XFree(data: *mut c_void) -> c_int;
}

pub fn configure_x11_desktop_surface(window: &gtk::Window) -> Result<(), String> {
    let (xdisplay, xid) = x11_handles(window)?;

    let desktop_type = intern_atom(xdisplay, b"_NET_WM_WINDOW_TYPE_DESKTOP\0")?;
    let window_type = intern_atom(xdisplay, b"_NET_WM_WINDOW_TYPE\0")?;
    change_atom_property(xdisplay, xid, window_type, &[desktop_type]);

    let sticky = intern_atom(xdisplay, b"_NET_WM_STATE_STICKY\0")?;
    let skip_taskbar = intern_atom(xdisplay, b"_NET_WM_STATE_SKIP_TASKBAR\0")?;
    let skip_pager = intern_atom(xdisplay, b"_NET_WM_STATE_SKIP_PAGER\0")?;
    let state = intern_atom(xdisplay, b"_NET_WM_STATE\0")?;
    change_atom_property(xdisplay, xid, state, &[sticky, skip_taskbar, skip_pager]);

    let desktop = intern_atom(xdisplay, b"_NET_WM_DESKTOP\0")?;
    change_cardinal_property(xdisplay, xid, desktop, &[u32::MAX as c_ulong]);

    // An empty input region handles pointer pass-through. Also tell the X11
    // window manager this surface never accepts keyboard focus.
    unsafe { set_no_input_focus_hint(xdisplay, xid)? };

    // The desktop type keeps the surface below normal application windows;
    // the renderer's empty GDK input region makes it click-through. No window
    // manager command, root-window drawing, or stacking poll is involved.
    unsafe { XFlush(xdisplay) };
    Ok(())
}

/// GTK refreshes WM_HINTS while mapping the window, so reassert only the
/// non-focusable policy immediately after `present()`.
pub fn set_x11_surface_non_focusable(window: &gtk::Window) -> Result<(), String> {
    let (xdisplay, xid) = x11_handles(window)?;
    unsafe {
        set_no_input_focus_hint(xdisplay, xid)?;
        XFlush(xdisplay);
    }
    Ok(())
}

fn x11_handles(window: &gtk::Window) -> Result<(*mut XDisplay, XWindow), String> {
    let display = gtk::prelude::WidgetExt::display(window)
        .downcast::<gdk4_x11::X11Display>()
        .map_err(|_| "the renderer surface is not using the X11 GDK backend".to_owned())?;
    let surface = window
        .surface()
        .ok_or_else(|| "GTK did not create an X11 surface for the renderer".to_owned())?
        .downcast::<gdk4_x11::X11Surface>()
        .map_err(|_| "the renderer surface is not a GDK X11 surface".to_owned())?;

    let xid = surface.xid();
    if xid == 0 {
        return Err("GDK returned an invalid X11 window ID".to_owned());
    }

    // GDK's X11 surface helpers and Xlib use the same display connection.
    // The pointer is borrowed from GDK and is valid for the lifetime of the
    // display object above.
    let xdisplay = unsafe {
        gdk4_x11_sys::gdk_x11_display_get_xdisplay(display.as_ptr().cast()) as *mut XDisplay
    };
    if xdisplay.is_null() {
        return Err("GDK did not expose its X11 display connection".to_owned());
    }
    Ok((xdisplay, xid))
}

unsafe fn set_no_input_focus_hint(display: *mut XDisplay, window: XWindow) -> Result<(), String> {
    let existing = unsafe { XGetWMHints(display, window) };
    let mut hints = if existing.is_null() {
        XWmHints {
            flags: 0,
            input: FALSE,
            initial_state: 0,
            icon_pixmap: 0,
            icon_window: 0,
            icon_x: 0,
            icon_y: 0,
            icon_mask: 0,
            window_group: 0,
        }
    } else {
        // XGetWMHints returns an Xlib-owned allocation; copy before updating.
        let hints = unsafe { std::ptr::read(existing) };
        unsafe { XFree(existing.cast()) };
        hints
    };
    hints.flags |= INPUT_HINT;
    hints.input = FALSE;
    if unsafe { XSetWMHints(display, window, &mut hints) } == 0 {
        return Err("X11 could not mark the desktop surface as non-focusable".to_owned());
    }
    Ok(())
}

fn intern_atom(display: *mut XDisplay, name: &'static [u8]) -> Result<XAtom, String> {
    let atom = unsafe { XInternAtom(display, name.as_ptr().cast(), FALSE) };
    if atom == 0 {
        Err(format!(
            "X11 did not provide the required EWMH atom {}",
            String::from_utf8_lossy(&name[..name.len() - 1])
        ))
    } else {
        Ok(atom)
    }
}

fn change_atom_property(display: *mut XDisplay, window: XWindow, property: XAtom, atoms: &[XAtom]) {
    unsafe {
        XChangeProperty(
            display,
            window,
            property,
            XA_ATOM,
            32,
            PROP_MODE_REPLACE,
            atoms.as_ptr().cast(),
            atoms.len() as i32,
        );
    }
}

fn change_cardinal_property(
    display: *mut XDisplay,
    window: XWindow,
    property: XAtom,
    values: &[c_ulong],
) {
    unsafe {
        XChangeProperty(
            display,
            window,
            property,
            XA_CARDINAL,
            32,
            PROP_MODE_REPLACE,
            values.as_ptr().cast(),
            values.len() as i32,
        );
    }
}
