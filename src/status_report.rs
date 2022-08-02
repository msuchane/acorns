use std::default::Default;

use askama::Template;
use chrono::prelude::*;
use color_eyre::eyre::{Result, WrapErr};
use counter::Counter;

use crate::ticket_abstraction::AbstractTicket;

#[derive(Default)]
struct OverallProgress {
    all: u32,
    complete: u32,
    complete_pct: f32,
    warnings: u32,
    warnings_pct: f32,
    incomplete: u32,
    incomplete_pct: f32,
}

#[derive(Default)]
struct WriterStats<'a> {
    name: &'a str,
    total: u32,
    complete: u32,
    warnings: u32,
    incomplete: u32,
    percent: f32,
}

#[derive(Default)]
struct Checks {
    overall: Status,
    development: Status,
    title_and_text: Status,
}

enum Status {
    Ok,
    Warning(String),
    Error(String),
}

impl Default for Status {
    fn default() -> Self {
        Self::Ok
    }
}

impl Status {
    fn message(&self) -> &str {
        match self {
            Self::Ok => "OK",
            Self::Warning(message) | Self::Error(message) => message,
        }
    }
    fn color(&self) -> &'static str {
        match self {
            Self::Ok => "green",
            Self::Warning(_) => "orange",
            Self::Error(_) => "red",
        }
    }
}

impl AbstractTicket {
    fn checks(&self) -> Checks {
        Checks::default()
    }

    fn docs_contact_short(&self) -> &str {
        email_prefix(&self.docs_contact)
    }

    fn assignee_short(&self) -> &str {
        if let Some(assignee) = &self.assignee {
            email_prefix(assignee)
        } else {
            "No assignee"
        }
    }

    fn flags_or_labels(&self) -> String {
        // TODO: Maybe combine flags and labels together as one list?
        if let Some(flags) = &self.flags {
            flags.join(", ")
        } else if let Some(labels) = &self.labels {
            labels.join(", ")
        } else {
            "No flags or labels".to_string()
        }
    }

    fn display_target_releases(&self) -> String {
        if self.target_releases.is_empty() {
            "No releases".to_string()
        } else {
            self.target_releases.join(", ")
        }
    }

    fn display_subsystems(&self) -> String {
        if self.subsystems.is_empty() {
            "No subsystems".to_string()
        } else {
            self.subsystems.join(", ")
        }
    }

    fn display_components(&self) -> String {
        if self.components.is_empty() {
            "No components".to_string()
        } else {
            self.components.join(", ")
        }
    }
}

fn email_prefix(email: &str) -> &str {
    if let Some(prefix) = email.split('@').next() {
        prefix
    } else {
        email
    }
}

/// List the products set in the tickets, sorted from most common to least common.
/// Returns up to 3 most common products and ignores the rest.
fn combined_products(tickets: &[AbstractTicket]) -> Vec<&str> {
    let products: Counter<&str> = tickets
        .iter()
        .map(|ticket| ticket.product.as_str())
        .collect();

    products
        .k_most_common_ordered(3)
        .iter()
        .map(|(elem, _frequency)| *elem)
        .collect()
}

/// List the releases set in the tickets, sorted from most common to least common.
/// Returns up to 3 most common releases and ignores the rest.
fn combined_releases(tickets: &[AbstractTicket]) -> Vec<&str> {
    let mut releases: Counter<&str> = Counter::new();

    for ticket in tickets.iter() {
        releases.update(
            ticket
                .target_releases
                .iter()
                .map(String::as_str),
        );
    }

    releases
        .k_most_common_ordered(3)
        .iter()
        .map(|(elem, _frequency)| *elem)
        .collect()
}

/// Display the list of releases or products as a string.
/// If the list is empty, provide a placeholder instead.
fn list_or_placeholder(list: &[&str], name: &str) -> String {
    if list.is_empty() {
        format!("no {}", name)
    } else {
        list.join(", ")
    }
}

#[derive(Template)] // this will generate the code...
#[template(path = "status-table.html")] // using the template in this path, relative
                                        // to the `templates` dir in the crate root
struct StatusTableTemplate<'a> {
    products: &'a str,
    release: &'a str,
    overall_progress: OverallProgress,
    tickets: &'a [AbstractTicket],
    per_writer_stats: &'a [WriterStats<'a>],
    generated_date: &'a str,
}

pub fn analyze_status(tickets: &[AbstractTicket]) -> Result<String> {
    let products = combined_products(tickets);
    let products_display = list_or_placeholder(&products, "products");

    let releases = combined_releases(tickets);
    let releases_display = list_or_placeholder(&releases, "releases");

    let date_today = Utc::now().to_rfc2822();

    let status_table = StatusTableTemplate {
        products: &products_display,
        release: &releases_display,
        overall_progress: OverallProgress {
            ..Default::default()
        },
        per_writer_stats: &[],
        tickets,
        generated_date: &date_today,
    };

    status_table
        .render()
        .wrap_err("Failed to prepare the status table.")
}
