# RustPlay
A desktop program for playing Rust scratch files

This program is currently alpha, but has a basic working terminal and compiles and runs rust scratches.  
Currently Windows only.

### But why?
I got tired of having to go to play.rust-lang.org. I just want something I can use on my own, without limit. Something I don't need a browser for.

I am also the author of an IntelliJ rust scratch plugin, but it was too cumbersome to continue with all of their api changes. But now - now with a desktop app, there are no limits!

## Requirements
- You need to have Rust / cargo installed of course :wink:
- You use windows (Linux support will come in time)

## Current Features
- Windows support
- Nice titlebar with tabs integrated into decoration WITH windows acrylic support (win11 only)
- Multiple tabs
- Can play scratches (very basic functionality currently)
- Nice terminal snapping close / open functionality
- Dynamic code analysis automatically infers dependencies from everywhere in your source code using your `use` statements
- - Is even smart enough to know when the crate is named `crate-name` instead of `crate_name` on crates.io and automatically fix it for you. These cases don't need a custom dependency declaration!
- For more complex needs such as specific versions and crate features, can manually include deps in cargo.toml or even whole sections of cargo.toml code

![Ui Demo](/readme_assets/ui.gif)

## Next Goals
- Theming with config
- Linux Support
- Rename / save / upload to play.rust-lang.org (gist creation functionality is already done)
- Project options window to configure every aspect of the rust scratch build. Will support nearly all optionsthat the website supports!
- Parse color in the terminal
- Possible redesign of terminal window (not sure what to do yet)
- Fix caption buttons to clip ui instead of overlay
- Clearing terminal screen
- Misc bugs such as properly switching terminal to other tabs terminal output when switching tabs
- Obvious redesign of all the buttons, including that play button. It looks wrong
- Potential caching of files / inference / cargo.toml in order to speed up runs.
- [Line numbers in code editor](https://github.com/emilk/egui/issues/1534)
- Fix/improve svg caption button icons. They don't seem to look the same in 1080p
- Panic popup window Linux support (not sure how)
- Potential multi-file module project builds (`cargo-player` does support this)
- Cleanup old textedit state (tabs with the same node/tabindex use the same id hashes, and textedit state is persisted and not cleared when closing tabs)

## Contributions
All contributions are welcome!

If you'd like to contribute, please choose a goal above! :smile: If you need help, please head over to the discussions section of the project

Let's make this the best Rust Scratch app ever!
