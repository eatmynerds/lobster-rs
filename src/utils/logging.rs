use chrono::Local;
use colored::Colorize;
use regex::Regex;
use std::fmt::Write;
use term_size;
use tracing::{Level, Metadata};
use tracing_subscriber::{
    layer::{Context, Filter},
    Layer,
};

pub struct CustomLayer;
struct PrintlnVisitor {
    buffer: String,
}

impl<S> Filter<S> for CustomLayer {
    fn enabled(&self, metadata: &Metadata<'_>, _: &Context<'_, S>) -> bool {
        metadata.level() <= &Level::TRACE
    }
}

impl<S> Layer<S> for CustomLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let timestamp = Local::now().format("%H:%M:%S").to_string();
        let timestamp = format!("[{}]", timestamp).truecolor(150, 150, 150);

        let level = match *event.metadata().level() {
            Level::ERROR => "ERROR".black().bold().on_bright_red(),
            Level::WARN => "WARNING".bright_red(),
            Level::INFO => "INFO".truecolor(123, 124, 188),
            Level::DEBUG => "DEBUG".truecolor(156, 179, 91),
            _ => todo!(),
        };

        let target = format!(
            "{}.rs:{}",
            event.metadata().target(),
            event.metadata().line().unwrap_or(0)
        )
        .truecolor(150, 150, 150)
        .bold();

        let mut visitor = PrintlnVisitor {
            buffer: String::new(),
        };
        event.record(&mut visitor);

        let message = format!("{} {} {}", timestamp, level, visitor.buffer.trim_end());

        let ansi_regex = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
        let stripped_message = ansi_regex.replace_all(&message, "");
        let visible_message_length = stripped_message.len();

        if let Some((width, _)) = term_size::dimensions() {
            let target_with_brackets = format!("[{}]", target);
            let target_length = target_with_brackets.len();

            let max_message_length = width.saturating_sub(target_length + 1);

            let truncated_message = if visible_message_length > max_message_length {
                let excess = visible_message_length - max_message_length;
                let truncated_str = stripped_message
                    .chars()
                    .take(stripped_message.len() - excess)
                    .collect::<String>();
                format!("{}...", truncated_str)
            } else {
                message.clone()
            };

            let truncated_message_length = ansi_regex.replace_all(&truncated_message, "").len();
            let padding = max_message_length.saturating_sub(truncated_message_length);
            println!(
                "{}{}{}",
                truncated_message,
                " ".repeat(padding),
                target_with_brackets
            );
        } else {
            println!("{} [{}]", message, target);
        }
    }
}

impl tracing::field::Visit for PrintlnVisitor {
    fn record_f64(&mut self, _: &tracing::field::Field, value: f64) {
        write!(&mut self.buffer, "{}", value).unwrap();
    }

    fn record_i64(&mut self, _: &tracing::field::Field, value: i64) {
        write!(&mut self.buffer, "{}", value).unwrap();
    }

    fn record_u64(&mut self, _: &tracing::field::Field, value: u64) {
        write!(&mut self.buffer, "{}", value).unwrap();
    }

    fn record_bool(&mut self, _: &tracing::field::Field, value: bool) {
        write!(&mut self.buffer, "{}", value).unwrap();
    }

    fn record_str(&mut self, _: &tracing::field::Field, value: &str) {
        write!(&mut self.buffer, "{}", value).unwrap();
    }

    fn record_error(
        &mut self,
        _: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        write!(&mut self.buffer, "{}", value).unwrap();
    }

    fn record_debug(&mut self, _: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        write!(&mut self.buffer, "{:?}", value).unwrap();
    }
}

