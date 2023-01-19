use std::sync::{mpsc::Receiver, Mutex};

use crate::widgets::titlebar::TITLEBAR_HEIGHT;
use crate::CaptionMaxRect;
use egui::{mutex::RwLock, Rect};
use once_cell::sync::OnceCell;

use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowLongPtrW, HTCLOSE, HTMAXBUTTON, HTMINBUTTON, WM_CREATE, WM_NCLBUTTONDOWN,
    WM_STYLECHANGED, WS_SYSMENU,
};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Dwm::{DwmDefWindowProc, DwmExtendFrameIntoClientArea, DwmIsCompositionEnabled},
    System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
    UI::{
        Controls::MARGINS,
        Shell::{DefSubclassProc, SetWindowSubclass},
        WindowsAndMessaging::{
            AdjustWindowRectEx, CallNextHookEx, DefWindowProcW, GetClassLongW, GetWindowLongPtrW,
            GetWindowLongW, GetWindowRect, SetWindowsHookExW, GCW_ATOM, GWL_STYLE, HCBT_CREATEWND,
            HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTLEFT, HTNOWHERE, HTRIGHT, HTTOP, HTTOPLEFT,
            HTTOPRIGHT, WH_CBT, WINDOW_EX_STYLE, WM_NCCALCSIZE, WM_NCHITTEST, WS_BORDER,
            WS_CAPTION, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
        },
    },
};

use super::dwm_win32::apply_acrylic;

const WC_DIALOG: u32 = 0x8002;

static MAX_RECT: OnceCell<RwLock<CaptionMaxRect>> = OnceCell::new();

// macro_rules! RGB {
//     ($r:expr, $g:expr, $b:expr) => {{
//         let rgb = $r as u32 | ($g as u32) << 8 | ($b as u32) << 16;
//         ::windows::Win32::Foundation::COLORREF(rgb)
//     }};
// }

// extract low bits for x coord
macro_rules! x_coord {
    ($x: ident) => {
        ($x as i32 & 0xffff)
    };
}

// extract high bits for y coord
macro_rules! y_coord {
    ($y: ident) => {
        ($y as i32 >> 16 & 0xffff)
    };
}

pub fn init(receiver: Receiver<CaptionMaxRect>) {
    // continually update the covered titlebar area
    let _ = MAX_RECT.set(RwLock::new(Rect::NOTHING));

    // thread to watch for events down the channel and update them
    std::thread::spawn(move || loop {
        let rects = receiver.recv();

        if let Ok(rects) = rects {
            let mut writer = MAX_RECT.get().unwrap().write();
            *writer = rects;
        } else {
            break;
        }
    });

    // install a hook to wait for window creation, then set a new subclassproc
    unsafe {
        let hinstance = GetModuleHandleW(None).expect("Failed to get HINSTANCE");

        SetWindowsHookExW(
            WH_CBT,
            Some(window_hook_callback),
            hinstance,
            GetCurrentThreadId(),
        )
        .expect("Failed to setup hook");
    }
}

static SUBCLASS_UID_COUNTER: Mutex<usize> = Mutex::new(0);

// Callback function for the window hook
unsafe extern "system" fn window_hook_callback(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code as u32 == HCBT_CREATEWND {
        let hwnd = HWND(wparam.0 as isize);

        let window_style = GetWindowLongW(hwnd, GWL_STYLE) as u32;

        // only allow top-level actual windows to pass through
        if window_style & (WS_CAPTION.0 | WS_BORDER.0 | WS_VISIBLE.0) > 0 {
            let atom = GetClassLongW(hwnd, GCW_ATOM);
            if atom != WC_DIALOG {
                let counter = &mut *SUBCLASS_UID_COUNTER.lock().unwrap();

                let res = SetWindowSubclass(hwnd, Some(subclass_proc), *counter + 1, 0).as_bool();

                if !res {
                    panic!("Failed to set subclass proc");
                }

                *counter += 1;
            }
        }
    }

    // Call the next hook in the chain
    CallNextHookEx(None, code, wparam, lparam)
}

pub unsafe fn is_dwm_enabled() -> bool {
    let dwm_enabled_result = DwmIsCompositionEnabled();

    dwm_enabled_result.is_ok() && dwm_enabled_result.unwrap().as_bool()
}

// handle a custom subclassproc
unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    u_msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    uidsubclass: usize,
    _dw_ref_data: usize,
) -> LRESULT {
    let mut f_call_dsp = true;
    let mut l_ret = 0;

    if is_dwm_enabled() {
        l_ret = custom_subclass_proc(
            hwnd,
            u_msg,
            wparam.0,
            lparam.0,
            &mut f_call_dsp,
            uidsubclass,
        );
    }

    if f_call_dsp {
        return DefSubclassProc(hwnd, u_msg, wparam, lparam);
    }

    LRESULT(l_ret)
}

