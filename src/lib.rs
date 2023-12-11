mod cornucopia;
mod routes;

pub mod authentication;
pub mod configuration;
pub mod connection_pool;
pub mod domain;
pub mod email_client;
pub mod html_template_gen;
pub mod startup;
pub mod telemetry;
pub mod validation;

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}
