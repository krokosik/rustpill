use std::path::Path;

use anyhow::anyhow;
use defmt_decoder::{
    DecodeError, Frame, Locations, Table,
    log::{
        DefmtLoggerType,
        format::{Formatter, FormatterConfig, HostFormatter},
    },
};
use tokio::{fs, sync::mpsc};

pub async fn run_decoder<P>(
    bin_path: P,
    mut rx: mpsc::UnboundedReceiver<Vec<u8>>,
) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let bytes = fs::read(bin_path).await?;
    let table = Table::parse(&bytes)?.ok_or_else(|| anyhow!(".defmt data not found"))?;
    let locs = table.get_locations(&bytes)?;

    // check if the locations info contains all the indicies
    let locs = if table.indices().all(|idx| locs.contains_key(&(idx as u64))) {
        Some(locs)
    } else {
        log::warn!("(BUG) location info is incomplete; it will be omitted from the output");
        None
    };

    let logger_type = DefmtLoggerType::Stdout;
    let mut formatter_config = FormatterConfig::default().with_location();
    formatter_config.is_timestamp_available = table.has_timestamp();

    let host_formatter_config = FormatterConfig::default().with_location();

    let formatter = Formatter::new(formatter_config);
    let host_formatter = HostFormatter::new(host_formatter_config);

    defmt_decoder::log::init_logger(formatter, host_formatter, logger_type, |_| true);

    let mut stream_decoder = table.new_stream_decoder();
    let current_dir = std::env::current_dir()?;

    while let Some(data) = rx.recv().await {
        if data.is_empty() {
            break;
        }

        stream_decoder.received(&data);

        loop {
            match stream_decoder.decode() {
                Ok(frame) => forward_to_logger(&frame, location_info(&locs, &frame, &current_dir)),
                Err(DecodeError::UnexpectedEof) => break,
                Err(DecodeError::Malformed) => match table.encoding().can_recover() {
                    false => return Err(DecodeError::Malformed.into()),
                    true => {
                        log::warn!("(HOST) malformed frame, skipping");
                        log::warn!("└─ {} @ {}:{}", env!("CARGO_PKG_NAME"), file!(), line!());
                        continue;
                    }
                },
            }
        }
    }

    Ok(())
}

type LocationInfo = (Option<String>, Option<u32>, Option<String>);

fn forward_to_logger(frame: &Frame, location_info: LocationInfo) {
    let (file, line, mod_path) = location_info;
    defmt_decoder::log::log_defmt(frame, file.as_deref(), line, mod_path.as_deref());
}

fn location_info(locs: &Option<Locations>, frame: &Frame, current_dir: &Path) -> LocationInfo {
    let (mut file, mut line, mut mod_path) = (None, None, None);

    let loc = locs.as_ref().map(|locs| locs.get(&frame.index()));

    if let Some(Some(loc)) = loc {
        // try to get the relative path, else the full one
        let path = loc.file.strip_prefix(current_dir).unwrap_or(&loc.file);

        file = Some(path.display().to_string());
        line = Some(loc.line as u32);
        mod_path = Some(loc.module.clone());
    }

    (file, line, mod_path)
}
