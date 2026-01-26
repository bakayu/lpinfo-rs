use clap::Parser;
use cups_rs::bindings::{
    cupsDoRequest, cupsLastErrorString, ipp_op_e_IPP_OP_CUPS_GET_DEVICES,
    ipp_op_e_IPP_OP_CUPS_GET_PPDS, ipp_status_e_IPP_STATUS_OK,
    ipp_status_e_IPP_STATUS_OK_CONFLICTING, ipp_status_e_IPP_STATUS_OK_IGNORED_OR_SUBSTITUTED,
    ipp_tag_e_IPP_TAG_CHARSET, ipp_tag_e_IPP_TAG_INTEGER, ipp_tag_e_IPP_TAG_KEYWORD,
    ipp_tag_e_IPP_TAG_LANGUAGE, ipp_tag_e_IPP_TAG_NAME, ipp_tag_e_IPP_TAG_OPERATION,
    ipp_tag_e_IPP_TAG_TEXT, ippAddInteger, ippAddString, ippDelete, ippFirstAttribute, ippGetName,
    ippGetStatusCode, ippGetString, ippNewRequest, ippNextAttribute,
};
use cups_rs::config::{EncryptionMode, set_encryption, set_server};
use cups_rs::connection::HttpConnection;
use cups_rs::{ConnectionFlags, get_all_destinations, get_default_destination};

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
    let request = unsafe { ippNewRequest(ipp_op_e_IPP_OP_CUPS_GET_DEVICES as i32) };

    if request.is_null() {
        return Err("Failed to create IPP request".into());
    }

    // add required IPP op attributes (matching libcups defaults)
    add_standard_ipp_attrs(request)?;

    // Add timeout attribute
    let timeout_name = std::ffi::CString::new("timeout")?;
    unsafe {
        ippAddInteger(
            request,
            ipp_tag_e_IPP_TAG_OPERATION,
            ipp_tag_e_IPP_TAG_INTEGER,
            timeout_name.as_ptr(),
            args.timeout,
        );
    }

    // Add include/exclude schemes if specified
    if let Some(ref schemes) = args.include_schemes {
        let name = std::ffi::CString::new("include-schemes")?;
        let value = std::ffi::CString::new(schemes.as_str())?;
        unsafe {
            ippAddString(
                request,
                ipp_tag_e_IPP_TAG_OPERATION,
                ipp_tag_e_IPP_TAG_KEYWORD,
                name.as_ptr(),
                std::ptr::null(),
                value.as_ptr(),
            );
        }
    }

    if let Some(ref schemes) = args.exclude_schemes {
        let name = std::ffi::CString::new("exclude-schemes")?;
        let value = std::ffi::CString::new(schemes.as_str())?;
        unsafe {
            ippAddString(
                request,
                ipp_tag_e_IPP_TAG_OPERATION,
                ipp_tag_e_IPP_TAG_KEYWORD,
                name.as_ptr(),
                std::ptr::null(),
                value.as_ptr(),
            );
        }
    }

    // Send request
    let response = send_ipp_request(request, &connection, "CUPS-GET-DEVICES")?;

    // Parse the result
    let mut attr = unsafe { ippFirstAttribute(response) };

    let mut device_class = String::new();
    let mut device_id = String::new();
    let mut device_info = String::new();
    let mut device_make_model = String::new();
    let mut device_location = String::new();

    while !attr.is_null() {
        let name = unsafe {
            let ptr = ippGetName(attr);
            if ptr.is_null() {
                attr = ippNextAttribute(response);
                continue;
            }
            std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
        };

        let value = unsafe {
            let ptr = ippGetString(attr, 0, std::ptr::null_mut());
            if !ptr.is_null() {
                std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
            } else {
                String::new()
            }
        };

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

        attr = unsafe { ippNextAttribute(response) };
    }

    unsafe { ippDelete(response) };

    Ok(())
}

