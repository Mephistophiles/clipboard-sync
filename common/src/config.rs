use clap::{crate_authors, crate_version, App, AppSettings, Arg};
use clap_generate::{
    generate,
    generators::{Bash, Elvish, Fish, PowerShell, Zsh},
};
use serde::Deserialize;
use std::{fs, io, net::IpAddr, process};

pub enum Type {
    Server,
    Client,
}

#[derive(Deserialize)]
pub struct Settings {
    pub host: IpAddr,
    pub port: u16,
    pub log_level: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub client: Settings,
    pub server: Settings,
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
    fn from_args(&mut self, t: Type) {
        let app_name = match t {
            Type::Client => "clipboard-sync",
            Type::Server => "clipboard-sync-server",
        };
        let host = match t {
            Type::Client => "localhost",
            Type::Server => "0.0.0.0",
        };

        let mut app = App::new(app_name)
            .setting(AppSettings::SubcommandsNegateReqs)
            .author(crate_authors!())
            .version(crate_version!())
            .about("Sync clipboard over HTTP")
            .arg(
                Arg::new("host")
                    .short('h')
                    .long("host")
                    .takes_value(true)
                    .default_value(host),
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

        if let Some(init) = matches.subcommand_matches("init") {
            autocomplete(init.value_of("shell").unwrap(), &mut app);
            process::exit(0);
        }

        let log_level = match matches.occurrences_of("verbose") {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        };

        let settings = match t {
            Type::Client => &mut self.client,
            Type::Server => &mut self.server,
        };

        if matches.is_present("verbose") {
            settings.log_level = log_level.to_string();
        }

        if matches.is_present("host") {
            settings.host = matches.value_of("host").unwrap().parse().unwrap();
        }

        if matches.is_present("port") {
            settings.port = matches.value_of("port").unwrap().parse().unwrap();
        }
    }
}

impl Config {
    pub fn load(t: Type) -> Config {
        let mut home_dir = dirs::config_dir().unwrap();
        home_dir.push("clipboard-sync");

        fs::create_dir_all(&home_dir).unwrap();

        home_dir.push("config.toml");

        let f = fs::read_to_string(&home_dir).unwrap_or_else(|_| {
            panic!(format!(
                "Unable to open {}. Please configure.",
                home_dir.display()
            ))
        });

        let mut c: Config = toml::from_str(&f).unwrap();
        c.from_args(t);
        c
    }
}
