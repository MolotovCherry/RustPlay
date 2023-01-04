use std::panic;
#[cfg(debug_assertions)]
use {regex::Regex, std::backtrace::Backtrace};

use crate::popup::{display_popup, MessageBoxIcon};

pub fn set_hook() {
    panic::set_hook(Box::new(|v| {
        #[cfg(debug_assertions)]
        {
            let panic_msg = v.to_string();
            let backtrace = Backtrace::force_capture();

            let full_backtrace = backtrace.to_string();
            let raw_frames = full_backtrace.split("\n").collect::<Vec<_>>();

            // Sort frames into a single frame depending on frame content
            let mut frames = vec![];
            for chunk_frames in raw_frames.chunks(2) {
                let main_frame = chunk_frames.get(0);
                let sub_frame = chunk_frames.get(1);

                if main_frame.is_some() && sub_frame.is_some() {
                    let main_frame = *main_frame.unwrap();
                    let sub_frame = *sub_frame.unwrap();

                    if sub_frame.trim().starts_with("at") {
                        frames.push(format!("{main_frame}\n{sub_frame}"));
                    } else if main_frame.trim().starts_with("at") {
                        frames
                            .last_mut()
                            .unwrap()
                            .push_str(&format!("\n{main_frame}"));
                        frames.push(sub_frame.to_string());
                    } else {
                        frames.push(main_frame.to_string());
                        if !sub_frame.trim().is_empty() {
                            frames.push(sub_frame.to_string());
                        }
                    }
                } else {
                    let main_frame = main_frame.unwrap();
                    if !main_frame.trim().is_empty() {
                        // end of array
                        frames.push(main_frame.to_string());
                    }
                }
            }

            // use the frame list generated earlier and sort through them and create a short backtrace from it
            let re = Regex::new(r"[0-9]+: ").unwrap();
            let mut capture = false;
            let frames = frames
                .into_iter()
                // filter out all non-short backtraces
                .filter(|frame| {
                    if frame.contains("__rust_end_short_backtrace") {
                        capture = true;
                        // skip this current frame
                        return false;
                    }

                    if frame.contains("__rust_begin_short_backtrace") {
                        // skip this frame and all following frames
                        capture = false;
                    }

                    capture
                })
                .enumerate()
                .map(|(i, frame)| re.replace(&frame, format!("{i}: ")).into_owned())
                .collect::<Vec<_>>();

            eprintln!("{}\n\nstack backtrace:\n{}", panic_msg, frames.join("\n"));
        }

        display_popup(
            "RustPlay panicked :(",
            &v.to_string(),
            MessageBoxIcon::Error,
        );
    }));
}
