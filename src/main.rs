use clap::Parser;

/// Show available printers and drivers
#[derive(Parser, Debug)]
#[command(name = "lpinfo-rs")]
#[command(about = "Lists available devices or drivers for CUPS")]
#[command(version)]
struct Args {
    /// Show long listing (verbose output)
    #[arg(short = 'l', long = "long")]
    long_status: bool,

    /// Show available devices
    #[arg(short = 'v', long = "devices")]
    show_devices: bool,

    /// Show available models/drivers
    #[arg(short = 'm', long = "models")]
    show_models: bool,

    /// Encrypt the connection to the server
    #[arg(short = 'E', long = "encrypt")]
    encrypt: bool,

    /// Connect to the named server
    #[arg(short = 'h', long = "host", value_name = "SERVER[:PORT]")]
    host: Option<String>,

    /// IEEE 1284 device ID to match
    #[arg(long = "device-id", value_name = "DEVICE-ID")]
    device_id: Option<String>,

    /// Language/locale to match
    #[arg(long = "language", value_name = "LOCALE")]
    language: Option<String>,

    /// Make and model to match
    #[arg(long = "make-and-model", value_name = "NAME")]
    make_model: Option<String>,

    /// PostScript product to match
    #[arg(long = "product", value_name = "NAME")]
    product: Option<String>,

    /// URI schemes to include
    #[arg(long = "include-schemes", value_name = "SCHEME-LIST")]
    include_schemes: Option<String>,

    /// URI schemes to exclude
    #[arg(long = "exclude-schemes", value_name = "SCHEME-LIST")]
    exclude_schemes: Option<String>,

    /// Device discovery timeout in seconds
    #[arg(long = "timeout", value_name = "SECONDS", default_value = "15")]
    timeout: i32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