/// Handles `-m` flag: lists the available drivers.
fn show_models(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let connection = scheduler_connection()?;

    // Create CUPS_GET_PPDS ipp request using raw operation code
    let request = unsafe { ippNewRequest(ipp_op_e_IPP_OP_CUPS_GET_PPDS as i32) };

    if request.is_null() {
        return Err("Failed to create IPP request".into());
    }

    add_standard_ipp_attrs(request)?;

    // Add filter attributes
    if let Some(ref device_id) = args.device_id {
        let name = std::ffi::CString::new("ppd-device-id")?;
        let value = std::ffi::CString::new(device_id.as_str())?;
        unsafe {
            ippAddString(
                request,
                ipp_tag_e_IPP_TAG_OPERATION,
                ipp_tag_e_IPP_TAG_TEXT,
                name.as_ptr(),
                std::ptr::null(),
                value.as_ptr(),
            );
        }
    }

    if let Some(ref language) = args.language {
        let name = std::ffi::CString::new("ppd-language")?;
        let value = std::ffi::CString::new(language.as_str())?;
        unsafe {
            ippAddString(
                request,
                ipp_tag_e_IPP_TAG_OPERATION,
                ipp_tag_e_IPP_TAG_LANGUAGE,
                name.as_ptr(),
                std::ptr::null(),
                value.as_ptr(),
            );
        }
    }

    if let Some(ref make_model) = args.make_model {
        let name = std::ffi::CString::new("ppd-make-and-model")?;
        let value = std::ffi::CString::new(make_model.as_str())?;
        unsafe {
            ippAddString(
                request,
                ipp_tag_e_IPP_TAG_OPERATION,
                ipp_tag_e_IPP_TAG_TEXT,
                name.as_ptr(),
                std::ptr::null(),
                value.as_ptr(),
            );
        }
    }

    if let Some(ref product) = args.product {
        let name = std::ffi::CString::new("ppd-product")?;
        let value = std::ffi::CString::new(product.as_str())?;
        unsafe {
            ippAddString(
                request,
                ipp_tag_e_IPP_TAG_OPERATION,
                ipp_tag_e_IPP_TAG_TEXT,
                name.as_ptr(),
                std::ptr::null(),
                value.as_ptr(),
            );
        }
    }

    if let Some(ref schemes) = args.include_schemes {
        let name = std::ffi::CString::new("include-schemes")?;
        let value = std::ffi::CString::new(schemes.as_str())?;
        unsafe {
            ippAddString(
                request,
                ipp_tag_e_IPP_TAG_OPERATION,
                ipp_tag_e_IPP_TAG_KEYWORD,
                name.as_ptr(),
                std::ptr::null(),
                value.as_ptr(),
            );
        }
    }

    if let Some(ref schemes) = args.exclude_schemes {
        let name = std::ffi::CString::new("exclude-schemes")?;
        let value = std::ffi::CString::new(schemes.as_str())?;
        unsafe {
            ippAddString(
                request,
                ipp_tag_e_IPP_TAG_OPERATION,
                ipp_tag_e_IPP_TAG_KEYWORD,
                name.as_ptr(),
                std::ptr::null(),
                value.as_ptr(),
            );
        }
    }

    // Send request
    let response = send_ipp_request(request, &connection, "CUPS-GET-PPDS")?;

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

    // Parse result
    let mut attr = unsafe { ippFirstAttribute(response) };
    while !attr.is_null() {
        let name = unsafe {
            let ptr = ippGetName(attr);
            if ptr.is_null() {
                attr = ippNextAttribute(response);
                continue;
            }
            std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
        };

        let value = unsafe {
            let ptr = ippGetString(attr, 0, std::ptr::null_mut());
            if !ptr.is_null() {
                std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
            } else {
                String::new()
            }
        };

        match name.as_str() {
            "ppd-name" => {
                // New record; flush previous one
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

        attr = unsafe { ippNextAttribute(response) };
    }

    // Flush last record
    flush(args, &ppd_name, &ppd_lang, &ppd_make_model, &ppd_device_id);

    unsafe { ippDelete(response) };

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

fn send_ipp_request(
    request: *mut cups_rs::bindings::_ipp_s,
    connection: &HttpConnection,
    op_name: &str,
) -> Result<*mut cups_rs::bindings::_ipp_s, Box<dyn std::error::Error>> {
    let resource = std::ffi::CString::new("/")?;
    let response = unsafe { cupsDoRequest(connection.as_ptr(), request, resource.as_ptr()) };

    if response.is_null() {
        return Err(format!("lpinfo: {}", cups_last_error()).into());
    }

    let status = unsafe { ippGetStatusCode(response) };
    if !ipp_is_success(status) {
        unsafe { ippDelete(response) };
        return Err(format!("lpinfo: {} failed: {}", op_name, cups_last_error()).into());
    }

    Ok(response)
}

/// Check if response to the ipp request was a success
fn ipp_is_success(status: i32) -> bool {
    status == ipp_status_e_IPP_STATUS_OK
        || status == ipp_status_e_IPP_STATUS_OK_IGNORED_OR_SUBSTITUTED
        || status == ipp_status_e_IPP_STATUS_OK_CONFLICTING
}

fn add_standard_ipp_attrs(
    request: *mut cups_rs::bindings::_ipp_s,
) -> Result<(), Box<dyn std::error::Error>> {
    let charset = std::ffi::CString::new("utf-8")?;
    let language = std::ffi::CString::new("en")?;
    let user = std::ffi::CString::new(env::var("USER").unwrap_or_else(|_| "unknown".into()))?;

    let name_charset = std::ffi::CString::new("attributes-charset")?;
    let name_language = std::ffi::CString::new("attributes-natural-language")?;
    let name_user = std::ffi::CString::new("requesting-user-name")?;

    unsafe {
        ippAddString(
            request,
            ipp_tag_e_IPP_TAG_OPERATION,
            ipp_tag_e_IPP_TAG_CHARSET,
            name_charset.as_ptr(),
            std::ptr::null(),
            charset.as_ptr(),
        );
        ippAddString(
            request,
            ipp_tag_e_IPP_TAG_OPERATION,
            ipp_tag_e_IPP_TAG_LANGUAGE,
            name_language.as_ptr(),
            std::ptr::null(),
            language.as_ptr(),
        );
        ippAddString(
            request,
            ipp_tag_e_IPP_TAG_OPERATION,
            ipp_tag_e_IPP_TAG_NAME,
            name_user.as_ptr(),
            std::ptr::null(),
            user.as_ptr(),
        );
    }

    Ok(())
}

/// Returns last cups error string
fn cups_last_error() -> String {
    unsafe {
        std::ffi::CStr::from_ptr(cupsLastErrorString())
            .to_string_lossy()
            .into_owned()
    }
}
