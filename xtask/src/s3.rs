use std::{env, error::Error, path::Path};

const MINIO_BUCKET: &str = "rustpill-firmwares";

pub fn get_bucket() -> Result<Box<s3::Bucket>, Box<dyn Error>> {
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
    Ok(bucket)
}

pub fn upload_to_s3(
    bucket: Box<s3::Bucket>,
    source_dir: &Path,
    firmware_name: &str,
    chip_type: &str,
) -> Result<(), Box<dyn Error>> {
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
