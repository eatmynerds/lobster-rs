pub mod vidcloud;

pub trait VideoExtractor {
    async fn extract(&mut self, video_url: &str) -> anyhow::Result<()>;
}
