use std::default::Default;

use askama::Template;

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

impl Status {
    fn message(&self) -> &str {
        match self {
            Self::Ok => "OK",
            Self::Warning(message) => message,
            Self::Error(message) => message,
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
        todo!()
    }

    fn docs_contact_short(&self) -> &str {
        email_prefix(&self.docs_contact)
    }

    fn assignee_short(&self) -> &str {
        if let Some(assignee) = &self.assignee {
            email_prefix(assignee)
        } else {
            "No assignee set"
        }
    }

    fn flags_or_labels(&self) -> String {
        todo!()
    }

    fn display_target_release(&self) -> &str {
        if let Some(release) = &self.target_release {
            release
        } else {
            "No release set"
        }
    }

    fn display_subsystems(&self) -> String {
        todo!()
    }

    fn display_components(&self) -> String {
        todo!()
    }
}

fn email_prefix(email: &str) -> &str {
    if let Some(prefix) = email.split('@').next() {
        prefix
    } else {
        email
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

fn main() {
    let status_table = StatusTableTemplate {
        products: "RHEL",
        release: "9.0",
        overall_progress: OverallProgress {
            ..Default::default()
        },
        per_writer_stats: &[],
        tickets: &[],
        generated_date: "",
    };
    println!("{}", status_table.render().unwrap());
}
