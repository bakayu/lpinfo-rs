use clap::Parser;
use cups_rs::bindings::{
    ipp_op_e_IPP_OP_CUPS_GET_DEVICES, ipp_op_e_IPP_OP_CUPS_GET_PPDS, ipp_tag_e_IPP_TAG_OPERATION,
};
use cups_rs::config::{EncryptionMode, set_encryption, set_server};
use cups_rs::connection::HttpConnection;
use cups_rs::options::encode_options_with_group;
use cups_rs::{
    ConnectionFlags, IppRequest, IppTag, IppValueTag, get_all_destinations, get_default_destination,
};

/// Show available printers and drivers
#[derive(Parser, Debug)]
#[command(name = "lpinfo-rs")]
#[command(about = "Lists available devices or drivers for CUPS")]
#[command(version)]
#[command(disable_help_flag = true)]
struct Args {
    /// Print help information
    #[arg(long = "help", action = clap::ArgAction::Help)]
    help: (),

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
    let args = Args::parse();

    // Validate that at least -m or -v is used
    if !args.show_devices && !args.show_models {
        eprintln!("Usage: lpinfo [options] -m");
        eprintln!("       lpinfo [options] -v");
        eprintln!("Run 'lpinfo --help' for more information.");
        std::process::exit(1);
    }

    if args.encrypt {
        set_encryption(EncryptionMode::Required);
    }

    if let Some(ref host) = args.host {
        set_server(Some(host))?;
    }

    if args.show_devices {
        show_devices(&args)?;
    }

    if args.show_models {
        show_models(&args)?;
    }

    Ok(())
}

/// Handles `-v` flag: lists the available devices.
fn show_devices(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let connection = scheduler_connection()?;

    // Create CUPS_GET_DEVICES ipp request using raw operation code

    let mut request = IppRequest::new_raw(ipp_op_e_IPP_OP_CUPS_GET_DEVICES as i32)
        .expect("Failed to create request");

    // add required IPP op attributes (matching libcups defaults)
    request.add_standard_attrs()?;

    // Add timeout attribute
    request
        .add_integer(
            IppTag::Operation,
            IppValueTag::Integer,
            "timeout",
            args.timeout,
        )
        .expect("failed at add int");

    // Add include/exclude schemes if specified
    add_cups_options(&mut request, &args.include_schemes, &args.exclude_schemes)
        .expect("failed at add options");

    // Send request
    let response = request.send(&connection, "/")?;

    let mut device_class = String::new();
    let mut device_id = String::new();
    let mut device_info = String::new();
    let mut device_make_model = String::new();
    let mut device_location = String::new();

    // Parse the result
    for attr in response.attributes() {
        let name = match attr.name() {
            Some(n) => n,
            None => continue,
        };
        let value = attr.get_string(0).unwrap_or_default();

        match name.as_str() {
            "device-class" => device_class = value,
            "device-id" => device_id = value,
            "device-info" => device_info = value,
            "device-make-and-model" => device_make_model = value,
            "device-location" => device_location = value,
            "device-uri" => {
                if args.long_status {
                    println!("Device: uri = {}", value);
                    println!("        class = {}", device_class);
                    println!("        info = {}", device_info);
                    println!("        make-and-model = {}", device_make_model);
                    println!("        device-id = {}", device_id);
                    println!("        location = {}", device_location);
                } else {
                    println!("{} {}", device_class, value);
                }
                device_class.clear();
                device_id.clear();
                device_info.clear();
                device_make_model.clear();
                device_location.clear();
            }
            _ => {}
        }
    }

    Ok(())
}

