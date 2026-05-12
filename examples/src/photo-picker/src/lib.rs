#![allow(unused_imports)]

use pax_kit::*;

const MAX_VISIBLE_PHOTOS: usize = 4;
const PHOTO_ROW_HEIGHT: f64 = 112.0;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub photos: Property<Vec<PickedPhoto>>,
    pub photo_count: Property<usize>,
    pub status_text: Property<String>,
    pub detail_text: Property<String>,
}

#[pax]
pub struct PickedPhoto {
    pub id: String,
    pub name: String,
    pub metadata: String,
    pub source: String,
    pub handle: String,
    pub has_preview: bool,
    pub y: f64,
}

impl Example {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.photos.set(Vec::new());
        self.photo_count.set(0);
        self.status_text.set("No photos selected.".to_string());
        self.detail_text
            .set("Library selection supports multiple images.".to_string());
    }

    pub fn handle_picker_change(&mut self, _ctx: &NodeContext, event: Event<PhotoPickerChange>) {
        let selected_count = event.photos.len();
        let total_bytes: u64 = event.photos.iter().map(|photo| photo.byte_size).sum();
        let copied_bytes: usize = event
            .photos
            .iter()
            .map(|photo| photo.data.as_ref().map(|data| data.len()).unwrap_or(0))
            .sum();

        self.photo_count.set(selected_count);
        self.status_text.set(status_message(
            &event.status,
            selected_count,
            event.message.as_deref(),
        ));

        let shown_count = selected_count.min(MAX_VISIBLE_PHOTOS);
        let hidden_suffix = if selected_count > MAX_VISIBLE_PHOTOS {
            format!("; showing first {}", MAX_VISIBLE_PHOTOS)
        } else {
            String::new()
        };
        self.detail_text.set(format!(
            "request {} | {} selected | {} copied{}",
            event.request_id,
            format_bytes(total_bytes),
            format_bytes(copied_bytes as u64),
            hidden_suffix
        ));

        let rows = event
            .photos
            .iter()
            .take(shown_count)
            .enumerate()
            .map(|(index, photo)| PickedPhoto {
                id: photo.temp_id.clone(),
                name: photo
                    .file_name
                    .clone()
                    .unwrap_or_else(|| format!("photo {}", index + 1)),
                metadata: format!(
                    "{} | {}",
                    dimensions_label(photo.width, photo.height),
                    format_bytes(photo.byte_size)
                ),
                source: format!("source: {}", source_label(&photo.source_kind)),
                handle: photo.handle.clone().unwrap_or_default(),
                has_preview: photo.handle.is_some(),
                y: index as f64 * PHOTO_ROW_HEIGHT,
            })
            .collect();
        self.photos.set(rows);
    }
}

fn status_message(
    status: &PhotoPickerStatus,
    selected_count: usize,
    message: Option<&str>,
) -> String {
    let mut output = match status {
        PhotoPickerStatus::Selected => {
            if selected_count == 1 {
                "Selected 1 photo.".to_string()
            } else {
                format!("Selected {} photos.", selected_count)
            }
        }
        PhotoPickerStatus::Cancelled => "Picker cancelled.".to_string(),
        PhotoPickerStatus::PermissionDenied => "Permission denied.".to_string(),
        PhotoPickerStatus::Unavailable => "Requested source unavailable.".to_string(),
        PhotoPickerStatus::SizeLimitExceeded => "Photo exceeds configured byte limit.".to_string(),
        PhotoPickerStatus::Failed => "Picker failed.".to_string(),
    };

    if let Some(message) = message {
        if !message.is_empty() {
            output.push(' ');
            output.push_str(message);
        }
    }

    output
}

fn source_label(source: &PhotoPickerSourceKind) -> String {
    match source {
        PhotoPickerSourceKind::Library => "library".to_string(),
        PhotoPickerSourceKind::File => "file".to_string(),
        PhotoPickerSourceKind::Camera => "camera".to_string(),
        PhotoPickerSourceKind::Other(value) => value.clone(),
    }
}

fn dimensions_label(width: Option<u32>, height: Option<u32>) -> String {
    match (width, height) {
        (Some(width), Some(height)) => format!("{} x {} px", width, height),
        _ => "dimensions unknown".to_string(),
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;

    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / MB)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / KB)
    } else {
        format!("{} B", bytes)
    }
}