// handle the custom window
unsafe fn custom_subclass_proc(
    hwnd: HWND,
    u_msg: u32,
    wparam: usize,
    lparam: isize,
    f_call_dsp: &mut bool,
    uidsubclass: usize,
) -> isize {
    let mut l_ret = LRESULT(0);
    *f_call_dsp =
        !DwmDefWindowProc(hwnd, u_msg, WPARAM(wparam), LPARAM(lparam), &mut l_ret).as_bool();
    let mut l_ret = l_ret.0;

    match u_msg {
        WM_CREATE => {
            // Extend the frame into the client area.
            let margins = MARGINS {
                cxLeftWidth: 0,
                cxRightWidth: 0,
                cyBottomHeight: 0,
                cyTopHeight: TITLEBAR_HEIGHT,
            };

            DwmExtendFrameIntoClientArea(hwnd, &margins).expect("Failed to extend frame");

            apply_acrylic(hwnd, None);
        }

        WM_STYLECHANGED => {
            // remove all caption buttons - we'll manually implement them instead
            let current_style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            if current_style & WS_SYSMENU.0 as isize > 0 {
                SetWindowLongPtrW(hwnd, GWL_STYLE, current_style & !(WS_SYSMENU.0 as isize));
            }
        }

        WM_NCCALCSIZE => {
            if wparam == 0 {
                return DefWindowProcW(hwnd, u_msg, WPARAM(wparam), LPARAM(lparam)).0;
            }

            *f_call_dsp = false;
            l_ret = 0;
        }

        // conduct non-client hit testing
        WM_NCHITTEST => {
            // for ease, we will always return HTNOWHERE and let egui handle this, except for the maximize button
            l_ret = hit_test_nca(hwnd, wparam, lparam, uidsubclass);

            if l_ret != HTNOWHERE as isize {
                *f_call_dsp = false;
            }
        }

        // When HTMAXBUTTON is pressed, DO NOT let default handler handle it, just no-op it
        WM_NCLBUTTONDOWN => match wparam as u32 {
            HTMINBUTTON | HTMAXBUTTON | HTCLOSE => {
                *f_call_dsp = false;
                l_ret = 0;
            }

            _ => (),
        },

        _ => (),
    }

    l_ret
}

// Hit test the frame for resizing and moving, and overlayed content in titlebar
fn hit_test_nca(hwnd: HWND, _: usize, lparam: isize, uidsubclass: usize) -> isize {
    // Get the point coordinates for the hit test.
    let cursor_pos = POINT {
        x: x_coord!(lparam),
        y: y_coord!(lparam),
    };

    // Get the window rectangle.
    let mut rc_window = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut rc_window);
    }

    // Get the frame rectangle, adjusted for the style without a caption.
    let mut rc_frame = RECT::default();
    unsafe {
        AdjustWindowRectEx(
            &mut rc_frame,
            WS_OVERLAPPEDWINDOW & !WS_CAPTION,
            false,
            WINDOW_EX_STYLE::default(),
        );
    }

    // Determine if the hit test is for resizing. Default middle (1,1).
    let mut u_row = 1;
    let mut u_col = 1;

    // Calculate here whether we are on client area in the titlebar and trigger the maximize button
    if uidsubclass == 1 {
        let rect = MAX_RECT.get().unwrap().read();

        // this rect is in client coords instead of screenspace coords, so we need to convert it
        let covered_rect = RECT {
            left: rc_window.left + (rect.left().ceil() as i32 * 2),
            right: rc_window.left + (rect.right().ceil() as i32 * 2),
            top: rc_window.top + 5,
            bottom: rc_window.top + (rect.bottom().ceil() as i32 * 2),
        };

        if cursor_pos.x >= covered_rect.left
            && cursor_pos.x <= covered_rect.right
            && cursor_pos.y >= covered_rect.top
            && cursor_pos.y <= covered_rect.bottom
        {
            return HTMAXBUTTON as isize;
        }
    }

    // Determine if the point is at the top or bottom of the window.

    // First, check if we're anywhere on the titlebar
    if cursor_pos.y >= rc_window.top && cursor_pos.y < rc_window.top + TITLEBAR_HEIGHT {
        // now check if we're on the titlebar division for top resizing
        if cursor_pos.y >= rc_window.top && cursor_pos.y < rc_window.top + 5 {
            u_row = 0;
        }
    } else if cursor_pos.y < rc_window.bottom && cursor_pos.y >= rc_window.bottom - 10 {
        u_row = 2;
    }

    // Determine if the point is at the left or right of the window.
    if cursor_pos.x >= rc_window.left && cursor_pos.x < rc_window.left + 10 {
        u_col = 0; // left side
    } else if cursor_pos.x < rc_window.right && cursor_pos.x >= rc_window.right - 10 {
        u_col = 2; // right side
    }

    // Hit test (HTTOPLEFT, ... HTBOTTOMRIGHT)
    [
        [HTTOPLEFT, HTTOP, HTTOPRIGHT],
        [HTLEFT, HTNOWHERE, HTRIGHT],
        [HTBOTTOMLEFT, HTBOTTOM, HTBOTTOMRIGHT],
    ][u_row][u_col] as isize
}
