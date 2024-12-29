use crate::CLIENT;

pub fn generate_desktop(
    media_title: String,
    media_id: String,
    image_path: String,
) -> anyhow::Result<()> {
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
        std::fs::create_dir(&image_preview_dir)?;
    }

    let desktop_file = image_preview_dir.join(format!("{}.desktop", media_id.replace("/", "-")));

    std::fs::write(desktop_file, desktop_entry)?;

    Ok(())
}

pub fn remove_desktop_and_tmp(media_id: String) -> anyhow::Result<()> {
    let image_preview_dir = dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".local/share/applications/imagepreview");

    let desktop_file = image_preview_dir.join(format!("{}.desktop", media_id.replace("/", "-")));

    if desktop_file.exists() {
        std::fs::remove_file(desktop_file)?;
    }

    if std::fs::exists("/tmp/images")? {
        std::fs::remove_dir_all("/tmp/images")?;
    }

    Ok(())
}

pub async fn image_preview(
    images: &Vec<(String, String, String)>,
) -> anyhow::Result<Vec<(String, String, String)>> {
    if std::fs::exists("/tmp/images")? {
        std::fs::remove_dir_all("/tmp/images")?;
    }

    std::fs::create_dir_all("/tmp/images").expect("Failed to image cache directory");
    let mut temp_images: Vec<(String, String, String)> = vec![];

    for (media_name, image_url, media_id) in images.iter() {
        let image_bytes = CLIENT
            .get(image_url.to_string())
            .send()
            .await?
            .bytes()
            .await?;

        image::load_from_memory(&image_bytes)
            .unwrap()
            .save(format!("/tmp/images/{}.jpg", media_id.replace("/", "-")))?;

        temp_images.push((
            media_name.to_string(),
            media_id.to_string(),
            format!("/tmp/images/{}.jpg", media_id.replace("/", "-")),
        ));
    }

    Ok(temp_images)
}
