use clap::{crate_authors, crate_version, App, AppSettings, Arg};
use clap_generate::{
    generate,
    generators::{Bash, Elvish, Fish, PowerShell, Zsh},
};
use client::client;
use clipboard::ClipboardContext;
use crossbeam_channel::Receiver;
use log::info;
use std::{io, process};

mod client;
mod clipboard;
mod error;
mod server;

async fn client_mode(host: &str, port: u16, ctx: ClipboardContext, receiver: Receiver<String>) {
    info!("run in client mode");
    client(host, port, ctx, receiver).await;
}

async fn server_mode(host: &str, port: u16, ctx: ClipboardContext, receiver: Receiver<String>) {
    info!("run in server mode");
    server::server(&host, port, ctx, receiver).await.unwrap()
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

#[actix_web::main]
async fn main() {
    let (sender, receiver) = crossbeam_channel::unbounded();

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
                .required_if_eq("mode", "client")
                .default_value("0.0.0.0"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .takes_value(true)
                .default_value("8000"),
        )
        .arg(
            Arg::new("mode")
                .short('m')
                .long("mode")
                .required(true)
                .takes_value(true)
                .possible_values(&["server", "client"]),
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

    let host = matches.value_of("host").unwrap();
    let port = matches.value_of("port").unwrap().parse().unwrap();

    flexi_logger::Logger::with_env_or_str(default_log_level)
        .start()
        .expect("logger");

    let clipboard_ctx = ClipboardContext::new();

    let ctx = clipboard_ctx.clone();
    actix_rt::spawn(async move { clipboard::clipboard_loop(ctx, sender).await.unwrap() });

    match matches.value_of("mode") {
        Some("server") => server_mode(host, port, clipboard_ctx, receiver).await,
        Some("client") => client_mode(host, port, clipboard_ctx, receiver).await,
        _ => unreachable!(),
    }
}
