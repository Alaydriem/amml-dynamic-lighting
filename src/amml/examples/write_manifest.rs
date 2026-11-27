//! Rewrites the `uses` block of `amml.hcl` from `DynamicLightingMod::USES`. Run it through `mise run manifest`.

use std::path::Path;

use alaydriem_dynamic_lighting::DynamicLightingMod;
use amml_api::AmmlMod;

struct ManifestWriter;

impl ManifestWriter {
    fn run() -> std::io::Result<()> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("amml.hcl");
        let manifest = std::fs::read_to_string(&path)?;
        let written = DynamicLightingMod::USES.write_into(&manifest);
        std::fs::write(&path, written)?;
        println!("wrote the uses block of {} for {}", path.display(), DynamicLightingMod::id());
        Ok(())
    }
}

fn main() -> std::process::ExitCode {
    match ManifestWriter::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("write_manifest failed: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
