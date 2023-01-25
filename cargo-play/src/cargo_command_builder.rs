use std::process::Command;

use crate::{BuildType, Channel, Subcommand};

#[derive(Debug, Default, Clone)]
pub struct CargoCommandBuilder<'a> {
    pub channel: Option<Channel>,
    pub subcommand: Option<Subcommand>,
    // debug or release
    pub build_type: Option<BuildType>,
    pub cargo_flags: Option<Vec<&'a str>>,
    pub subcommand_flags: Option<Vec<&'a str>>,
    pub dash_args: Option<Vec<&'a str>>,
}

#[allow(dead_code)]
impl<'a> CargoCommandBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn channel(&mut self, channel: Channel) -> &mut Self {
        self.channel = Some(channel);
        self
    }

    pub fn subcommand(&mut self, subcommand: Subcommand) -> &mut Self {
        self.subcommand = Some(subcommand);
        self
    }

    pub fn build_type(&mut self, build_type: BuildType) -> &mut Self {
        self.build_type = Some(build_type);
        self
    }

    pub fn subcommand_flag(&mut self, flag: &'a str) -> &mut Self {
        if self.subcommand_flags.is_none() {
            self.subcommand_flags = Some(vec![]);
        }

        self.subcommand_flags.as_mut().unwrap().push(flag);
        self
    }

    pub fn subcommand_flags(&mut self, flags: &[&'a str]) -> &mut Self {
        if let Some(subflags) = &mut self.subcommand_flags {
            subflags.extend(flags);
        } else {
            self.subcommand_flags = Some(flags.to_vec());
        }

        self
    }

    pub fn cargo_flag(&mut self, flag: &'a str) -> &mut Self {
        if self.cargo_flags.is_none() {
            self.cargo_flags = Some(vec![]);
        }

        self.cargo_flags.as_mut().unwrap().push(flag);
        self
    }

    pub fn cargo_flags(&mut self, flags: &[&'a str]) -> &mut Self {
        if let Some(cargoflags) = &mut self.cargo_flags {
            cargoflags.extend(flags);
        } else {
            self.cargo_flags = Some(flags.to_vec());
        }

        self
    }

    pub fn dash_arg(&mut self, arg: &'a str) -> &mut Self {
        if self.dash_args.is_none() {
            self.dash_args = Some(vec![]);
        }

        self.dash_args.as_mut().unwrap().push(arg);
        self
    }

    pub fn dash_args(&mut self, args: &[&'a str]) -> &mut Self {
        if let Some(dashargs) = &mut self.dash_args {
            dashargs.extend(args);
        } else {
            self.dash_args = Some(args.to_vec());
        }

        self
    }

    pub fn build(&self) -> Command {
        let mut command = Command::new("cargo");

        if let Some(channel) = self.channel {
            let channel: &str = channel.into();
            command.arg(&format!("+{channel}"));
        }

        if let Some(flags) = &self.cargo_flags {
            command.args(flags);
        }

        if let Some(subcommand) = self.subcommand {
            command.arg::<&str>(subcommand.into());
        }

        if let Some(flags) = &self.subcommand_flags {
            command.args(flags);
        }

        if let Some(build_type) = self.build_type {
            if build_type == BuildType::Release {
                command.arg::<&str>(build_type.into());
            }
        }

        if let Some(flags) = &self.dash_args {
            command.arg("--");
            command.args(flags);
        }

        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_command_format_order() {
        let mut builder = CargoCommandBuilder::new();
        builder
            .channel(Channel::Stable)
            .subcommand(Subcommand::Run)
            .cargo_flags(&["--bar2"])
            .cargo_flag("--foo")
            .cargo_flags(&["--bar", "--baz"])
            .subcommand_flags(&["--subflag2"])
            .subcommand_flag("--subflag")
            .subcommand_flags(&["--subbar", "--subbaz"])
            .build_type(BuildType::Release)
            .dash_arg("--dash")
            .dash_args(&["--another-dash", "--and-two"]);

        let command = builder.build();

        let mut commandline = command.get_program().to_str().unwrap().to_string();
        commandline.push_str(
            &command
                .get_args()
                .map(|i| format!(" {}", i.to_str().unwrap()))
                .collect::<String>(),
        );

        assert_eq!("cargo +stable --bar2 --foo --bar --baz run --subflag2 --subflag --subbar --subbaz --release -- --dash --another-dash --and-two", commandline);

        // BuildType::Debug is a no-op
        let mut builder = CargoCommandBuilder::new();
        builder
            .channel(Channel::Stable)
            .subcommand(Subcommand::Run)
            .build_type(BuildType::Debug);
        let command = builder.build();

        let mut commandline = command.get_program().to_str().unwrap().to_string();
        commandline.push_str(
            &command
                .get_args()
                .map(|i| format!(" {}", i.to_str().unwrap()))
                .collect::<String>(),
        );

        assert_eq!("cargo +stable run", commandline);
    }
}
