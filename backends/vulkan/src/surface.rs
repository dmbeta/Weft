//! Vulkan surfaces from `raw-window-handle` handles, which `ash` leaves to the application. This
//! is what `ash-window` does, minus its `cocoa` 0.25 path (unmaintained, and `block` there is
//! rejected by a future Rust); Apple goes through `raw-window-metal` 1.x, built on `objc2`.

use std::ffi::c_char;

use ash::prelude::VkResult;
use ash::{Entry, Instance, ext, khr, vk};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

#[cfg(target_os = "macos")]
use std::{ffi::c_void, ptr::NonNull};

/// The instance extensions a surface for `display` needs, in the order Vulkan wants them.
pub fn required_extensions(display: RawDisplayHandle) -> VkResult<&'static [*const c_char]> {
    const WINDOWS: [*const c_char; 2] = [khr::surface::NAME.as_ptr(), khr::win32_surface::NAME.as_ptr()];
    const WAYLAND: [*const c_char; 2] = [khr::surface::NAME.as_ptr(), khr::wayland_surface::NAME.as_ptr()];
    const XLIB: [*const c_char; 2] = [khr::surface::NAME.as_ptr(), khr::xlib_surface::NAME.as_ptr()];
    const XCB: [*const c_char; 2] = [khr::surface::NAME.as_ptr(), khr::xcb_surface::NAME.as_ptr()];
    const METAL: [*const c_char; 2] = [khr::surface::NAME.as_ptr(), ext::metal_surface::NAME.as_ptr()];
    match display {
        RawDisplayHandle::Windows(_) => Ok(&WINDOWS),
        RawDisplayHandle::Wayland(_) => Ok(&WAYLAND),
        RawDisplayHandle::Xlib(_) => Ok(&XLIB),
        RawDisplayHandle::Xcb(_) => Ok(&XCB),
        RawDisplayHandle::AppKit(_) | RawDisplayHandle::UiKit(_) => Ok(&METAL),
        _ => Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT),
    }
}

/// # Safety
///
/// Both handles must belong to a live window and display connection that outlive the surface, and
/// `instance` must have been made with `required_extensions` of that display.
pub unsafe fn create(
    entry: &Entry,
    instance: &Instance,
    display: RawDisplayHandle,
    window: RawWindowHandle,
) -> VkResult<vk::SurfaceKHR> {
    unsafe {
        match (display, window) {
            (RawDisplayHandle::Windows(_), RawWindowHandle::Win32(w)) => {
                let info = vk::Win32SurfaceCreateInfoKHR::default()
                    .hwnd(w.hwnd.get())
                    .hinstance(w.hinstance.ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)?.get());
                khr::win32_surface::Instance::new(entry, instance).create_win32_surface(&info, None)
            }
            (RawDisplayHandle::Wayland(d), RawWindowHandle::Wayland(w)) => {
                let info =
                    vk::WaylandSurfaceCreateInfoKHR::default().display(d.display.as_ptr()).surface(w.surface.as_ptr());
                khr::wayland_surface::Instance::new(entry, instance).create_wayland_surface(&info, None)
            }
            (RawDisplayHandle::Xlib(d), RawWindowHandle::Xlib(w)) => {
                let info = vk::XlibSurfaceCreateInfoKHR::default()
                    .dpy(d.display.ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)?.as_ptr())
                    .window(w.window);
                khr::xlib_surface::Instance::new(entry, instance).create_xlib_surface(&info, None)
            }
            (RawDisplayHandle::Xcb(d), RawWindowHandle::Xcb(w)) => {
                let info = vk::XcbSurfaceCreateInfoKHR::default()
                    .connection(d.connection.ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)?.as_ptr())
                    .window(w.window.get());
                khr::xcb_surface::Instance::new(entry, instance).create_xcb_surface(&info, None)
            }
            #[cfg(target_os = "macos")]
            (RawDisplayHandle::AppKit(_), RawWindowHandle::AppKit(w)) => {
                let info = vk::MetalSurfaceCreateInfoEXT::default().layer(metal_layer(w.ns_view));
                ext::metal_surface::Instance::new(entry, instance).create_metal_surface(&info, None)
            }
            _ => Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT),
        }
    }
}

/// The view's `CAMetalLayer`, made now (as a sublayer) if it has none.
///
/// The layer is never released: the surface lives as long as the game does, and MoltenVK uses the
/// layer for as long as its swapchain does.
#[cfg(target_os = "macos")]
fn metal_layer(view: NonNull<c_void>) -> *const c_void {
    let layer = unsafe { raw_window_metal::Layer::from_ns_view(view) };
    layer.into_raw().as_ptr().cast_const()
}
