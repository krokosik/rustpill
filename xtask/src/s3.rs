use std::{env, error::Error, path::Path};

const MINIO_BUCKET: &str = "rustpill-firmwares";

pub fn get_bucket() -> Result<Box<s3::Bucket>, Box<dyn Error>> {
    let minio_endpoint = env::var("MINIO_ENDPOINT").unwrap_or("https://s3.qodl.eu".to_string());
    let access_key_id = env::var("MINIO_ACCESS_KEY_ID").ok();
    let secret_access_key = env::var("MINIO_SECRET_ACCESS_KEY").ok();

    let region = s3::Region::Custom {
        region: "eu-central-1".to_string(),
        endpoint: minio_endpoint,
    };

    let mut bucket = if access_key_id.is_none() || secret_access_key.is_none() {
        Box::new(s3::Bucket::new_public(MINIO_BUCKET, region)?)
    } else {
        let creds = s3::creds::Credentials::new(
            access_key_id.as_deref(),
            secret_access_key.as_deref(),
            None,
            None,
            None,
        )?;
        s3::Bucket::new(MINIO_BUCKET, region, creds)?
    };
    bucket.set_path_style();
    Ok(bucket)
}

pub fn upload_to_s3(
    bucket: Box<s3::Bucket>,
    source_dir: &Path,
    firmware_name: &str,
    chip_type: &str,
) -> Result<(), Box<dyn Error>> {
    let s3_key = [chip_type, env!("CARGO_PKG_VERSION"), firmware_name].join("/");

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
