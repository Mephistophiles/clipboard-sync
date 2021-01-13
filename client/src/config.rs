use clap::{crate_authors, crate_version, App, AppSettings, Arg};
use clap_generate::{
    generate,
    generators::{Bash, Elvish, Fish, PowerShell, Zsh},
};
use std::{io, process};

pub struct Config {
    pub host: String,
    pub port: u16,
    pub default_log_level: &'static str,
}

fn autocomplete(shell: &str, mut app: &mut App) {
    match shell {
        "bash" => generate::<Bash, _>(&mut app, clap::crate_name!(), &mut io::stdout()),
        "elvish" => generate::<Elvish, _>(&mut app, clap::crate_name!(), &mut io::stdout()),
        "fish" => generate::<Fish, _>(&mut app, clap::crate_name!(), &mut io::stdout()),
        "powershell" => generate::<PowerShell, _>(&mut app, clap::crate_name!(), &mut io::stdout()),
        "zsh" => generate::<Zsh, _>(&mut app, clap::crate_name!(), &mut io::stdout()),
        _ => panic!("Unknown generator"),
    }
}

impl Config {
    pub fn from_args() -> Self {
        let mut app = App::new("clipboard-sync")
            .setting(AppSettings::SubcommandsNegateReqs)
            .author(crate_authors!())
            .version(crate_version!())
            .about("Sync clipboard over HTTP")
            .arg(
                Arg::new("host")
                    .short('h')
                    .long("host")
                    .takes_value(true)
                    .default_value("localhost"),
            )
            .arg(
                Arg::new("port")
                    .short('p')
                    .long("port")
                    .takes_value(true)
                    .default_value("8000"),
            )
            .arg(Arg::new("verbose").short('v').multiple_occurrences(true))
            .subcommand(
                App::new("init")
                    .about("Prints the shell function used to execute")
                    .arg(
                        Arg::new("shell")
                            .value_name("SHELL")
                            .takes_value(true)
                            .required(true)
                            .possible_values(&["bash", "elvish", "fish", "powershell", "zsh"]),
                    ),
            );

        let matches = app.clone().get_matches();

        let default_log_level = match matches.occurrences_of("verbose") {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        };

        if let Some(init) = matches.subcommand_matches("init") {
            autocomplete(init.value_of("shell").unwrap(), &mut app);
            process::exit(0);
        }

        let host = matches.value_of("host").unwrap().to_string();
        let port = matches.value_of("port").unwrap().parse().unwrap();

        Self {
            host,
            port,
            default_log_level,
        }
    }
}
