use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

const MINIO_BUCKET: &str = "rustpill-firmwares";

type DynError = Box<dyn std::error::Error>;

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    dotenvy::dotenv().ok();
    let task = env::args().nth(1);
    let args = env::args().skip(2).collect::<Vec<_>>();

    match (task.as_deref(), args.as_slice()) {
        (Some("flash"), [binary]) => flash(binary)?,
        (Some("pygen"), _) => build_bindings()?,
        (Some("publish"), _) => publish()?,
        _ => print_help(),
    }
    Ok(())
}

fn print_help() {
    eprintln!(
        "Tasks:

flash <name>            flashes the firmware binary to the device
pygen                   generates the Python bindings
publish                 publishes the Python bindings to PyPI
"
    )
}

fn flash(binary: &str) -> Result<(), DynError> {
    build_firmware(Some(binary))?;

    let target_bin = project_root()
        .join("target")
        .join("thumbv7m-none-eabi")
        .join("release")
        .join(binary);

    let mut cmd = Command::new("probe-rs");

    cmd.arg("run")
        .arg("--chip=STM32F103C8")
        .arg("--protocol")
        .arg("swd")
        .arg(target_bin);

    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("Failed to flash: {}", status).into());
    }

    Ok(())
}

fn build_firmware(firmware_name: Option<&str>) -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(cargo);

    let defmt_log = env::var("DEFMT_LOG").unwrap_or_else(|_| "info".to_string());

    cmd.current_dir(project_root())
        .env("DEFMT_LOG", defmt_log)
        .arg("build")
        .arg("-p")
        .arg("firmware")
        .arg("--release");

    if let Some(name) = firmware_name {
        cmd.arg("--bin").arg(name);
    }

    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("Failed to build firmware: {}", status).into());
    }
    Ok(())
}

fn publish() -> Result<(), DynError> {
    build_firmware(None)?;
    upload_firmwares()?;
    build_stubs()?;

    let mut pycmd = pycmd();

    pycmd.arg("maturin").arg("publish").arg("--no-sdist");

    let status = pycmd.status()?;
    if !status.success() {
        return Err(format!("Failed to build bindings: {}", status).into());
    }
    Ok(())
}

fn build_bindings() -> Result<(), DynError> {
    build_stubs()?;

    let mut pycmd = pycmd();

    pycmd.arg("maturin").arg("develop");

    let status = pycmd.status()?;
    if !status.success() {
        return Err(format!("Failed to build bindings: {}", status).into());
    }
    Ok(())
}

fn build_stubs() -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(cargo);

    cmd.current_dir(project_root());

    cmd.arg("run")
        .arg("-p")
        .arg("host")
        .arg("--release")
        .arg("--bin=stub_gen");

    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("Failed to build type stubs: {}", status).into());
    }
    Ok(())
}

fn upload_firmwares() -> Result<(), DynError> {
    let compiled_firmware_dir = project_root()
        .join("target")
        .join("thumbv7m-none-eabi")
        .join("release");

    let firmware_bin_dir = project_root().join("firmware").join("src").join("bin");

    let minio_endpoint = env::var("MINIO_ENDPOINT").expect("MINIO_ENDPOINT not set in .env");
    let access_key_id =
        env::var("MINIO_ACCESS_KEY_ID").expect("MINIO_ACCESS_KEY_ID not set in .env");
    let secret_access_key =
        env::var("MINIO_SECRET_ACCESS_KEY").expect("MINIO_SECRET_ACCESS_KEY not set in .env");

    let region = s3::Region::Custom {
        region: "eu-central-1".to_string(),
        endpoint: minio_endpoint,
    };
    let creds = s3::creds::Credentials::new(
        Some(&access_key_id),
        Some(&secret_access_key),
        None,
        None,
        None,
    )?;
    let mut bucket = s3::Bucket::new(MINIO_BUCKET, region, creds)?;
    bucket.set_path_style();
    if !bucket.exists()? {
        return Err(format!("Bucket {} does not exist", MINIO_BUCKET).into());
    }

    for firmware_name in std::fs::read_dir(&firmware_bin_dir)?
        .filter_map(Result::ok)
        .map(|e| {
            e.path()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(String::from)
        })
        .flatten()
    {
        upload_to_s3(
            bucket.clone(),
            &compiled_firmware_dir,
            &firmware_name,
            "stm32f103c8",
        )?;
    }

    Ok(())
}

fn upload_to_s3(
    bucket: Box<s3::Bucket>,
    source_dir: &Path,
    firmware_name: &str,
    chip_type: &str,
) -> Result<(), DynError> {
    let s3_key = [chip_type, firmware_name, env!("CARGO_PKG_VERSION")].join("/");

    let content = std::fs::read(source_dir.join(firmware_name))?;

    bucket.put_object(&s3_key, &content)?;
    println!(
        "Uploaded {} to s3://{}/{}",
        firmware_name,
        bucket.name(),
        s3_key
    );

    Ok(())
}

fn pycmd() -> Command {
    let mut cmd = Command::new("uv");
    cmd.current_dir(project_root().join("host"));
    cmd.arg("run");
    cmd
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
