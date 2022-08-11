use std::convert::From;
use std::default::Default;

use askama::Template;
use chrono::prelude::*;
use color_eyre::eyre::{Result, WrapErr};
use counter::Counter;
use regex::Regex;

use crate::extra_fields::DocTextStatus;
use crate::ticket_abstraction::AbstractTicket;

/// These doc types don't belong to any particular target release.
/// Skip the release check for these.
const UNCHECKED_DOC_TYPES: [&str; 3] = [
    "known issue",
    "technology preview",
    "deprecated functionality",
];

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
    development: Status,
    doc_type: Status,
    doc_status: Status,
    title_and_text: Status,
    target_release: Status,
}

impl Checks {
    /// Present an overview of all the particular status checks:
    ///
    /// * If any check resulted in an error, return the list of all errors.
    /// * If any check resulted in a warning, return the list of all warnings.
    /// * If there are no errors or warnings, return `Ok`.
    fn overall(&self) -> Status {
        // All fields on `Checks`, so that we can iterate over them.
        let items = [
            &self.doc_type,
            &self.title_and_text,
            &self.doc_status,
            &self.development,
            &self.target_release,
        ];

        // Capture all errors.
        let errors: Vec<&str> = items
            .iter()
            .filter_map(|status| match status {
                Status::Error(e) => Some(e.as_str()),
                _ => None,
            })
            .collect();

        // Capture all warnings.
        let warnings: Vec<&str> = items
            .iter()
            .filter_map(|status| match status {
                Status::Error(e) => Some(e.as_str()),
                _ => None,
            })
            .collect();

        if !errors.is_empty() {
            Status::Error(errors.join(", "))
        } else if !warnings.is_empty() {
            Status::Warning(warnings.join(", "))
        } else {
            Status::Ok
        }
    }
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

    fn from_title(text: &str) -> Self {
        let first_content_line = text
            .lines()
            .find(|line| !line.trim().is_empty() || !line.starts_with("//"));

        if let Some(first_content_line) = first_content_line {
            let title_regex = Regex::new(r"\.\S+").unwrap();

            if title_regex.is_match(first_content_line) {
                Self::Ok
            } else {
                Self::Error("First line is not a title.".into())
            }
        } else {
            Self::Error("The release note is empty.".into())
        }
    }

    /// Report when the bug is in early stages of development.
    fn from_devel_status(status: &str) -> Self {
        match status.to_lowercase().as_str() {
            "to do" | "new" | "assigned" | "modified" => Self::Warning("Early development.".into()),
            _ => Self::Ok,
        }
    }

    /// Report if the doc type is set to a non-release note type.
    fn from_doc_type(doc_type: &str) -> Self {
        match doc_type {
            "If docs needed, set a value" => Self::Error("Bad doc type.".into()),
            _ => Self::Ok,
        }
    }

    /// Report if the ticket's target release doesn't match the the global target release.
    fn from_target_release(
        ticket_releases: &[String],
        likely_release: Option<&&str>,
        doc_type: &str,
    ) -> Self {
        if let Some(&likely_release) = likely_release {
            // TODO: This is an awkward way to compare &str with String. Revisit.
            if ticket_releases.contains(&likely_release.to_string())
                || UNCHECKED_DOC_TYPES.contains(&doc_type.to_lowercase().as_str())
            {
                Self::Ok
            } else {
                Self::Warning("Check target release.".into())
            }
        } else {
            Self::Ok
        }
    }
}

impl From<DocTextStatus> for Status {
    fn from(item: DocTextStatus) -> Self {
        match item {
            DocTextStatus::Approved => Self::Ok,
            DocTextStatus::InProgress => Self::Error("Release note not approved.".into()),
            DocTextStatus::NoDocumentation => Self::Error("Release note disabled.".into()),
        }
    }
}

impl AbstractTicket {
    /// Analyze the release note status of the ticket. Record the analysis as `Checks`.
    fn checks(&self, releases: &[&str]) -> Checks {
        Checks {
            development: Status::from_devel_status(&self.status),
            title_and_text: Status::from_title(&self.doc_text),
            doc_type: Status::from_doc_type(&self.doc_type),
            doc_status: Status::from(self.doc_text_status),
            target_release: Status::from_target_release(
                &self.target_releases,
                releases.first(),
                &self.doc_type,
            ),
        }
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

    // Releases are a list, and each ticket can have several of them.
    // Update the counter with the values in the lists, rather than
    // with the lists themselves as values.
    for ticket in tickets.iter() {
        releases.update(ticket.target_releases.iter().map(String::as_str));
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
    tickets_with_checks: &'a [(&'a AbstractTicket, &'a Checks)],
    per_writer_stats: &'a [WriterStats<'a>],
    generated_date: &'a str,
}

pub fn analyze_status(tickets: &[AbstractTicket]) -> Result<String> {
    let products = combined_products(tickets);
    let products_display = list_or_placeholder(&products, "products");

    let releases = combined_releases(tickets);
    let releases_display = list_or_placeholder(&releases, "releases");

    let date_today = Utc::now().to_rfc2822();

    // Store checks in their own Vec and zip them with tickets by reference,
    // This satisfies ownership requirements, because the template
    // needs to receive both tickets and checks by reference.
    let checks: Vec<Checks> = tickets
        .iter()
        .map(|ticket| ticket.checks(&releases))
        .collect();
    let tickets_with_checks: Vec<(&AbstractTicket, &Checks)> =
        tickets.iter().zip(checks.iter()).collect();

    let status_table = StatusTableTemplate {
        products: &products_display,
        release: &releases_display,
        overall_progress: OverallProgress {
            ..Default::default()
        },
        per_writer_stats: &[],
        tickets_with_checks: &tickets_with_checks,
        generated_date: &date_today,
    };

    status_table
        .render()
        .wrap_err("Failed to prepare the status table.")
}
