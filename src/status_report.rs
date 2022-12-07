/*
cizrna: Generate an AsciiDoc release notes document from tracking tickets.
Copyright (C) 2022  Marek Suchánek  <msuchane@redhat.com>

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::collections::HashMap;
use std::convert::From;
use std::default::Default;
use std::ops::Neg;

use askama::Template;
use chrono::prelude::*;
use color_eyre::eyre::{Result, WrapErr};
use counter::Counter;
use regex::Regex;
use serde::Serialize;

use crate::extra_fields::DocTextStatus;
use crate::note::content_lines;
use crate::ticket_abstraction::AbstractTicket;

/// These doc types don't belong to any particular target release.
/// Skip the release check for these.
const UNCHECKED_DOC_TYPES: [&str; 3] = [
    "known issue",
    "technology preview",
    "deprecated functionality",
];
/// The maximum allowed title length for a release note.
const MAX_TITLE_LENGTH: usize = 120;

/// An overview of the completeness status across all tickets.
#[derive(Default, Serialize)]
struct OverallProgress {
    all: usize,
    complete: usize,
    complete_pct: f64,
    warnings: usize,
    warnings_pct: f64,
    incomplete: usize,
    incomplete_pct: f64,
}

impl From<&[Checks]> for OverallProgress {
    /// Calculate the global progress statistics for the whole release notes project,
    /// based on the overall status of every ticket.
    fn from(item: &[Checks]) -> Self {
        let all = item.len();
        // TODO: Currently, we calculate the overall checks twice. Once here, and once
        // for the status table. Consolidate to only calculate them once.
        let overall_checks: Vec<Status> = item.iter().map(Checks::overall).collect();
        let complete = overall_checks
            .iter()
            .filter(|status| matches!(status, Status::Ok))
            .count();
        let complete_pct = percentage(complete, all);
        let warnings = overall_checks
            .iter()
            .filter(|status| matches!(status, Status::Warning(_)))
            .count();
        let warnings_pct = percentage(warnings, all);
        let incomplete = overall_checks
            .iter()
            .filter(|status| matches!(status, Status::Error(_)))
            .count();
        let incomplete_pct = percentage(incomplete, all);

        Self {
            all,
            complete,
            complete_pct,
            warnings,
            warnings_pct,
            incomplete,
            incomplete_pct,
        }
    }
}

/// Calculate the percentage of a part in a total amount.
/// Uses `usize` as input because it works with list lengths here.
fn percentage(part: usize, total: usize) -> f64 {
    (part as f64) / (total as f64) * 100.0
}

/// Records all tickets that belong to a writer and stores statistics
/// on the overall completeness of the release notes.
#[derive(Default, Serialize)]
struct WriterStats<'a> {
    name: &'a str,
    total: i32,
    complete: i32,
    warnings: i32,
    incomplete: i32,
}

impl<'a> WriterStats<'a> {
    /// Update these writer statistics with data from a ticket and its release note.
    fn update(&mut self, checks: &Checks) {
        self.total += 1;

        // TODO: This is calculating the overall status once more. Consolidate.
        match checks.overall() {
            Status::Ok => self.complete += 1,
            Status::Warning(_) => self.warnings += 1,
            Status::Error(_) => self.incomplete += 1,
        }
    }

    // TODO: Consolidate with the `percentage` function if possible.
    /// Calculate the percentage of complete release notes assigned to this writer.
    fn percent(&self) -> f64 {
        // If no release notes are assigned to the writer, dividing by 0 would result in NaN.
        // To make the result more readable and useful, report that case as 0% complete.
        if self.total == 0 {
            0.0
        } else {
            f64::from(self.complete) / f64::from(self.total) * 100.0
        }
    }
}

/// Gather statistics on all writers involved in the project and all their release notes.
/// Returns a list of statistics per writer, sorted by the total number of release notes
/// assigned to the writer.
fn calculate_writer_stats<'a>(
    tickets_with_checks: &[(&'a AbstractTicket, &Checks)],
) -> Vec<WriterStats<'a>> {
    let mut writers_map: HashMap<&str, WriterStats> = HashMap::new();

    for (ticket, checks) in tickets_with_checks {
        let name = &ticket.docs_contact;
        writers_map
            .entry(name)
            .and_modify(|stats| stats.update(checks))
            .or_insert(WriterStats {
                name,
                ..Default::default()
            });
    }

    let mut writers: Vec<_> = writers_map.into_values().collect();

    // Sort by the number of assigned release notes in reverse, descending order,
    // so by the negative number of total release notes.
    writers.sort_by_key(|stats| stats.total.neg());

    writers
}

/// Several checks on a ticket, which capture the status of properties
/// relevant to documentation.
#[derive(Default, Serialize)]
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
        // The text status has a dedicated column in the status table.
        // Its errors might also be long. Because of that, present
        // only a brief error in the overall column instead.
        let short_text_error = Status::Error("Bad text.".into());
        let text_check = match &self.title_and_text {
            Status::Error(_) => &short_text_error,
            other => other,
        };

        // All fields on `Checks`, so that we can iterate over them.
        let items = [
            &self.doc_type,
            text_check,
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
            Status::Error(errors.join(" "))
        } else if !warnings.is_empty() {
            Status::Warning(warnings.join(" "))
        } else {
            Status::Ok
        }
    }
}

/// The status of a particular ticket property. It can be either okay,
/// a non-serious warning with a message, or a serious error with a message.
#[derive(Serialize)]
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
    /// A human-readable status message for this ticket property.
    /// If the status is a warning or an error, provide the message. If it's `Ok`, display `OK`.
    fn message(&self) -> &str {
        match self {
            Self::Ok => "OK",
            Self::Warning(message) | Self::Error(message) => message,
        }
    }

    /// An HTML color associated with a status. It's applied to text in the status table.
    fn color(&self) -> &'static str {
        match self {
            // TODO: Consider tweaking the colors to less obvious, prettier ones.
            Self::Ok => "green",
            Self::Warning(_) => "orange",
            Self::Error(_) => "red",
        }
    }

    // TODO: Consider comparing the doc text with the predefined Bugzilla doc text templates,
    // if Jira also implements them in some way.
    /// Analyze the doc text and check if it conforms to a general release note format.
    fn from_text(text: &str) -> Self {
        let content_lines = content_lines(text);

        match content_lines.len() {
            // If the doc text contains too few paragraphs, return with an error.
            0 => Self::Error("Empty RN.".into()),
            // TODO: If the project configuration auto-generates titles, release notes
            // can normally have just one paragraph. Revisit when the option is available.
            1 => Self::Error("Text in one paragraph.".into()),
            _ => {
                // If the doc text contains at least two paragraphs, it can be a release note.
                // In that case, proceed with the analysis.
                // It's now safe to index directly into the list, because it contains at least 2 items.
                // Use this to analyze the release note title in detail.
                let first_content_line = content_lines[0];
                Self::from_title(first_content_line)
            }
        }
    }

    /// Check that the first line in a release note is a title
    /// in the AsciiDoc label format, and that it matches other title requirements.
    fn from_title(text: &str) -> Self {
        // Identify the title as a line that starts with a dot (`.`) followed by a character,
        // and capture everything after the dot for analysis.
        // Also match if the line starts with spaces and then such a title,
        // because Jira inserts a space at the start of the doc text,
        // so make sure to detect that error.
        let title_regex =
            Regex::new(r"^ *\.(\S+.*)").expect("Failed to parse a regular expression.");

        let title: Option<&str> = title_regex
            .captures(text)
            .and_then(|captures| captures.get(1))
            .map(|capture| capture.as_str());

        if let Some(title) = title {
            // Measure the title length in characters, not bytes.
            let length = title.chars().count();

            // Report leading spaces.
            if text.starts_with(' ') {
                Self::Error("Title starts with a space.".into())
            // Report a long title.
            } else if length > MAX_TITLE_LENGTH {
                Self::Warning(format!("Long title: {} characters.", length))
            } else {
                Self::Ok
            }
        } else {
            Self::Error("Missing title.".into())
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
        if let Some(likely_release) = likely_release {
            // This is a replacement to the `contains` method that converts the `String` list to `&str`,
            // and thus enables us to compare the two strings without allocating every time.
            if ticket_releases.iter().any(|r| r == likely_release)
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
            DocTextStatus::InProgress => Self::Error("RN not approved.".into()),
            DocTextStatus::NoDocumentation => Self::Error("RN not needed.".into()),
        }
    }
}

impl AbstractTicket {
    /// Analyze the release note status of the ticket. Record the analysis as `Checks`.
    fn checks(&self, releases: &[&str]) -> Checks {
        Checks {
            development: Status::from_devel_status(&self.status),
            title_and_text: Status::from_text(&self.doc_text),
            doc_type: Status::from_doc_type(&self.doc_type),
            doc_status: Status::from(self.doc_text_status),
            target_release: Status::from_target_release(
                &self.target_releases,
                releases.first(),
                &self.doc_type,
            ),
        }
    }

    /// Extract the account name before `@` from the docs contact email address.
    fn docs_contact_short(&self) -> &str {
        email_prefix(&self.docs_contact)
    }

    /// Extract the account name before `@` from the assignee email address.
    fn assignee_short(&self) -> &str {
        if let Some(assignee) = &self.assignee {
            email_prefix(assignee)
        } else {
            "No assignee"
        }
    }

    /// Display the list of flags or labels for this ticket, depending on which it contains.
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

    /// Display the list of target releases, or a placeholder if there are none.
    fn display_target_releases(&self) -> String {
        if self.target_releases.is_empty() {
            "No releases".to_string()
        } else {
            self.target_releases.join(", ")
        }
    }

    /// Display the list of subsystems, or a placeholder if there are none.
    fn display_subsystems(&self) -> String {
        match &self.subsystems {
            Ok(subsystems) => {
                if subsystems.is_empty() {
                    "No subsystems".to_string()
                } else {
                    subsystems.join(", ")
                }
            }
            // If getting the subsystems field resulted in an error, it's not
            // a fatal issue in the status table. Just report it and proceed.
            Err(_) => "Invalid subsystems".to_string(),
        }
    }

    /// Display the list of components, or a placeholder if there are none.
    fn display_components(&self) -> String {
        if self.components.is_empty() {
            "No components".to_string()
        } else {
            self.components.join(", ")
        }
    }
}

/// Extract the account name before `@` from an email address.
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

/// All the data that the status table needs to render.
#[derive(Template, Serialize)] // this will generate the code...
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

/// Analyze all tickets and release notes, and produce a status table in two variants:
///
/// * As text with HTML markup.
/// * As a JSON map in text form.
pub fn analyze_status(tickets: &[AbstractTicket]) -> Result<(String, String)> {
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

    let overall_progress: OverallProgress = checks.as_slice().into();

    let writer_stats = calculate_writer_stats(&tickets_with_checks);

    let status_table = StatusTableTemplate {
        products: &products_display,
        release: &releases_display,
        overall_progress,
        per_writer_stats: &writer_stats,
        tickets_with_checks: &tickets_with_checks,
        generated_date: &date_today,
    };

    let as_html = status_table
        .render()
        .wrap_err("Failed to prepare the status table.")?;

    let as_json = serde_json::to_string(&status_table)
        .wrap_err("Failed to prepare the JSON status output.")?;

    Ok((as_html, as_json))
}
