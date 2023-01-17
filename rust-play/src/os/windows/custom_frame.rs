use std::{
    cmp::max,
    sync::{mpsc::Receiver, Mutex},
};

use crate::widgets::titlebar::TITLEBAR_HEIGHT;
use crate::CoveredRects;
use egui::{mutex::RwLock, Pos2, Rect};
use once_cell::sync::OnceCell;
use smallvec::SmallVec;

use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowLongPtrW, WM_CREATE, WM_STYLECHANGED, WS_SYSMENU,
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
            GetWindowLongW, GetWindowRect, SendMessageW, SetWindowsHookExW, GCW_ATOM, GWL_STYLE,
            HCBT_CREATEWND, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTLEFT, HTNOWHERE,
            HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, MINMAXINFO, TITLEBARINFOEX, WH_CBT,
            WINDOW_EX_STYLE, WM_GETMINMAXINFO, WM_GETTITLEBARINFOEX, WM_NCCALCSIZE, WM_NCHITTEST,
            WS_BORDER, WS_CAPTION, WS_OVERLAPPEDWINDOW, WS_THICKFRAME, WS_VISIBLE,
        },
    },
};

use super::dwm_win32::apply_acrylic;

const WC_DIALOG: u32 = 0x8002;

static COVERED_TITLEBAR_AREA: OnceCell<RwLock<CoveredRects>> = OnceCell::new();

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

pub fn init(receiver: Receiver<CoveredRects>) {
    // continually update the covered titlebar area
    let _ = COVERED_TITLEBAR_AREA.set(RwLock::new(SmallVec::new()));

    // thread to watch for events down the channel and update them
    std::thread::spawn(move || loop {
        let rects = receiver.recv();

        if let Ok(rects) = rects {
            let mut writer = COVERED_TITLEBAR_AREA.get().unwrap().write();
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

            // let dpi = GetDpiForWindow(hwnd);

            // let frame_x = GetSystemMetricsForDpi(SM_CXFRAME, dpi);
            // let frame_y = GetSystemMetricsForDpi(SM_CYFRAME, dpi);
            // let padding = GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);

            // let mut params = *(lparam as *mut NCCALCSIZE_PARAMS);
            // let rc_rect = &mut params.rgrc[0];

            // rc_rect.right -= frame_x + padding;
            // rc_rect.left += frame_x + padding;
            // rc_rect.bottom -= frame_y + padding;

            // // check if it's maximized
            // let mut placement = WINDOWPLACEMENT::default();
            // if GetWindowPlacement(hwnd, &mut placement).as_bool()
            //     && placement.showCmd == SW_SHOWMAXIMIZED
            // {
            //     rc_rect.top += padding;
            // }

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

        WM_GETMINMAXINFO => {
            if uidsubclass == 1 {
                let mut minmaxinfo = &mut *(lparam as *mut MINMAXINFO);

                let reader = COVERED_TITLEBAR_AREA.get().unwrap().read();
                // noteL:: this is in window coords, not screen space coords!
                let blank_rect = Rect::from_two_pos(Pos2::new(0.0, 0.0), Pos2::new(0.0, 0.0));
                let collision_rect = *reader
                    .iter()
                    .reduce(|acc, e| if acc.right() < e.right() { acc } else { e })
                    .unwrap_or(&blank_rect);

                // only allowed to go as far as the latest tab
                minmaxinfo.ptMinTrackSize.x =
                    max((collision_rect.left() + collision_rect.right()) as i32, 500);
                minmaxinfo.ptMinTrackSize.y = 500;

                *f_call_dsp = false;
                l_ret = 0;
            }
        }

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

    // test if the window is resizable; entire frame should be draggable if it's not resizable
    let window_style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 };

    // Determine if the hit test is for resizing. Default middle (1,1).
    let mut u_row = 1;
    let mut u_col = 1;
    let mut f_on_resize_border = false;

    let mut hit_client_area = false;
    // Calculate here whether we are on client area in the titlebar and skip the hittesting code
    // ONLY do this for the main window, cause that's the one with the tabs
    if uidsubclass == 1 {
        let covered_area = COVERED_TITLEBAR_AREA.get().unwrap().read();
        for rect in covered_area.iter() {
            // this rect is in client coords instead of screenspace coords, so we need to convert it
            let covered_rect = RECT {
                left: if rect.left() as i32 == 0 {
                    // don't cover the resizing border
                    if window_style & WS_THICKFRAME.0 == 0 {
                        rc_window.left
                    } else {
                        // we have a resize border, so account for that
                        rc_window.left + 10
                    }
                } else {
                    rc_window.left + rect.left().ceil() as i32
                },
                right: rc_window.left + rect.right().ceil() as i32,
                top: rc_window.top + 10,
                bottom: rc_window.top + rect.bottom().ceil() as i32,
            };

            if cursor_pos.x >= covered_rect.left
                && cursor_pos.x <= covered_rect.right
                && cursor_pos.y >= covered_rect.top
                && cursor_pos.y <= covered_rect.bottom
            {
                hit_client_area = true;
                break;
            }
        }
    }

    // only check if we hit non-client area
    if !hit_client_area {
        // Determine if the point is at the top or bottom of the window.

        // First, check if we're anywhere on the titlebar
        if cursor_pos.y >= rc_window.top && cursor_pos.y < rc_window.top + TITLEBAR_HEIGHT {
            // now check if we're on the titlebar division for top resizing
            if cursor_pos.y >= rc_window.top && cursor_pos.y < rc_window.top + 10 {
                // use the top resizing, but ONLY if window is not resizable
                f_on_resize_border = window_style & WS_THICKFRAME.0 > 0;
                u_row = 0;

                // otherwise, use the caption dragging, ONLY IF not within 10 of X sides
            } else if !(cursor_pos.x >= rc_window.left && cursor_pos.x < rc_window.left + 10
                || cursor_pos.x < rc_window.right && cursor_pos.x >= rc_window.right - 10)
            {
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
    }

    // Hit test (HTTOPLEFT, ... HTBOTTOMRIGHT)
    [
        [
            HTTOPLEFT,
            if f_on_resize_border { HTTOP } else { HTCAPTION },
            HTTOPRIGHT,
        ],
        [HTLEFT, HTNOWHERE, HTRIGHT],
        [HTBOTTOMLEFT, HTBOTTOM, HTBOTTOMRIGHT],
    ][u_row][u_col] as isize;

    return HTNOWHERE as isize;
}
