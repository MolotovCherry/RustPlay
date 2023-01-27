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
- Nice titlebar with tabs integrated into decoration
- Multiple tabs
- Can play scratches (very basic functionality currently)
- Nice terminal snapping close / open functionality
- Automatically infers dependencies from your `use` statements via dynamic code analysis
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

## Contributions
All contributions are welcome!

If you'd like to contribute, please choose a goal above! :smile: If you need help, please head over to the discussions section of the project

Let's make this the best Rust Scratch app ever!