/// Handles `-m` flag: lists the available drivers.
fn show_models(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let connection = scheduler_connection()?;

    // // Create CUPS_GET_PPDS ipp request using raw operation code
    let mut request = IppRequest::new_raw(ipp_op_e_IPP_OP_CUPS_GET_PPDS as i32).expect("Failed");

    request.add_standard_attrs()?;

    // // Add filter attributes
    if let Some(ref device_id) = args.device_id {
        request.add_string(
            IppTag::Operation,
            IppValueTag::Text,
            "ppd-device-id",
            device_id,
        )?;
    }

    if let Some(ref language) = args.language {
        request.add_string(
            IppTag::Operation,
            IppValueTag::Language,
            "ppd-language",
            language,
        )?;
    }

    if let Some(ref make_model) = args.make_model {
        request.add_string(
            IppTag::Operation,
            IppValueTag::Text,
            "ppd-make-and-model",
            make_model,
        )?;
    }

    if let Some(ref product) = args.product {
        request.add_string(IppTag::Operation, IppValueTag::Text, "ppd-product", product)?;
    }

    // Add include/exclude schemes if specified
    add_cups_options(&mut request, &args.include_schemes, &args.exclude_schemes)?;

    // Send request
    let response = request.send(&connection, "/")?;

    let mut ppd_name = String::new();
    let mut ppd_lang = String::new();
    let mut ppd_make_model = String::new();
    let mut ppd_device_id = String::new();

    let flush = |args: &Args, name: &str, lang: &str, make_model: &str, device_id: &str| {
        if name.is_empty() {
            return;
        }
        let lang = if lang.is_empty() { "en" } else { lang };
        if args.long_status {
            println!("Model:  name = {}", name);
            println!("        natural_language = {}", lang);
            println!("        make-and-model = {}", make_model);
            println!("        device-id = {}", device_id);
        } else {
            println!("{} {}", name, make_model);
        }
    };

    for attr in response.attributes() {
        let name = match attr.name() {
            Some(n) => n,
            None => continue,
        };
        let value = attr.get_string(0).unwrap_or_default();

        match name.as_str() {
            "ppd-name" => {
                flush(args, &ppd_name, &ppd_lang, &ppd_make_model, &ppd_device_id);
                ppd_name = value;
                ppd_lang.clear();
                ppd_make_model.clear();
                ppd_device_id.clear();
            }
            "ppd-natural-language" => ppd_lang = value,
            "ppd-make-and-model" => ppd_make_model = value,
            "ppd-device-id" => ppd_device_id = value,
            _ => {}
        }
    }

    flush(args, &ppd_name, &ppd_lang, &ppd_make_model, &ppd_device_id);

    // Always show "everywhere" model if not excluded
    let include_everywhere = args
        .include_schemes
        .as_ref()
        .map(|s| s.contains("everywhere"))
        .unwrap_or(true);
    let exclude_everywhere = args
        .exclude_schemes
        .as_ref()
        .map(|s| s.contains("everywhere"))
        .unwrap_or(false);

    if include_everywhere && !exclude_everywhere {
        if args.long_status {
            println!("Model:  name = everywhere");
            println!("        natural_language = en");
            println!("        make-and-model = IPP Everywhere™");
            println!("        device-id = CMD:PwgRaster");
        } else {
            println!("everywhere IPP Everywhere");
        }
    }

    Ok(())
}

/// Get a `HttpConnection` to the scheduler
fn scheduler_connection() -> Result<HttpConnection, Box<dyn std::error::Error>> {
    let dest = match get_default_destination() {
        Ok(d) => d,
        Err(_) => {
            let list = get_all_destinations()?;
            list.into_iter().next().ok_or("No printers configured")?
        }
    };

    Ok(dest.connect(ConnectionFlags::Scheduler, Some(30000), None)?)
}

fn add_cups_options(
    request: &mut IppRequest,
    include: &Option<String>,
    exclude: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut opts = Vec::new();

    if let Some(schemes) = include {
        opts.push(("include-schemes".to_string(), schemes.clone()));
    };

    if let Some(schemes) = exclude {
        opts.push(("exclude-schemes".to_string(), schemes.clone()));
    }

    if !opts.is_empty() {
        encode_options_with_group(request.as_ptr(), &opts, ipp_tag_e_IPP_TAG_OPERATION)?;
    }

    Ok(())
}
