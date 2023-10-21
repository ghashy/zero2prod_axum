use clap::Parser;

use zero2prod_axum::{configuration::Settings, startup::Application};

/// Backend zero2prod server, all args passed, unix socket preferred
#[derive(Parser, Debug)]
#[clap(version, about)]
struct Arguments {
    /// Ipv4 or Ipv6 address.
    #[clap(short, long, value_parser, num_args = 1)]
    ip: Option<std::net::IpAddr>,
    /// Tcp socket port
    #[clap(short, long)]
    port: Option<u16>,
    /// Unix socket path
    #[clap(short, long)]
    unix_socket: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_level(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set up tracing");

    // Panic if we can't read configuration
    let mut config =
        Settings::load_configuration().expect("Failed to read configuration.");

    // If args passed, override configuration
    if let Some(unix) = args.unix_socket {
        config.unix_socket = unix;
    } else if let Some(ip) = args.ip {
        if let Some(port) = args.port {
            config.app_addr = ip.to_string();
            config.app_port = port;
        }
    }

    if let Err(e) = Application::build(config)
        .await
        .expect("Failed to build application")
        .run_until_stopped()
        .await
    {
        eprintln!("Error: {}", e);
    }
}
