use rfd::FileDialog;
use std::io::Write;

use crate::types::{Region, Unit};

// <csv export>
pub fn export_csv(regions: &[Region], unit: &Unit) -> String {
    let Some(path) = FileDialog::new()
        .set_file_name("regions.csv")
        .add_filter("CSV", &["csv"])
        .save_file()
    else {
        return "Export cancelled.".into();
    };

    match std::fs::File::create(&path) {
        Ok(mut f) => {
            let _ = writeln!(f, "Region,Pixels,Area ({}),Avg R,Avg G,Avg B", unit.label());
            for r in regions {
                let _ = writeln!(f, "{},{},{:.4},{},{},{}",
                                 r.index, r.pixel_count, r.area_cm2 * unit.factor(),
                                 r.avg_color[0], r.avg_color[1], r.avg_color[2]);
            }
            format!("Exported to {}", path.display())
        }
        Err(e) => format!("Export failed: {e}"),
    }
}
// </csv export>
