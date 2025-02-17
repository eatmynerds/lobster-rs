use crate::CLIENT;
use log::{debug, error};

pub fn generate_desktop(
    media_title: String,
    media_id: String,
    image_path: String,
) -> anyhow::Result<()> {
    debug!("Generating desktop entry for media_id: {}", media_id);

    let desktop_entry = String::from(format!(
        r#"[Desktop Entry]
Name={}
Exec=echo %c
Icon={}
Type=Application
Categories=imagepreview;"#,
        media_title, image_path
    ));

    let image_preview_dir = dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".local/share/applications/imagepreview");

    if !image_preview_dir.exists() {
        debug!("Creating directory: {:?}", image_preview_dir);
        std::fs::create_dir(&image_preview_dir)?;
    }

    let desktop_file = image_preview_dir.join(format!("{}.desktop", media_id.replace("/", "-")));

    debug!("Writing desktop entry to file: {:?}", desktop_file);
    std::fs::write(&desktop_file, desktop_entry)?;

    debug!(
        "Desktop entry generated successfully for media_id: {}",
        media_id
    );

    Ok(())
}

pub fn remove_desktop_and_tmp(media_id: String) -> anyhow::Result<()> {
    debug!(
        "Removing desktop entry and temporary files for media_id: {}",
        media_id
    );

    let image_preview_dir = dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".local/share/applications/imagepreview");

    let desktop_file = image_preview_dir.join(format!("{}.desktop", media_id.replace("/", "-")));

    if desktop_file.exists() {
        debug!("Removing desktop file: {:?}", desktop_file);
        std::fs::remove_file(&desktop_file)?;
    } else {
        debug!("Desktop file does not exist: {:?}", desktop_file);
    }

    if std::fs::metadata("/tmp/images").is_ok() {
        debug!("Removing temporary images directory: /tmp/images");
        std::fs::remove_dir_all("/tmp/images")?;
    } else {
        debug!("Temporary images directory does not exist: /tmp/images");
    }

    debug!(
        "Desktop entry and temporary files removed successfully for media_id: {}",
        media_id
    );

    Ok(())
}

pub async fn image_preview(
    images: &Vec<(String, String, String)>,
) -> anyhow::Result<Vec<(String, String, String)>> {
    debug!(
        "Starting image preview generation for {} images.",
        images.len()
    );

    if std::fs::metadata("/tmp/images").is_ok() {
        debug!("Removing existing temporary images directory: /tmp/images");
        std::fs::remove_dir_all("/tmp/images")?;
    }

    debug!("Creating temporary images directory: /tmp/images");
    std::fs::create_dir_all("/tmp/images").expect("Failed to create image cache directory");

    let mut temp_images: Vec<(String, String, String)> = vec![];

    for (media_name, image_url, media_id) in images.iter() {
        debug!(
            "Downloading image for media_id: {} from URL: {}",
            media_id, image_url
        );

        let image_bytes = CLIENT
            .get(image_url.to_string())
            .send()
            .await?
            .bytes()
            .await?;

        let output_path = format!("/tmp/images/{}.jpg", media_id.replace("/", "-"));
        debug!("Saving image to: {}", output_path);

        match image::load_from_memory(&image_bytes) {
            Ok(image) => {
                image.save(&output_path)?;
                temp_images.push((media_name.to_string(), media_id.to_string(), output_path));
                debug!("Image saved successfully for media_id: {}", media_id);
            }
            Err(e) => {
                error!(
                    "Failed to process image for media_id: {}. Error: {}",
                    media_id, e
                );
                return Err(anyhow::anyhow!(e));
            }
        }
    }

    debug!("Image preview generation completed successfully.");

    Ok(temp_images)
}
