// Copyright (c) 2022 pyke.io (https://github.com/pykeio/vibe)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(non_snake_case, clippy::upper_case_acronyms, non_camel_case_types)]

use super::win_version::{is_win10_1809, is_win11, is_win11_22h2};
use crate::popup::{display_popup, MessageBoxIcon};
use std::ffi::c_void;

use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Dwm::{
    DwmEnableBlurBehindWindow, DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND,
};
use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;
use windows::Win32::{
    Foundation::{BOOL, HWND},
    Graphics::Dwm::{DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWINDOWATTRIBUTE},
    UI::Controls::MARGINS,
};

use windows_dll::dll;

#[dll("user32.dll")]
extern "system" {
    #[allow(non_snake_case)]
    unsafe extern "system" fn SetWindowCompositionAttribute(
        hwnd: HWND,
        data: *mut WINDOWCOMPOSITIONATTRIBDATA,
    ) -> BOOL;
}

type WINDOWCOMPOSITIONATTRIB = u32;

const DWMWA_USE_IMMERSIVE_DARK_MODE: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(20i32);
const DWMWA_MICA_EFFECT: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(1029i32);
const DWMWA_SYSTEMBACKDROP_TYPE: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(38i32);

#[derive(PartialEq, Eq)]
#[repr(C)]
enum ACCENT_STATE {
    ACCENT_DISABLED = 0,
    ACCENT_ENABLE_BLURBEHIND = 3,
    ACCENT_ENABLE_ACRYLICBLURBEHIND = 4,
}

#[repr(C)]
struct ACCENT_POLICY {
    AccentState: u32,
    AccentFlags: u32,
    GradientColour: u32,
    AnimationId: u32,
}

#[repr(C)]
struct WINDOWCOMPOSITIONATTRIBDATA {
    Attrib: WINDOWCOMPOSITIONATTRIB,
    pvData: *mut c_void,
    cbData: usize,
}

#[repr(C)]
enum DWM_SYSTEMBACKDROP_TYPE {
    DWMSBT_DISABLE = 1,
    DWMSBT_MAINWINDOW = 2,      // Mica
    DWMSBT_TRANSIENTWINDOW = 3, // Acrylic
}

unsafe fn set_accent_policy(hwnd: HWND, accent_state: ACCENT_STATE, colour: Option<[u8; 4]>) {
    let mut colour = colour.unwrap_or_default();

    let is_acrylic = accent_state == ACCENT_STATE::ACCENT_ENABLE_ACRYLICBLURBEHIND;
    if is_acrylic && colour[3] == 0 {
        // acrylic doesn't like to have 0 alpha
        colour[3] = 1;
    }

    let mut policy = ACCENT_POLICY {
        AccentState: accent_state as _,
        AccentFlags: if is_acrylic { 0 } else { 2 },
        GradientColour: (colour[0] as u32)
            | (colour[1] as u32) << 8
            | (colour[2] as u32) << 16
            | (colour[3] as u32) << 24,
        AnimationId: 0,
    };
    let mut data = WINDOWCOMPOSITIONATTRIBDATA {
        Attrib: 0x13,
        pvData: &mut policy as *mut _ as _,
        cbData: std::mem::size_of_val(&policy),
    };
    SetWindowCompositionAttribute(hwnd, &mut data);
}

pub fn force_dark_theme(hwnd: HWND) {
    if is_win11() {
        unsafe {
            DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &1 as *const _ as _, 4)
                .expect("Failed to set window attribute");
        }
    } else if is_win10_1809() {
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWINDOWATTRIBUTE(DWMWA_USE_IMMERSIVE_DARK_MODE.0 - 1),
                &1 as *const _ as _,
                4,
            )
            .expect("Failed to set window attribute");
        }
    } else {
        display_popup(
            "Not available",
            "\"force_dark_theme()\" is only available on Windows 10 v1809+ or Windows 11",
            MessageBoxIcon::Error,
        );
    }
}

pub fn force_light_theme(hwnd: HWND) {
    if is_win11() {
        unsafe {
            DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &0 as *const _ as _, 4)
                .expect("Failed to set window attribute");
        }
    } else if is_win10_1809() {
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWINDOWATTRIBUTE(DWMWA_USE_IMMERSIVE_DARK_MODE.0 - 1),
                &0 as *const _ as _,
                4,
            )
            .expect("Failed to set window attribute");
        }
    } else {
        display_popup(
            "Not available",
            "\"force_light_theme()\" is only available on Windows 10 v1809+ or Windows 11",
            MessageBoxIcon::Error,
        );
    }
}

pub fn apply_acrylic(hwnd: HWND, color: Option<[u8; 4]>) {
    if is_win11_22h2() {
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                &DWM_SYSTEMBACKDROP_TYPE::DWMSBT_TRANSIENTWINDOW as *const _ as _,
                4,
            )
            .expect("Failed to set window attribute");
        }
    } else {
        unsafe {
            set_accent_policy(
                hwnd,
                if is_win10_1809() {
                    ACCENT_STATE::ACCENT_ENABLE_ACRYLICBLURBEHIND
                } else {
                    // win7 +
                    ACCENT_STATE::ACCENT_ENABLE_BLURBEHIND
                },
                color,
            );
        }
    }
}

pub fn clear_acrylic(hwnd: HWND) {
    if is_win11_22h2() {
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                &DWM_SYSTEMBACKDROP_TYPE::DWMSBT_DISABLE as *const _ as _,
                4,
            )
            .expect("Failed to set window attribute");
        }
    } else {
        display_popup(
            "Not available",
            "\"clear_acrylic()\" is only available on Windows 7+",
            MessageBoxIcon::Error,
        );
    }
}

pub fn apply_mica(hwnd: HWND) {
    if is_win11_22h2() {
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                &DWM_SYSTEMBACKDROP_TYPE::DWMSBT_MAINWINDOW as *const _ as _,
                4,
            )
            .expect("Failed to set window attribute");
        }
    } else if is_win11() {
        unsafe {
            DwmSetWindowAttribute(hwnd, DWMWA_MICA_EFFECT, &1 as *const _ as _, 4)
                .expect("Failed to set window attribute");
        }
    } else {
        display_popup(
            "Not available",
            "\"apply_mica()\" is only available on Windows 11",
            MessageBoxIcon::Error,
        );
    }
}

pub fn clear_mica(hwnd: HWND) {
    if is_win11_22h2() {
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                &DWM_SYSTEMBACKDROP_TYPE::DWMSBT_DISABLE as *const _ as _,
                4,
            )
            .expect("Failed to set window attribute");
        }
    } else if is_win11() {
        unsafe {
            DwmSetWindowAttribute(hwnd, DWMWA_MICA_EFFECT, &0 as *const _ as _, 4)
                .expect("Failed to set window attribute");
        }
    } else {
        display_popup(
            "Not available",
            "\"clear_mica()\" is only available on Windows 11",
            MessageBoxIcon::Error,
        );
    }
}
